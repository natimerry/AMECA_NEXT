use crate::BoxResult;
use serenity::all::{GuildChannel, GuildId};
use sqlx::{FromRow, Pool, Postgres};
use std::future::Future;
use tracing::{debug, info};

#[derive(Debug,FromRow)]
pub struct Channel {
    pub channel_id: i64,
    pub muted: bool,
    pub logging_channel: bool,
    #[sqlx(rename = "guild_id")]
    pub parent_guild_id: i64,
}

pub trait ChannelData {
    fn new_channel(
        db: &Pool<Postgres>,
        channel: GuildChannel,
        guild: GuildId,
    ) -> impl std::future::Future<Output = BoxResult<()>> + Send;
}

impl ChannelData for Pool<Postgres> {
    async fn new_channel(db: &Pool<Postgres>, channel: GuildChannel, guild: GuildId) -> BoxResult<()>{
        debug!("Inserting channel into database");
        let channel_id = channel.id.get() as i64;
        let guild_id = guild.get() as i64;
        let _channel = sqlx::query!("INSERT INTO channel (channel_id, guild_id,logging_channel,muted) VALUES ($1, $2, $3, $4)"
            ,channel_id, guild_id, false, false).execute(db).await?;
        debug!("channel insertion result: {:?}", _channel);

        Ok(())
    }
}
