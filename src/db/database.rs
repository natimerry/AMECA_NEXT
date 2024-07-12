use std::env;
use std::fmt::{Debug, Display};
use log::error;

use serde::de::DeserializeOwned;
use surrealdb::{Error, Response};
use surrealdb::engine::remote::ws::{Client, Ws};
// use surrealdb::sql::query;
use surrealdb::opt::auth::Root;
use surrealdb::opt::IntoQuery;
use surrealdb::Surreal;
use tracing::{debug, info};

#[derive(Clone)]
pub struct Database {
    pub client: Surreal<Client>,
    pub name_space: String,
    pub db_name: String,
}

impl Database {
    pub async fn init(address: String) -> Result<Self, Error> {
        info!({ address }, "Initialising SurrealDB on address:");

        let client = Surreal::new::<Ws>(address).await?;
        client
            .signin(Root {
                username: &env::var("SURREALDB_USER").expect("No SURREALDB_USER"),
                password: &env::var("SURREALDB_PASS").expect("NO SURREALDB_PASS"),
            })
            .await?;

        client
            .use_ns("database")
            .use_db("storage")
            .await
            .unwrap();
        // TODO: schema

        Ok(Database {
            client,
            name_space: String::from("AMECA_NEXT"),
            db_name: String::from("storage"),
        })
    }

    pub async fn set_schema(&mut self,schema: String) -> Result<(),surrealdb::Error> {
        info!("Starting migrations");

        let queries =
            schema.lines().filter(|string|!(*string).starts_with("--"))
                .filter(|string| !(*string).is_empty()).collect::<Vec<&str>>();

        for query in queries{
            let mut query = self.db_query(query).await?;
            for i in 0..query.num_statements() {
                let result: Result<Option<String>, Error> = query.take(i);
                match result {
                    Ok(_) => {}
                    Err(E) => {
                        error!("{}", E.to_string());
                        panic!();
                    }
                }
            }
        }
        Ok(())
    }
    pub async fn db_query<R>(
        &self,
        query: R,
    ) -> Result<Response, surrealdb::Error>
    where
        R: Into<String> + Debug + IntoQuery + std::fmt::Display
    {
        debug!("Sending query: {query}");
        Ok(self.client.query(query).await.unwrap())
    }
}