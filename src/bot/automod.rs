use crate::bot::AMECA;
use crate::models::channel::ChannelData;
use crate::models::messasges::{DbMessage, MessageData};
use crate::models::role::Role;
use crate::BoxResult;
use poise::serenity_prelude::{
    self as serenity, ChannelId, MessageId, 
};
use poise::serenity_prelude::{Color, Context, CreateEmbed, CreateEmbedFooter, GuildId};
use regex::Regex;
use serenity::all::Message;
use sqlx::{FromRow, PgPool};
use std::ops::Deref;
use tracing::{debug, error, info, span, trace, warn, Level};

#[derive(FromRow, Debug)]
struct Banned {
    id: i32,
    name: String,
    pattern: String,
    author: i64,
    guild_id: i64,
}

pub async fn on_msg_delete(
    ctx: &Context,
    data: &AMECA,
    channel_id: &ChannelId,
    deleted_message_id: &MessageId,
    guild_id: &Option<GuildId>,
) -> BoxResult<()> {
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
            let channel_id = channel_id.get();
            let guild_id = guild_id.get();
            error!(channel_id, guild_id, "Unable to fetch message in db: {}", e);
        }
        Ok(Some(mut msg)) => {
            msg.mark_deleted(&data.db).await?;
            log_msg_delete(msg, guild_id, ctx, data).await?;
        }
        Ok(None) => {
            let channel_id = channel_id.get();
            let guild_id = guild_id.get();
            let msg_id = deleted_message_id.get();
            warn!(
                channel_id,
                guild_id, msg_id, "Deleted message unavailable in the database"
            );
        }
    }
    Ok(())
}

pub async fn on_new_msg(ctx: &Context, data: &AMECA, new_message: &Message) -> BoxResult<()> {
    if new_message.guild_id.is_none() {
        debug!(
            "BOT DM: {} (Message is not sent in a guild!)",
            new_message.content
        );
        return Ok(());
    }

    if crate::utils::check_if_author_is_bot(new_message) {
        return Ok(());
    }
    // log the message to db
    #[allow(unused_assignments)]
    let mut to_print = String::new();
    let msg = new_message.clone();
    if !new_message.embeds.is_empty() {
        to_print = (new_message)
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
    let res = DbMessage::new_message(
        &data.db,
        new_message.clone(),
        channel.clone().guild().unwrap(),
    )
        .await;

    if let Err(e) = res {
        let content = to_print;
        let msg_author = &new_message.author.name;
        let guild = new_message.guild_id.unwrap().get();
        let channel = new_message.channel_id.get();
        error!(
            content,
            msg_author, guild, channel, "Unable to store message in db: {}", e
        );
    }
    // run automod through processed message!
    analyse_msg(new_message.clone(), &data.db, &data, &ctx).await?;


    // check if one of mentioned user was afk

    Ok(())
}

pub async fn cache_roles(data: &AMECA) -> BoxResult<()> {
    data.watch_msgs.clear();
    let list_of_roles: Vec<Role> = sqlx::query_as("SELECT * from reaction_role")
        .fetch_all(&data.db)
        .await?;
    for role in &list_of_roles {
        let guild = role.guild_id;
        debug!("Caching role to map {role:#?}");
        data.watch_msgs
            .entry(guild)
            .and_modify(|list| list.push(role.clone()))
            .or_insert(vec![role.clone()]);
    }
    trace!("{:?}", list_of_roles);

    Ok(())
}

pub async fn cache_regex(db: &PgPool, data: &AMECA) -> BoxResult<()> {
    data.cached_regex.clear();
    let list_of_banned_patterns: Vec<Banned> =
        sqlx::query_as("SELECT id,name,pattern,author,guild_id FROM prohibited_words_for_guild")
            .fetch_all(db)
            .await?;

    trace!("{:?}", list_of_banned_patterns);
    for banned_word in list_of_banned_patterns {
        let re = Regex::new(&banned_word.pattern.to_string()).expect("Unable to compile regex");
        debug!("Caching regex to map {re:#?}");
        data.cached_regex
            .entry(banned_word.guild_id)
            .and_modify(|list| list.push(re.clone()))
            .or_insert(vec![re.clone()]);
    }
    Ok(())
}
// TODO: STORE BANNED PATTERNS IN DB ON UPDATE / DELETION
pub async fn analyse_word(db: &PgPool, msg: Message, data: &AMECA) -> BoxResult<bool> {
    debug!("Map state: {:#?}", &data.cached_regex);
    if data.cached_regex.is_empty() {
        trace!("{:?}", &msg);
        cache_regex(db, data).await?;
    }
    let guild_id = msg.guild_id.unwrap_or(GuildId::from(1231232131231));
    let id = guild_id.get() as i64;
    let vec = data.cached_regex.get(&id);
    match vec {
        Some(vec) => {
            let vec = vec.deref().clone();
            let mut flag = false;
            for x in vec {
                if (x).is_match(&msg.content) {
                    flag = true;
                    break;
                }
            }
            Ok(flag)
        }
        None => {
            debug!("No regex rule for guild");
            Ok(false)
        }
    }
}

pub async fn log_msg_delete(
    msg: DbMessage,
    guild_id: GuildId,
    ctx: &Context,
    data: &AMECA,
) -> BoxResult<()> {
    let embed = CreateEmbed::new()
        .title("Deleted Message")
        .description(format!(
            "Content: {}\nTime:{}\nChannel:<#{}>\nAuthor:<@{}>",
            msg.content, msg.time, msg.channel_id, msg.author_id
        ))
        .color(Color::from_rgb(255, 0, 0))
        .footer(CreateEmbedFooter::new(
            "https://github.com/natimerry/ameca_next",
        ));
    // let msg = CreateMessage::new().embed(embed);
    crate::models::channel::Channel::send_to_logging_channel(embed, &ctx, &data.db, guild_id)
        .await?;
    trace!("{:?}", msg);

    Ok(())
}
async fn analyse_msg(msg: Message, db: &PgPool, data: &AMECA, ctx: &Context) -> BoxResult<()> {
    let span = span!(Level::TRACE, "AUTOMOD", "shard" = ctx.shard_id.to_string());
    let _ = span.enter();
    if msg.author.id.get() == std::env::var("BOT_USER").unwrap().parse::<u64>().unwrap() {
        return Ok(());
    }
    if analyse_word(db, msg.clone(), data).await? {
        info!(
            "Removing banned word in sentence {} by {}: {:?}",
            msg.content, msg.author.name, msg.guild_id
        );
        msg.delete(&ctx.http).await?;
        msg.channel_id
            .say(&ctx.http, "Message removed because of violation!")
            .await?;
    }
    Ok(())
}
