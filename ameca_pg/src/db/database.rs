use tracing::{info, log};
use log::error;
use sqlx::{Postgres};
use tracing::log::log;
use crate::BoxResult;

#[derive(Clone, Debug)]
pub struct Database {
    pub client: sqlx::Pool<Postgres>,
}
impl Database {
    pub async fn init() -> BoxResult<Self>{
        let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        info!("Connecting to Postgres at {}", &url);
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(256).connect(&url).await?;

        info!("Running migrations");
        sqlx::migrate!("./migrations").run(&pool).await?;

        Ok(Self{
            client: pool,
        })
    }
}