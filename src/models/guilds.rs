use sqlx::types::chrono::{DateTime, Utc};
use sqlx::{FromRow, Pool, Postgres};
use std::future::Future;

use crate::BoxResult;
use poise::serenity_prelude as serenity;
use poise::serenity_prelude::Member;
use serenity::all::GuildId;
use tracing::{debug, info, trace};
use crate::models::channel::Channel;


pub trait GuildData {
    fn joined_guild(
        db: &Pool<Postgres>,
        members: i32,
        guild_id: &GuildId,
        guild_name: &str,
        join_time: DateTime<Utc>,
    ) -> impl Future<Output=BoxResult<()>>;
}

impl GuildData for Pool<Postgres> {
    async fn joined_guild(
        db: &Pool<Postgres>,
        members: i32,
        guild_id: &GuildId,
        guild_name: &str,
        join_time: DateTime<Utc>,
    ) -> BoxResult<()> {
        info!("Registering new guild in database");
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

        trace!("guild insertion result: {:?}", _guild);
        Ok(())
    }
}

mod tests {
    use std::num::NonZeroU64;
    use poise::serenity_prelude::GuildId;
    use sqlx::{PgPool, Row};
    use sqlx::types::chrono::Utc;
    use tracing::debug;
    use crate::BoxResult;
    use crate::models::guilds::{GuildData};

    #[sqlx::test]
    async fn insert_guild(pool: PgPool) -> BoxResult<()> {
        let mut conn = pool.acquire().await?;
        let _x = PgPool::joined_guild(&pool, 132, &GuildId::new(1278906090913923082),"test_server").await?;

        let foo = sqlx::query("SELECT * FROM guild WHERE guild_id = $1::BIGINT").bind(1278906090913923082i64)
            .fetch_one(&mut *conn)
            .await?;

        dbg!(&foo);
        Ok(())
    }
}