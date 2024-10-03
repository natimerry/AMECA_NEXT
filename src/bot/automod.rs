use crate::bot::AMECA;
use crate::BoxResult;
use poise::serenity_prelude as serenity;
use poise::Context;
use serenity::all::Message;
use sqlx::{FromRow, PgPool};
use std::sync::atomic::{AtomicI32, AtomicUsize};
use tracing::{info, trace};
#[derive(FromRow, Debug)]
struct Banned {
    id: i32,
    name: String,
    pattern: String,
    author: i64,
}
pub async fn analyse_word(db: &PgPool, msg: String) -> BoxResult<bool> {
    let list_of_banned_words: Vec<Banned> =
        sqlx::query_as("SELECT id,name,pattern,author FROM banned_words")
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

pub async fn on_msg(msg: Message, db: &PgPool) -> BoxResult<()> {
    info!("Running automod");
    if analyse_word(db, msg.content).await? {
        info!("Word banned");
    }
    Ok(())
}
