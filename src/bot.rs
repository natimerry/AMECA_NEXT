mod automod;
mod banned_patterns;
mod purge;
mod register_bot;
mod role_for_reaction;

use crate::bot::automod::{cache_roles, log_msg_delete};
use crate::bot::banned_patterns::{ban_pattern, remove_banned_pattern};
use crate::bot::purge::purge;
use crate::bot::register_bot::{deregister_logging, register_logging_channel};
use crate::bot::role_for_reaction::{set_role_assignment, stop_watching_for_reactions};
use crate::models::channel::{Channel, ChannelData};
use crate::models::guilds::GuildData;
use crate::models::member::MemberData;
use crate::models::messasges::{DbMessage, MessageData};
use crate::models::role::Role;
use crate::{Args, BoxResult, DynError};
use dashmap::DashMap;
use poise::builtins::register_globally;
use poise::serenity_prelude as serenity;
use poise::serenity_prelude::FullEvent::Ratelimit;
use poise::serenity_prelude::{CacheHttp, GuildInfo, Reaction, RoleId, User, UserId};
use regex::Regex;
use serenity::all::{ChannelType, MessagePagination, Settings};
use sqlx::types::chrono::Utc;
use sqlx::{PgPool, Pool, Postgres};
use std::ops::Deref;
use std::thread::sleep;
use std::time::Duration;
use tokio::task::JoinHandle;
use tracing::log::debug;
use tracing::{error, info, span, trace, warn, Level};

