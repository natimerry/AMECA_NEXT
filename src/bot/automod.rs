use log::debug;
use poise::futures_util::SinkExt;
use crate::models::messasges::DbMessage;
use crate::BoxResult;
use poise::serenity_prelude as serenity;
use poise::serenity_prelude::{ChannelId, Color, Context, CreateEmbed, CreateEmbedFooter, CreateMessage, GuildChannel};
use serenity::all::Message;
use sqlx::types::chrono::{DateTime, Local};
use sqlx::{FromRow, PgPool, Pool, Postgres};
use tracing::{info, trace};
use crate::bot::AMECA;

#[derive(FromRow, Debug)]
struct Banned {
    id: i32,
    name: String,
    pattern: String,
    author: i64,
}
pub async fn analyse_word(db: &PgPool, msg: String,data: &AMECA) -> BoxResult<bool> {
    let list_of_banned_words: Vec<Banned> =
        sqlx::query_as("SELECT id,name,pattern,author FROM prohibited_words_for_guild")
            .fetch_all(db)
            .await?;
    trace!("{:?}", list_of_banned_words);
    for banned_word in list_of_banned_words {
        if msg.as_str().eq(&banned_word.name) {
            return Ok(true);
        }
    }
    Ok(false)
}

pub async fn log_msg_delete(msg: DbMessage, guild_channel: ChannelId,ctx: &Context) -> BoxResult<()>{
    let embed = CreateEmbed::new()
        .title(format!("Message by <@{}>", msg.author_id))
        .description(format!(
            "Content: {}\nTime:{}\nChannel:<#{}>",
            msg.content,
            msg.time.to_string(),
            msg.channel_id
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
pub async fn on_msg(msg: Message, db: &PgPool,data: &AMECA) -> BoxResult<()> {
    if analyse_word(db, msg.content, data).await? {
        debug!("Word banned");
    }
    Ok(())
}
