use crate::db::database::Database;
use serde::{Deserialize, Serialize};

use tracing::{debug, error, warn};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Channel {
    pub muted: bool,
    pub name: String,
}

pub trait ChannelData {
    fn new_channel(
        db: &Database,
        channel: serenity::all::GuildChannel,
    ) -> impl std::future::Future<Output = ()> + Send;
}

impl ChannelData for Database {
    async fn new_channel(db: &Database, channel: serenity::all::GuildChannel) {
        let channel_id = channel.id.get();
        let created_channel: Result<Option<Channel>, surrealdb::Error> = db
            .client
            .create(("channel", channel_id))
            .content(Channel {
                muted: false,
                name: channel.name().to_string(),
            })
            .await;
        match created_channel {
            Ok(created_channel) => {
                debug!("Stored channel successfully {:?}", created_channel);
                // todo: relate message to author after user database model is created
            }
            Err(e) => {
                warn!(
                    "Unable to store message {} in database, may already exist.",
                    channel
                );
                error!("{e:?}")
            }
        }
    }
}
