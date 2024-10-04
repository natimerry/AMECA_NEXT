use std::future::Future;
use log::error;
use crate::BoxResult;
use poise::serenity_prelude as serenity;
use serenity::all::GuildChannel;
use sqlx::{FromRow, PgPool, Pool, Postgres};
use tracing::{debug, trace};

#[derive(Debug, FromRow)]
pub struct Channel {
    pub channel_id: i64,
    pub muted: bool,
    pub logging_channel: bool,
    pub channel_name: String,
    pub automod_exempt: bool,
    pub guild_id: Option<i64>,
}

pub trait ChannelData {
    fn new_channel(
        db: &Pool<Postgres>,
        channel: &GuildChannel,
    ) -> impl std::future::Future<Output=BoxResult<()>> + Send;

    fn get_logging_channel(db: &PgPool) -> impl std::future::Future<Output=Option<Channel>> + Send;
}

impl ChannelData for Channel {
    async fn new_channel(db: &Pool<Postgres>, channel: &GuildChannel) -> BoxResult<()> {
        debug!("Inserting new channel into database");
        let channel_id = channel.id.get() as i64;
        let guild_id: Option<i64> = Some(i64::from(channel.guild_id));
        // TODO: create guild before channel to ensure healthy relationship
        let _channel = sqlx::query!(
            "INSERT INTO channel (channel_id, guild_id,logging_channel,muted,channel_name) VALUES ($1, $2, $3, $4,$5) ON CONFLICT DO NOTHING"
            ,channel_id, guild_id, false, false,channel.name).execute(db).await?;
        trace!("channel insertion result: {:?}", _channel);

        Ok(())
    }

    async fn get_logging_channel(db: &PgPool) -> Option<Channel> {
        let data = sqlx::query_as::<_, Channel>("SELECT * FROM channels WHERE channel_id = $1 AND guild_id = $2").fetch_optional(db).await;
        return if let Err(ref e) = data {
            error!("Error getting logging channel: {:?}", e);
            None
        }
        else{
            data.unwrap()
        }
    }
}
