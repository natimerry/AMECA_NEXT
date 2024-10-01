use std::future::Future;
use sqlx::types::chrono;
use sqlx::types::chrono::{DateTime, Utc};
use sqlx::{ FromRow, Pool, Postgres};

use poise::serenity_prelude as serenity;
use serenity::all::GuildId;
use tracing::{debug, info};
use crate::BoxResult;

#[derive(FromRow, Debug)]
pub struct Guilds {
    pub guild_id: i64,
    pub join_date: DateTime<Utc>,
    pub members: i32,
}

pub trait GuildData {
    fn joined_guild(db: &Pool<Postgres>, members: i32, guild_id: GuildId) -> impl Future<Output =  BoxResult<()>>;
}

impl GuildData for Pool<Postgres> {
    async fn joined_guild(db: &Pool<Postgres>, members: i32, guild_id: GuildId) -> BoxResult<()>{
        info!("Registering new guild in database");
        let _guild = sqlx::query!(
            "INSERT INTO guild (guild_id,members,join_date) VALUES ($1,$2,$3::TIMESTAMPTZ) ON CONFLICT DO NOTHING",
            guild_id.get() as i64,
            members,
            Utc::now()
        ).execute(db).await.unwrap();

        debug!("guild insertion result: {:?}", _guild);
        Ok(())
    }
}
