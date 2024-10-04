use crate::models::channel::ChannelData;
use crate::models::member::{MemberData, Members};
use crate::{models, BoxResult};
use poise::serenity_prelude as serenity;
use serenity::all::{GuildChannel, User};
use sqlx::types::chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool, Pool, Postgres};
use std::fmt::Write;
use poise::serenity_prelude::MessageId;
use tracing::{debug, warn};

#[derive(FromRow, Clone, Debug, Default)]
pub struct DbMessage {
    pub msg_id: i64,
    pub content: String,
    pub time: DateTime<Utc>,
    pub author_id: i64,
    pub channel_id: i64,
}

impl From<serenity::Message> for DbMessage {
    fn from(msg: serenity::Message) -> Self {
        Self {
            msg_id: msg.id.get() as i64,
            content: msg.content,
            time: *msg.timestamp,
            author_id: msg.author.id.get() as i64,
            channel_id: msg.channel_id.get() as i64,
        }
    }
}

pub trait MessageData {
    fn new_message(
        db: &Pool<Postgres>,
        msg: serenity::Message,
        channel: GuildChannel,
    ) -> impl std::future::Future<Output = BoxResult<()>> + Send;

    fn fetch_message(
        db: &Pool<Postgres>,
        channel: &MessageId,
    ) -> impl std::future::Future<Output = BoxResult<Option<DbMessage>>> + Send;
}

impl DbMessage {
    async fn check_violations(db: &PgPool, author: User, channel: GuildChannel) -> BoxResult<()> {
        // if a message author doesnt exist in the database create one
        let db_author = sqlx::query_as::<_, Members>(
            "SELECT member_id,name,warnings_issued FROM member WHERE member_id = $1",
        )
        .bind(author.id.get() as i64)
        .fetch_optional(db)
        .await?;

        if let None = db_author {
            warn!("Message author is not cached!");
            PgPool::new_user(&db, author).await?;
        }

        struct dummychannel {
            channel_id: i64,
        }
        let db_channel = sqlx::query_as!(
            dummychannel,
            "SELECT channel_id FROM channel WHERE channel_id = $1",
            channel.id.get() as i64
        )
        .fetch_optional(db)
        .await?;

        if let None = db_channel {
            warn!("Message channel is not cached!");
            models::channel::Channel::new_channel(&db, &channel).await?;
        }

        Ok(())
    }
}
impl MessageData for DbMessage {
    async fn new_message(
        db: &Pool<Postgres>,
        msg: serenity::all::Message,
        channel: GuildChannel,
    ) -> BoxResult<()> {
        let msg_id = msg.id.get() as i64;
        let msg_content = msg.content;
        let msg_time = msg.timestamp.naive_utc().and_utc();
        let author = i64::from(msg.author.id);
        let guild = channel.guild_id;

        DbMessage::check_violations(db, msg.author, channel).await?;
        let _msg = sqlx::query!(
            "INSERT INTO message(msg_id, content, time, author_id,channel_id) VALUES ($1, $2, $3, $4,$5) ON CONFLICT DO NOTHING;",
            msg_id,
            msg_content,
            msg_time,
            author,
            msg.channel_id.get() as i64,
        )
            .execute(db)
            .await?;
        debug!("Created new message {}", msg_id);
        debug!("Message insertion result {:?}", _msg);
        Ok(())
    }
    async fn fetch_message(db: &Pool<Postgres>, msg_id: &MessageId) -> BoxResult<Option<DbMessage>> {
        Ok(
            (sqlx::query_as::<_, DbMessage>("SELECT * FROM message WHERE msg_id = $1;")
                .bind(msg_id.get()as i64)
                .fetch_optional(&*db)
                .await?),
        )
    }
}
