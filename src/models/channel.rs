use crate::BoxResult;
use poise::serenity_prelude as serenity;
use serenity::all::GuildChannel;
use sqlx::{FromRow, Pool, Postgres};
use tracing::debug;

#[derive(Debug, FromRow)]
pub struct Channel {
    pub channel_id: i64,
    pub muted: bool,
    pub logging_channel: bool,
    pub channel_name: String,
    #[sqlx(rename = "guild_id")]
    pub parent_guild_id: Option<i64>,
}

pub trait ChannelData {
    fn new_channel(
        db: &Pool<Postgres>,
        channel: GuildChannel,
    ) -> impl std::future::Future<Output = BoxResult<()>> + Send;
}

impl ChannelData for Pool<Postgres> {
    async fn new_channel(db: &Pool<Postgres>, channel: GuildChannel) -> BoxResult<()> {
        debug!("Inserting channel into database");
        let channel_id = channel.id.get() as i64;
        let guild_id: Option<i64> = Some(i64::from(channel.guild_id));
        // TODO: create guild before channel to ensure healthy relationship
        let _channel = sqlx::query!(
            "INSERT INTO channel (channel_id, guild_id,logging_channel,muted,channel_name) VALUES ($1, $2, $3, $4,$5) ON CONFLICT DO NOTHING"
            ,channel_id, guild_id, false, false,channel.name).execute(db).await?;
        debug!("channel insertion result: {:?}", _channel);

        Ok(())
    }
}
