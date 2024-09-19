use crate::db::database::Database;
use log::debug;
use serde::{Deserialize, Serialize};
use serenity::all::GuildChannel;
use tracing::{error, warn};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Message {
    pub message_id: i64,
    pub time: String,
    pub content: String,
}

pub trait MessageData {
    fn new_message(
        db: &Database,
        msg: serenity::all::Message,
        channel: GuildChannel,
    ) -> impl std::future::Future<Output = ()> + Send;
    fn fetch_msg(
        db: &Database,
        msg_id: u64,
    ) -> impl std::future::Future<Output = Option<Message>> + Send;
}

impl MessageData for Database {
    async fn new_message(db: &Database, msg: serenity::all::Message, channel: GuildChannel) {
        let msg_id = msg.id.get();
        let created_msg: Result<Option<Message>, surrealdb::Error> = db
            .client
            .create(("message", msg_id as i64))
            .content((Message {
                content: msg.content,
                message_id: msg_id as i64,
                time: msg.timestamp.to_string(),
            }))
            .await; 

        // TODO: set relation
        let query = format!(
            "RELATE channel:{}->sent_in_channel->message:{};",
            channel.id.get(),
            msg_id
        );

        let query_member = format!(
            "RELATE members:{}->sent->message:{};",
            msg.author.id.get(),
            msg_id
        );

        let _ = db
            .db_query(query)
            .await
            .expect("Unable to set message relation");

        let _ = db
            .db_query(query_member)
            .await
            .expect("Unable to set message relation");

        match created_msg {
            Ok(msg) => {
                debug!("Stored msg successfully {:?}", msg.unwrap());
                // todo: relate message to author after user database model is created
            }
            Err(e) => {
                warn!(
                    "Unable to store message {} in database, may already exist.",
                    msg.id.get()
                );
                error!("{e:?}")
            }
        }
    }

    async fn fetch_msg(db: &Database, msg_id: u64) -> Option<Message> {
        let query = format!("(SELECT * from message:{msg_id})[0]");
        return if let Ok(mut resp) = db.db_query(query).await {
            let msg: Option<Message> = resp.take(0).expect("SHIT");
            msg
        } else {
            None
        };
    }
}