#[derive(Clone)]
pub struct AMECA {
    pub bot: User,
    pub(crate) db: Pool<Postgres>,
    cache: bool,
    cached_regex: DashMap<i64, Vec<Regex>>,
    pub watch_msgs: DashMap<i64, Vec<Role>>, // Name and guild
}
impl AMECA {
    async fn event_handler<'a>(
        ctx: &serenity::Context,
        event: &serenity::FullEvent,
        _framework: poise::FrameworkContext<'_, AMECA, DynError>,
        data: &AMECA,
    ) -> BoxResult<()> {
        let span = span!(Level::TRACE, "AMECA", "shard" = ctx.shard_id.to_string());
        let _enter = span.enter();
        match event {
            serenity::FullEvent::GuildMemberAddition { new_member } => {
                let new_member = new_member.user.clone();
                PgPool::new_user(&data.db, new_member).await?;
            }

            serenity::FullEvent::Message { new_message } => {
                if let None = new_message.guild_id {
                    debug!(
                        "BOT DM: {} (Message is not sent in a guild!)",
                        new_message.content
                    );
                    return Ok(());
                }
                let mut to_print = String::new();
                let msg = new_message.clone();
                if &new_message.embeds.len() > &0 {
                    to_print = (&new_message)
                        .embeds
                        .iter()
                        .map(|m| format!("EMBED({:?})", m))
                        .collect::<Vec<String>>()
                        .join("\n");
                } else {
                    to_print = msg.content;
                }
                let guild_id = new_message.guild_id.unwrap().to_string();

                info!(
                    guild_id,
                    "New message: {} {} in {:?}:{:?}",
                    to_print,
                    new_message.author.name,
                    new_message.guild_id,
                    new_message.channel_id,
                );

                let channel = new_message.channel(&ctx.http).await?;
                let res =
                    DbMessage::new_message(&data.db, new_message.clone(), channel.guild().unwrap())
                        .await;

                if let Err(e) = res {
                    error!("Unable to store message in db: {}", e);
                }
                automod::on_msg(new_message.clone(), &data.db, &data, &ctx).await?;
            }
            serenity::FullEvent::Ready { .. } => {
                info!("Bot is ready to start!");
                if data.cache {
                    AMECA::cache_data(&ctx, data.clone()).await?;
                }
                info!("Bot is ready!");
            }
            serenity::FullEvent::GuildDelete { incomplete, full } => {
                info!("Bot has left the guild {} ", incomplete.id);
            }
            serenity::FullEvent::GuildCreate { guild, is_new } => {
                debug!("Bot received guild data for: {}", guild.name);

                let time = guild.joined_at.naive_utc().and_utc();
                PgPool::joined_guild(
                    &data.db,
                    guild.member_count as i32,
                    &guild.id,
                    &*guild.name,
                    time,
                )
                .await?;
            }
            serenity::FullEvent::ChannelCreate { channel } => {
                info!(
                    "New channel {} created in guild {}",
                    channel.name,
                    channel.guild_id.get() as usize
                );
                Channel::new_channel(&data.db, channel).await?;
            }
            serenity::FullEvent::ChannelDelete { channel, messages } => {
                info!(
                    "Channel {} deleted: {}...",
                    channel.name,
                    channel.guild_id.get() as usize
                );
                match messages {
                    Some(messages) => {
                        debug!("Caching messages for channel: {}", channel.name);
                        trace!("{:?}", messages);

                        for msg in messages {
                            let msg =
                                DbMessage::new_message(&data.db, msg.clone(), channel.clone())
                                    .await;

                            if let Err(e) = msg {
                                error!("Unable to store message in db: {}", e);
                            }
                        }
                    }
                    None => {
                        warn!("No messages received for deleted channel!");
                    }
                }
            }

            serenity::FullEvent::MessageDelete {
                channel_id,
                deleted_message_id,
                guild_id,
            } => {
                debug!(
                    "Message deleted in channel `{}:{:?} deleted message '{}'",
                    channel_id.name(&ctx).await?,
                    channel_id,
                    deleted_message_id.get()
                );

                let x = DbMessage::fetch_message(&data.db, deleted_message_id).await;
                let guild_id = guild_id.unwrap();

                match x {
                    Err(e) => {
                        error!("Unable to fetch message in db: {}", e);
                    }
                    Ok(Some(mut msg)) => {
                        msg.mark_deleted(&data.db).await?;
                        log_msg_delete(msg, guild_id, &ctx, data).await?;
                    }
                    Ok(None) => {
                        warn!("Deleted message unavailable in the database");
                    }
                }
            }
            Ratelimit { data } => {
                warn!(
                    "We are being ratelimited for {} seconds",
                    data.timeout.as_secs()
                );
            }
            serenity::FullEvent::ReactionAdd { add_reaction } => {
                if data.watch_msgs.is_empty() {
                    info!("Caching role reactions I have to react to!");
                    cache_roles(&data).await?;
                }
                trace!("{:#?}", add_reaction);
                let guild = add_reaction.guild_id;
                if let None = guild {
                    debug!(
                        "Reaction {} is not in an guild",
                        add_reaction.channel_id.name(&ctx).await?
                    );
                    return Ok(());
                }
                if add_reaction.message_author_id
                    == Some(UserId::new(
                        std::env::var("BOT_USER").unwrap().parse::<u64>().unwrap(),
                    ))
                {
                    return Ok(());
                }
                let guild = guild.unwrap().get() as i64;
                let guild_watchlist = data.watch_msgs.get(&guild);
                if let Some(guild_watchlist) = guild_watchlist {
                    // if it actually exists
                    let guild_watchlist = guild_watchlist.deref();
                    for role_for_reaction in guild_watchlist {
                        if add_reaction.emoji.to_string() == role_for_reaction.emoji.to_string() {
                            info!(
                                "Updating roles for {} for reacting to watched msg!",
                                &add_reaction.user_id.unwrap()
                            );
                            let x = ctx
                                .http
                                .add_member_role(
                                    add_reaction.guild_id.unwrap(),
                                    add_reaction.user_id.unwrap(),
                                    RoleId::new(role_for_reaction.roles_id as u64),
                                    Some(&format!(
                                        "Assigning role for reaction to message. (WatchID: {})",
                                        role_for_reaction.roles_id
                                    )),
                                )
                                .await;
                            match x {
                                Ok(_) => {}
                                Err(e) => {
                                    info!("Error assigning roles {:#?}", e);
                                }
                            }
                        }
                    }
                }
            }
            &_ => (),
        }
        Ok(())
    }
    pub async fn cache_guild(
        ctx: &serenity::Context,
        data: &AMECA,
        guild: GuildInfo,
    ) -> BoxResult<()> {
        let guild_members = ctx.http.get_guild_members(guild.id, None, None).await?;
        trace!("Received data {:?}", &guild_members);

        PgPool::joined_guild(
            &data.db,
            guild_members.len() as i32,
            &guild.id,
            &*guild.name,
            Utc::now(),
        )
        .await?;

        // cache channels and members next
        for member in guild_members {
            let created_user = PgPool::new_user(&data.db, member.user.clone()).await;
            match created_user {
                Ok(_) => {
                    let timestamp = member.joined_at.unwrap().naive_utc().and_utc();
                    PgPool::mark_user_in_guild(&data.db, member.user, guild.id, timestamp).await?;
                }
                Err(e) => error!("Unable to mark user in guild {}: {}", guild.id, e),
            }
        }

        let channels = ctx.http.get_channels(guild.id).await?;
        trace!("Received data {:?}", &channels);

        let channels = channels
            .iter()
            .filter(|x| x.kind == ChannelType::Text)
            .collect::<Vec<_>>();

        for channel in channels {
            info!("Storing {}", channel.name);
            Channel::new_channel(&data.db, &channel.clone()).await?;
            //iterate over messaes in channel
            debug!("Storing messsages for channel {}", channel.name);
            let channel_binding = channel.clone();
            let last_msg = channel.last_message_id;

            if let Some(last_msg) = last_msg {
                let mut msgs = ctx
                    .http
                    .get_messages(
                        channel.id,
                        Some(MessagePagination::Before(last_msg)),
                        Some(100),
                    )
                    .await.unwrap_or(vec![]); 
                let msg = ctx.http.get_message(channel_binding.id, last_msg).await;
                debug!("caching {:?}",&msg);
                if let Ok(msg) = msg{
                    msgs.push(msg);
                }
                else{
                    error!("Error in getting msg");
                }
                for msg in msgs {
                    DbMessage::new_message(&data.db, msg, channel_binding.clone()).await?;
                }
            } else {
                error!("Error in receiving last msg for channels... ");
                let msgs = ctx.http.get_messages(channel.id, None, Some(100)).await?;
                for msg in msgs {
                    DbMessage::new_message(&data.db, msg, channel_binding.clone()).await?;
                }
            }
        }
        Ok(())
    }

    pub async fn cache_data(ctx: &serenity::Context, data: AMECA) -> BoxResult<()> {
        let span = span!(Level::TRACE, "cache", "shard" = ctx.shard_id.to_string());
        let _ = span.enter();
        info!("Starting caching of data");
        let ctx = ctx.clone();
        let thread: JoinHandle<BoxResult<()>> = tokio::spawn(async move {
            loop {
                let guilds = ctx.http.get_guilds(None, None).await?;
                trace!("Received data {:?}", &guilds);
                for guild in guilds {
                    AMECA::cache_guild(&ctx, &data, guild).await?;
                }
                info!("Finished data caching");
                tokio::time::sleep(Duration::from_secs(500)).await;
            }
        });
        thread.await??;

        Ok(())
    }
    pub async fn start_shard(token: String, db: Pool<Postgres>, args: Args) -> BoxResult<()> {
        let mut settings = Settings::default();
        settings.max_messages = 0;

        let framework = poise::Framework::builder()
            .options(poise::FrameworkOptions {
                commands: vec![
                    register_logging_channel(),
                    deregister_logging(),
                    purge(),
                    ban_pattern(),
                    remove_banned_pattern(),
                    set_role_assignment(),
                    stop_watching_for_reactions(),
                ],
                event_handler: |ctx, event, framework, data| {
                    Box::pin(AMECA::event_handler(ctx, event, framework, data))
                },
                prefix_options: poise::PrefixFrameworkOptions {
                    prefix: Some("~".into()),
                    ..Default::default()
                },
                ..Default::default()
            })
            .setup(move |ctx, _ready, _framework| {
                Box::pin(async move {
                    let x: DashMap<i64, Vec<Regex>> = DashMap::new();
                    register_globally(ctx, &_framework.options().commands).await?;
                    Ok(AMECA {
                        bot: ctx
                            .http
                            .get_user(UserId::from(
                                std::env::var("BOT_USER").unwrap().parse::<u64>().unwrap(),
                            ))
                            .await?,
                        db,
                        cache: args.cache,
                        cached_regex: x,
                        watch_msgs: DashMap::new(),
                    })
                })
            })
            .build();
        let intents = serenity::GatewayIntents::AUTO_MODERATION_CONFIGURATION
            | serenity::GatewayIntents::GUILD_MESSAGES
            | serenity::GatewayIntents::GUILD_MESSAGE_REACTIONS
            | serenity::GatewayIntents::AUTO_MODERATION_EXECUTION
            | serenity::GatewayIntents::GUILDS
            | serenity::GatewayIntents::GUILD_MEMBERS
            | serenity::GatewayIntents::privileged();

        let mut client = serenity::ClientBuilder::new(token, intents)
            .framework(framework)
            .cache_settings(settings)
            .await
            .expect("Error creating client");

        let manager = client.shard_manager.clone();
        tokio::spawn(async move {
            let span = span!(Level::TRACE, "latency_check");
            let _enter = span.enter();
            loop {
                tokio::time::sleep(Duration::from_secs(3600)).await;
                let shard_runners = manager.runners.lock().await;

                for (id, runner) in shard_runners.iter() {
                    info!(
                        "Shard ID {} is {} with a latency of {:?}",
                        id, runner.stage, runner.latency,
                    );
                }
            }
        });
        client.start_shards(args.shards as u32).await?;

        Ok(())
    }
}
