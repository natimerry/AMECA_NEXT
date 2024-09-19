use crate::db::database::Database;
use log::debug;
use serde::{de, Deserialize, Deserializer, Serialize};
use tracing::{error, warn};

pub trait UserData {
    fn new_user(
        db: &Database,
        user: serenity::all::User,
    ) -> impl std::future::Future<Output = ()> + Send;
}

fn de_from_str<'de, D>(deserializer: D) -> Result<i64, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    let x: Result<i64, D::Error> = s.parse().map_err(de::Error::custom);
    return x;
}
#[derive(Debug, Deserialize, Serialize)]
pub struct Members {
    pub member_id: i64,
    pub admin: bool,
    pub banned: bool,
    pub name: String, // real name
    #[serde(deserialize_with = "de_from_str")]
    pub warnings_issued: i64,
}

impl UserData for Database {
    async fn new_user(db: &Database, user: serenity::all::User) {
        debug!("Storing user {}", &user);
        let mem = Members {
            member_id: user.id.get() as i64,
            admin: false,
            banned: false,
            name: (user.name).to_string(),
            warnings_issued: 0,
        };
        debug!("Storing user {:#?}", &mem);

        let created_user: Result<Option<Members>, surrealdb::Error> = db
            .client
            .create(("members", user.id.get() as i64))
            .content(mem)
            .await;

        match created_user {
            Ok(msg) => {
                debug!("Stored user successfully {:?}", user);
                // todo: relate message to author after user database model is created
            }
            Err(e) => {
                warn!("Unable to store user in database, may already exist.",);
                error!("{e:?}")
            }
        }
    }
}
