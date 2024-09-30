use sqlx::Error::Database;
use tracing::Level;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::fmt::writer::MakeWriterExt;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

mod db;

type DynError = Box<dyn std::error::Error + Send + Sync>;
type BoxResult<T> = Result<T, DynError>;

#[tokio::main]
async fn main() {
    dotenv::dotenv().expect("Failed to read .env file");

    let debug_file =
        tracing_appender::rolling::hourly("./logs/", "debug").with_max_level(tracing::Level::TRACE);

    let warn_file = tracing_appender::rolling::hourly("./logs/", "warnings")
        .with_max_level(tracing::Level::WARN);
    let all_files = debug_file.and(warn_file);

    tracing_subscriber::registry()
        .with(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env()
                .expect("Unable to read log level"),
        )
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

    let db = db::database::Database::init().await.expect("db init failed");
}
