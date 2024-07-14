use chrono::{DateTime, Utc};
use clap::builder::Str;
use log::debug;
use serde::{Deserialize, Serialize};
use serenity::all::{GuildId, Timestamp};
use tracing::{error, warn};
use crate::db::database::Database;
use crate::models::guilds::Guild;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Message {
    pub time: String,
    pub content: String,
}

pub trait MessageData {
    async fn new_message(db: &Database,msg: serenity::all::Message);
}

impl MessageData for Database{
    async fn new_message(db: &Database, msg: serenity::all::Message) {
        let msg_id = msg.id.get();
        let created_msg: Result<Option<Message>, surrealdb::Error> = db
            .client
            .create(("message", msg_id))
            .content(Message {
                content: msg.content,
                time: msg.timestamp.to_string(),
            })
            .await;
        match created_msg{
            Ok(msg) => {
                debug!("Stored msg successfully {:?}",msg.unwrap());
                // todo: relate message to author after user database model is created
            },
            Err(e) => {
                warn!("Unable to store message {} in database, may already exist.",msg.id.get());
                error!("{e:?}")
            }
        }
    }
}