use crate::models::channel::{Channel, ChannelData};
use crate::models::member::{MemberData, Members};
use crate::BoxResult;
use poise::serenity_prelude as serenity;
use serenity::all::{ChannelId, GuildChannel, GuildId, Member, User};
use sqlx::types::chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool, Pool, Postgres};
use std::future::Future;
use tracing::{debug, info, warn};

#[derive(FromRow, Debug)]
pub struct Message {
    msg_id: i64,
    content: String,
    time: DateTime<Utc>,
    author_id: i64,
    channel_id: i64,
}

pub trait MessageData {
    fn new_message(
        db: &Pool<Postgres>,
        msg: serenity::Message,
        channel: GuildChannel,
    ) -> impl std::future::Future<Output = BoxResult<()>> + Send;

    fn fetch_messages(
        db: &Pool<Postgres>,
        channel: GuildChannel,
    ) -> impl std::future::Future<Output = BoxResult<Option<Message>>> + Send;
}

impl Message {
    async fn check_violations(db: &PgPool, author: User, channel: GuildChannel) -> BoxResult<()> {
        // if a message author doesnt exist in the database create one
        let db_author = sqlx::query_as::<_, Members>(
            "SELECT member_id,name,admin,warnings_issued FROM member WHERE member_id = $1",
        )
        .bind(author.id.get() as i64)
        .fetch_optional(db)
        .await?;

        if let None = db_author {
            warn!("Message author is not cached!");
            PgPool::new_user(&db, author).await?;
        }

        struct dummychannel {
            channel_id : i64,
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
            PgPool::new_channel(&db,channel).await?;
        }

        Ok(())
    }
}
impl MessageData for Pool<Postgres> {
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

        Message::check_violations(db, msg.author, channel).await?;
        let _msg = sqlx::query!(
            "INSERT INTO message(msg_id, content, time, author_id,channel_id) VALUES ($1, $2, $3, $4,$5)",
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
    async fn fetch_messages(
        db: &Pool<Postgres>,
        channel: GuildChannel,
    ) -> BoxResult<Option<Message>> {
        unimplemented!()
    }
}
