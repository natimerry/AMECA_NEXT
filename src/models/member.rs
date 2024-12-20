use crate::BoxResult;
use poise::serenity_prelude as serenity;
use poise::serenity_prelude::GuildId;
use serenity::all::User;
use sqlx::types::chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use std::future::Future;
use tracing::debug;

#[derive(Debug, FromRow)]
pub struct Members {
    pub member_id: i64,
    pub name: String, // real name
}

pub trait MemberData {
    fn new_user(db: &PgPool, user: User) -> impl Future<Output = BoxResult<()>> + Send;

    fn mark_user_in_guild(
        db: &PgPool,
        user: User,
        guild: GuildId,
        time: DateTime<Utc>,
    ) -> impl Future<Output = BoxResult<()>> + Send;
    fn get_user_join_time(
        db: &PgPool,
        user: User,
        guild: GuildId,
    ) -> impl Future<Output = BoxResult<DateTime<Utc>>>;
}

impl MemberData for Members {
    async fn mark_user_in_guild(
        db: &PgPool,
        user: User,
        guild: GuildId,
        time: DateTime<Utc>,
    ) -> BoxResult<()> {
        let user_id = user.id.get() as i64;
        let guild_id = guild.get() as i64;

        debug!(
            "Setting guild member relation for {}->{}",
            user_id, guild_id
        );
        let _ = sqlx::query!("INSERT INTO guild_join_member(guild_id, member_id, time) VALUES ($1,$2,$3::timestamptz) ON CONFLICT DO NOTHING",
            guild_id,
            user_id,
            time).execute(db).await?;

        Ok(())
    }

    async fn get_user_join_time(
        db: &PgPool,
        user: User,
        guild: GuildId,
    ) -> BoxResult<DateTime<Utc>> {
        #[derive(FromRow)]
        struct Relation {
            time: DateTime<Utc>,
        }
        let user_id = user.id.get() as i64;
        let guild_id = guild.get() as i64;

        debug!(
            "Fetching guild member relation for {}->{}",
            user_id, guild_id
        );

        let time: Relation = sqlx::query_as(
            "SELECT time FROM guild_join_member WHERE guild_id = $1 AND member_id = $2",
        )
        .bind(guild_id)
        .bind(user_id)
        .fetch_one(db)
        .await?;
        Ok(time.time)
    }

    async fn new_user(db: &PgPool, user: User) -> BoxResult<()> {
        let user_id = user.id.get() as i64;
        let name = &user.name;
        debug!("Inserting new user {:?} into database", &user);
        let _user = sqlx::query!(
            "INSERT INTO member(member_id,name) VALUES($1,$2) ON CONFLICT DO NOTHING;",
            user_id,
            name,
        )
        .execute(db)
        .await?;
        Ok(())
    }
}
