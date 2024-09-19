use crate::db::database::Database;
use log::info;
use serde::{Deserialize, Serialize};

use serenity::{all::GuildId};
use tracing::{debug, error, warn};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Channel {
    pub channel_id: i64,
    pub muted: bool,
    pub name: String,
}

pub trait ChannelData {
    fn new_channel(
        db: &Database,
        channel: serenity::all::GuildChannel,
        guild: GuildId,
    ) -> impl std::future::Future<Output = ()> + Send;
}

impl ChannelData for Database {
    async fn new_channel(db: &Database, channel: serenity::all::GuildChannel,guild: GuildId) {
        let channel_id = channel.id.get();
        let created_channel: Result<Option<Channel>, surrealdb::Error> = db
            .client
            .create(("channel", channel_id as i64))
            .content(Channel {
                channel_id: channel_id as i64,
                muted: false,
                name: channel.name().to_string(),
            })
            .await;
        match created_channel {
            Ok(created_channel) => {
                debug!("Stored channel successfully {:?}", created_channel);

                info!("Sending channel guild relationship query");
                let query = format!(
                    "RELATE guild:{}->has_channel->channel:{};",
                    guild.get() as i64,
                    channel.id.get(),
                );

                let _ = db
                    .db_query(query)
                    .await
                    .expect("Unable to set guild_channel relation");
            }
            Err(e) => {
                warn!(
                    "Unable to store channel {} in database, may already exist.",
                    channel
                );
                error!("{e:?}")
            }
        }
    }
}
