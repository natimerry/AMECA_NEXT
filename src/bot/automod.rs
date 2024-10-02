use std::sync::atomic::{AtomicI32, AtomicUsize};
use clap::builder::Str;
use log::debug;
use tracing::{info, trace};
use poise::Context;
use sqlx::{FromRow, PgPool};
use crate::BoxResult;
use regex::Regex;
use serenity::all::Message;
use crate::bot::AMECA;
#[derive(FromRow,Debug)]
struct Banned {
    id: i32,
    name: String,
    pattern: String,
    author: i64,
}
pub async fn analyse_word(db: &PgPool, msg: String) -> BoxResult<bool> {
    let list_of_banned_words :Vec<Banned>=
        sqlx::query_as("SELECT id,name,pattern,author FROM banned_words").fetch_all(db).await?;
    trace!("{:?}",list_of_banned_words);
    for banned_word in list_of_banned_words {
        let regex = format!(r"{}",banned_word.pattern);
        let re = Regex::new(&regex).unwrap();
        let x = re.captures(msg.as_str());
        trace!("{:?}",x);
        if let Some(x) = x {
            return Ok(true);
        }
        return Ok(false);
    }
    Ok(false)
}

pub async fn on_msg(msg: Message,db: &PgPool) -> BoxResult<()>{
    info!("Running automod");
    if analyse_word(db,msg.content).await?{
        info!("Word banned");
    }
    Ok(())
}