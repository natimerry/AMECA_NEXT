use crate::db::database::Database;
use serde::{Deserialize, Serialize};
use serenity::all::{GuildId, GuildInfo};

use chrono::{DateTime, Utc};
use surrealdb::Error;
use tracing::{debug, error, info};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Guild {
    pub guild_id: String,
    pub time_of_join: DateTime<Utc>,
    pub members: u64,
    pub logging_channel: u64,
    pub systems_channel: u64,
}

pub trait GuildData {
    fn joined_guild(
        db: &Database,
        members: u64,
        guild_id: GuildId,
    ) -> impl std::future::Future<Output = ()> + Send;
    fn get_all_guilds(
        db: &Database,
    ) -> impl std::future::Future<Output = Option<Vec<Guild>>> + Send;
}



impl GuildData for Database {
    async fn joined_guild(db: &Database, members: u64, guild_id: GuildId) {
        info!("Registering new GUILD in the database");
        let created_guild: Result<Option<Guild>, surrealdb::Error> = db
            .client
            .create(("guild", guild_id.get()))
            .content(Guild {
                members,
                guild_id: guild_id.to_string(),
                time_of_join: Utc::now(),
                logging_channel: 0,
                systems_channel: 0,
            })
            .await;
        debug!("{:?}", created_guild);
    }

    async fn get_all_guilds(db: &Database) -> Option<Vec<Guild>> {
        let query = "SELECT * FROM guild;";
        let mut response = db
            .db_query(query)
            .await
            .expect("Unable to query database for guilds");
        debug!("{response:#?}");
        let guilds: Result<Vec<Guild>, Error> = response.take(0);
        return match guilds {
            Ok(found_guilds) => {
                debug!("{:#?}", found_guilds);
                Some(found_guilds)
            }
            Err(_) => {
                error!("Unable to query database for guilds!");
                None
            }
        };
    }
}
