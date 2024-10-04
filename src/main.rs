use crate::bot::AMECA;
use sqlx::{Pool, Postgres};
use tracing::level_filters::LevelFilter;
use tracing::{debug, info, Level};
use tracing_subscriber::fmt::writer::MakeWriterExt;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

mod bot;
mod models;

type DynError = Box<dyn std::error::Error + Send + Sync>;
type BoxResult<T> = Result<T, DynError>;
type Context<'a> = poise::Context<'a, AMECA, DynError>;

#[derive(Debug)]
struct Args {
    cache: bool,
}

fn parse_args() -> BoxResult<Args> {
    use lexopt::prelude::*;
    let mut cache = false;

    let mut parser = lexopt::Parser::from_env();
    while let Some(arg) = parser.next()? {
        match arg {
            Short('c') | Long("cache") => {
                cache = true;
            }
            _ => (),
        }
    }
    Ok(Args { cache })
}

pub async fn database_init() -> BoxResult<Pool<Postgres>> {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    info!("Connecting to Postgres at {}", &url);
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(256)
        .connect(&url)
        .await?;

    info!("Running migrations");
    sqlx::migrate!("./migrations").run(&pool).await?;

    Ok(pool)
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().expect("Failed to read .env file");
    let debug_file =
        tracing_appender::rolling::hourly("./logs/", "debug").with_max_level(Level::TRACE);

    let warn_file =
        tracing_appender::rolling::hourly("./logs/", "warnings").with_max_level(Level::WARN);
    let all_files = debug_file.and(warn_file);
    let console_layer = console_subscriber::spawn();

    tracing_subscriber::registry()
        .with(console_layer)
        .with(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::TRACE.into())
                .from_env()
                .expect("Unable to read log level"),
        )
        .with(EnvFilter::from_env("LOG_LEVEL"))
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(all_files)
                .with_ansi(false),
        )
        .with(
            tracing_subscriber::fmt::Layer::new()
                .with_ansi(true)
                .with_writer(std::io::stdout.with_max_level(Level::TRACE))
                .with_file(true)
                .with_line_number(true),
        )
        .init();

    let db = database_init().await.expect("db init failed");

    let token = std::env::var("DISCORD_TOKEN").expect("missing DISCORD_TOKEN");
    let args = parse_args();
    debug!("{:?}", args);
    if let Ok(args) = args {
        AMECA::start_shard(token, db, args.cache)
            .await
            .expect("Error starting shard");
    }
}
