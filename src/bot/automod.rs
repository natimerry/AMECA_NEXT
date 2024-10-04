use crate::bot::AMECA;
use crate::models::messasges::DbMessage;
use crate::BoxResult;
use log::debug;
use poise::futures_util::SinkExt;
use poise::serenity_prelude as serenity;
use poise::serenity_prelude::{
    ChannelId, Color, Context, CreateEmbed, CreateEmbedFooter, CreateMessage, GuildChannel,
};
use regex::Regex;
use serenity::all::Message;
use sqlx::types::chrono::{DateTime, Local};
use sqlx::{FromRow, PgPool, Pool, Postgres};
use tracing::{info, trace};

#[derive(FromRow, Debug)]
struct Banned {
    id: i32,
    name: String,
    pattern: String,
    author: i64,
}

// TODO: STORE BANNED PATTERNS IN DB ON UPDATE / DELETION
pub async fn analyse_word(db: &PgPool, msg: Message, data: &AMECA) -> BoxResult<bool> {
    if data.cached_regex.is_empty() {
        let list_of_banned_patterns: Vec<Banned> = sqlx::query_as(
            "SELECT id,name,pattern,author FROM prohibited_words_for_guild WHERE guild_id = $1",
        )
        .bind(msg.guild_id.unwrap().get() as i64)
        .fetch_all(db)
        .await?;

        trace!("{:?}", list_of_banned_patterns);
        for banned_word in list_of_banned_patterns {
            let re =
                Regex::new(&format!(r"{}", banned_word.pattern)).expect("Unable to compile regex");
            data.cached_regex.insert(banned_word.id, re);
        }
    }

    for (_, re) in data.cached_regex.clone() {
        if re.is_match(msg.content.as_str()) {
            return Ok(true);
        }
    }
    Ok(false)
}

pub async fn log_msg_delete(
    msg: DbMessage,
    guild_channel: ChannelId,
    ctx: &Context,
) -> BoxResult<()> {
    let embed = CreateEmbed::new()
        .title("Deleted Message")
        .description(format!(
            "Content: {}\nTime:{}\nChannel:<#{}>\nAuthor:<@{}>",
            msg.content,
            msg.time.to_string(),
            msg.channel_id,
            msg.author_id
        ))
        .color(Color::from_rgb(255, 0, 0))
        .footer(CreateEmbedFooter::new(
            "https://github.com/natimerry/ameca_next",
        ));
    let msg = CreateMessage::new().embed(embed);
    trace!("{:?}", msg);

    let _ = guild_channel.send_message(&ctx.http, msg).await?;
    Ok(())
}
pub async fn on_msg(msg: Message, db: &PgPool, data: &AMECA) -> BoxResult<()> {
    if analyse_word(db, msg.clone(), data).await? {
        info!(
            "Removing banned word in sentence {} by {}: {:?}",
            msg.content, msg.author.name, msg.guild_id
        );
    }
    Ok(())
}
