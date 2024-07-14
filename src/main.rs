pub mod bot;
mod db;
pub mod models;

use crate::bot::AMECA;
use clap::Parser;
use tracing::{debug, error, info, trace, warn, Level};
use tracing_subscriber::fmt::writer::MakeWriterExt;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(long, default_value_t = false)]
    test: bool,
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().expect("NO .ENV file found");

    let debug_file =
        tracing_appender::rolling::hourly("./logs/", "debug").with_max_level(tracing::Level::TRACE);

    let warn_file = tracing_appender::rolling::hourly("./logs/", "warnings")
        .with_max_level(tracing::Level::WARN);
    let all_files = debug_file.and(warn_file);

    tracing_subscriber::registry()
        .with(EnvFilter::from_env("AMECA_LOG_LEVEL"))
        // .with(console_layer)
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(all_files)
                .with_ansi(false),
        )
        .with(
            tracing_subscriber::fmt::Layer::new()
                .with_ansi(true)
                .with_writer(std::io::stdout.with_max_level(Level::DEBUG))
                .with_file(true)
                .with_line_number(true),
        )
        .init();

    let token = std::env::var("DISCORD_TOKEN").expect("Expected a token.");
    debug!("Loaded token {}", token);

    let args = Args::parse();

    AMECA::start_shard(&token, args.test).await;
}
