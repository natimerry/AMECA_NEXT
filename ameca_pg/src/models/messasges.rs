use crate::BoxResult;
use poise::serenity_prelude as serenity;
use serenity::all::GuildChannel;
use sqlx::types::chrono::{DateTime, Utc};
use sqlx::{FromRow, Pool, Postgres};
use std::future::Future;
use tracing::{debug, info};

#[derive(FromRow, Debug)]
pub struct Message {
    msg_id: i64,
    content: String,
    time: DateTime<Utc>,
    author_id: i64,
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

impl MessageData for Pool<Postgres> {
    async fn new_message(
        db: &Pool<Postgres>,
        msg: serenity::all::Message,
        channel: GuildChannel,
    ) -> BoxResult<()> {
        let msg_id = msg.id.get() as i64;
        let msg_content = msg.content;
        let msg_time = msg.timestamp.naive_utc();
        let author =  i64::from(msg.author.id);


        sqlx::query!(
            "INSERT INTO message(msg_id, content, time, author_id) VALUES ($1, $2, $3, $4)",
            msg_id,
            msg_content,
            msg_time,
            msg.author.id.get() as i64
        ).execute(db).await?;
        .execute(db)
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
