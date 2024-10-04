use sqlx::types::chrono::{DateTime, Utc};
use sqlx::{Pool, Postgres};
use std::future::Future;

use crate::BoxResult;
use poise::serenity_prelude as serenity;
use serenity::all::GuildId;

pub trait GuildData {
    fn joined_guild(
        db: &Pool<Postgres>,
        members: i32,
        guild_id: &GuildId,
        guild_name: &str,
        join_time: DateTime<Utc>,
    ) -> impl Future<Output = BoxResult<()>>;
}

impl GuildData for Pool<Postgres> {
    async fn joined_guild(
        db: &Pool<Postgres>,
        members: i32,
        guild_id: &GuildId,
        guild_name: &str,
        join_time: DateTime<Utc>,
    ) -> BoxResult<()> {
        let guildid = guild_id.get() as i64;
        let _guild = sqlx::query_file!(
            "sql/insert_new_guild.sql",
            guildid,
            members,
            join_time,
            guild_name
        )
        .execute(db)
        .await
        .unwrap();

        Ok(())
    }
}

mod tests {
    
    
    
    
    
    
    


}
