use crate::BoxResult;
use log::{error, warn};
use poise::serenity_prelude as serenity;
use poise::serenity_prelude::{CacheHttp, CreateEmbed, CreateMessage, Embed, GuildId};
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
    ) -> impl std::future::Future<Output = BoxResult<()>> + Send;
    async fn send_to_logging_channel(
        embed: CreateEmbed,
        ctx: impl CacheHttp,
        db: &Pool<Postgres>,
        guild_id: GuildId,
    ) -> BoxResult<()>;

    fn get_logging_channel(
        db: &PgPool,
        guild_channel: GuildId,
    ) -> impl std::future::Future<Output = Option<Channel>> + Send;
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
    async fn send_to_logging_channel(
        embed: CreateEmbed,
        ctx: impl CacheHttp,
        db: &Pool<Postgres>,
        guild_id: GuildId,
    ) -> BoxResult<()>{
        let log_channel = Channel::get_logging_channel(&db, guild_id).await;
        if let Some(log_channel) = log_channel {
            let channel_obj = serenity::ChannelId::from(log_channel.channel_id as u64);
            let msg_builder = CreateMessage::new().embed(embed);
            channel_obj.send_message(&ctx,msg_builder).await?;
            // mark msg in db as deleted !!!
        } else {
            warn!("No logging channel found! Adding deletion to the log");
        }
        Ok(())
    }
    async fn get_logging_channel(db: &PgPool, guild_channel: GuildId) -> Option<Channel> {
        let data = sqlx::query_as::<_, Channel>(
            "SELECT * FROM channel WHERE guild_id = $1 AND logging_channel=true",
        )
        .bind(guild_channel.get() as i64)
        .fetch_optional(db)
        .await;
        if let Err(ref e) = data {
            error!("Error getting logging channel: {:?}", e);
            None
        } else {
            data.unwrap()
        }
    }
}
