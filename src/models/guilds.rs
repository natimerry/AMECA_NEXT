use sqlx::types::chrono;
use sqlx::types::chrono::{DateTime, Utc};
use sqlx::{FromRow, Pool, Postgres};
use std::future::Future;

use crate::BoxResult;
use poise::serenity_prelude as serenity;
use poise::serenity_prelude::GuildInfo;
use serenity::all::GuildId;
use tracing::{debug, info};

#[derive(FromRow, Debug)]
pub struct Guilds {
    pub guild_id: i64,
    pub join_date: DateTime<Utc>,
    pub members: i32,
}

pub trait GuildData {
    fn joined_guild(
        db: &Pool<Postgres>,
        members: i32,
        guild_id: &GuildId,
        guild_name: &str
    ) -> impl Future<Output = BoxResult<()>>;
}

impl GuildData for Pool<Postgres> {
    async fn joined_guild(
        db: &Pool<Postgres>,
        members: i32,
        guild_id: &GuildId,
        guild_name: &str
    ) -> BoxResult<()> {
        info!("Registering new guild in database");
        let guildid = guild_id.get() as i64;
        let _guild = sqlx::query_file!(
            "sql/insert_new_guild.sql",
            guildid,
            members,
            Utc::now(),
            guild_name
        )
        .execute(db)
        .await
        .unwrap();

        debug!("guild insertion result: {:?}", _guild);
        Ok(())
    }
}
