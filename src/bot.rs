mod automod;
mod register_bot;
use crate::models::channel::ChannelData;
use crate::models::guilds::GuildData;
use crate::models::member::MemberData;
use crate::models::messasges::MessageData;
use crate::{BoxResult, DynError};
use poise::builtins::register_globally;
use poise::serenity_prelude as serenity;
use poise::serenity_prelude::GuildInfo;
use serenity::all::{ChannelType, MessagePagination, Settings};
use sqlx::{PgPool, Pool, Postgres};
use tokio::task::JoinHandle;
use tracing::log::debug;
use tracing::{error, info, trace};
use crate::bot::register_bot::register_logging_channel;

#[derive(Clone)]
pub struct AMECA {
    db: Pool<Postgres>,
    cache: bool,
}
impl AMECA {
    async fn event_handler(
        ctx: &serenity::Context,
        event: &serenity::FullEvent,
        _framework: poise::FrameworkContext<'_, AMECA, DynError>,
        data: &AMECA,
    ) -> BoxResult<()> {
        match event {
            serenity::FullEvent::Message { new_message } => {
                info!(
                    "New message: {} <{}>:{}",
                    new_message.author.name, new_message.id, new_message.content
                );
                let channel = new_message.channel(&ctx.http).await?;
                let res =
                    PgPool::new_message(&data.db, new_message.clone(), channel.guild().unwrap())
                        .await;

                if let Err(e) = res {
                    error!("Unable to store message in db: {}", e);
                }
                automod::on_msg(new_message.clone(), &data.db).await?;
            }
            serenity::FullEvent::Ready { .. } => {
                info!("Bot is ready to start!");
                if data.cache {
                    AMECA::cache_data(&ctx, data.clone()).await?;
                }
                info!("Bot is ready!");
            }
            serenity::FullEvent::GuildCreate { guild, is_new } => {
                // bot has been added to a new guild... just generate new guild id for now
                // TODO: maybe cache?
                info!("Bot has joined new guild!");
                PgPool::joined_guild(&data.db, guild.member_count as i32, &guild.id, &*guild.name).await?;
            },
            &_ => ()
        }
        Ok(())
    }
    pub async fn cache_guild(ctx: &serenity::Context, data: &AMECA, guild: GuildInfo) -> BoxResult<()> {
        let guild_members = ctx.http.get_guild_members(guild.id, None, None).await?;
        trace!("Received data {:?}", &guild_members);
        PgPool::joined_guild(&data.db, guild_members.len() as i32, &guild.id, &*guild.name).await?;

        // cache channels and members next
        for member in guild_members {
            let created_user =
                PgPool::new_user(&data.db, member.user.clone()).await;
            match created_user {
                Ok(_) => {
                    let timestamp = member.joined_at.unwrap().naive_utc().and_utc();
                    PgPool::mark_user_in_guild(
                        &data.db,
                        member.user,
                        guild.id,
                        timestamp,
                    )
                        .await?;
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
            PgPool::new_channel(&data.db, channel.clone()).await?;
            //iterate over messaes in channel
            debug!("Storing messsages for channel {}", channel.name);
            let channel_binding = channel.clone();
            let last_msg = channel.last_message_id;

            if let Some(last_msg) = last_msg {
                let msgs = ctx
                    .http
                    .get_messages(
                        channel.id,
                        Some(MessagePagination::Before(last_msg)),
                        Some(100),
                    )
                    .await?;
                for msg in msgs {
                    PgPool::new_message(&data.db, msg, channel_binding.clone())
                        .await?;
                }
            } else {
                error!("Error in receiving last msg for channels... ");
                let msgs = ctx.http.get_messages(channel.id, None, Some(100)).await?;
                for msg in msgs {
                    PgPool::new_message(&data.db, msg, channel_binding.clone())
                        .await?;
                }
            }
        }
        Ok(())
    }

    pub async fn cache_data(ctx: &serenity::Context, data: AMECA) -> BoxResult<()> {
        info!("Starting caching of data");
        let ctx = ctx.clone();
        let data_binding = data.clone();
        let thread: JoinHandle<BoxResult<()>> = tokio::spawn(async move {
            let guilds = ctx.http.get_guilds(None, None).await?;
            trace!("Received data {:?}", &guilds);
            for guild in guilds {
                AMECA::cache_guild(&ctx, &data, guild).await?;
            }
            Ok(())
        });
        thread.await??;

        Ok(())
    }
    pub async fn start_shard(token: String, db: Pool<Postgres>, cache: bool) -> BoxResult<()> {
        let mut settings = Settings::default();
        settings.max_messages = 10000;

        let framework = poise::Framework::builder()
            .options(poise::FrameworkOptions {
                commands: vec![
                    register_logging_channel()
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
                    register_globally(ctx, &_framework.options().commands).await?;
                    Ok(AMECA { db, cache })
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

        let client = serenity::ClientBuilder::new(token, intents)
            .framework(framework)
            .cache_settings(settings)
            .await;

        client.unwrap().start().await?;
        Ok(())
    }
}
