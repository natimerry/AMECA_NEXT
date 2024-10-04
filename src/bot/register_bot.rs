use crate::bot::AMECA;
use crate::models::channel::Channel;
use crate::models::member::Members;
use crate::{BoxResult, DynError};
use poise::command;
use poise::futures_util::{Stream, StreamExt};
use poise::serenity_prelude as serenity;
use poise::serenity_prelude::futures;
use sqlx::PgPool;
use tracing::{debug, error, info};

type Context<'a> = poise::Context<'a, AMECA, DynError>;

async fn autocomplete_channel<'a>(
    ctx: Context<'_>,
    partial: &'a str,
) -> impl Stream<Item = String> + 'a {
    let guild_id = ctx.guild_id().expect("Cannot get guild ID");

    let channel_name = sqlx::query_as::<_, Channel>("SELECT * from channel WHERE guild_id = $1")
        .bind(guild_id.get() as i64)
        .fetch_all(&ctx.data().db)
        .await
        .expect("Error getting autocomplete channels")
        .iter()
        .map(|channel| channel.channel_name.clone())
        .collect::<Vec<_>>();

    let channel_binding = channel_name.clone();
    let x = futures::stream::iter(channel_binding.to_owned())
        .filter(move |name| futures::future::ready(name.starts_with(partial)))
        .map(|name| name.clone().to_string());
    x
}

pub async fn check_existing_log_channel(
    guild_id: i64,
    pool: &PgPool,
) -> BoxResult<Option<Channel>> {
    let x = sqlx::query_as::<_, Channel>(
        "SELECT * FROM channel WHERE guild_id = $1 AND logging_channel = true",
    )
    .bind(guild_id)
    .fetch_optional(pool)
    .await?;

    Ok(x)
}
#[command(slash_command)]
pub async fn register_logging_channel(
    ctx: Context<'_>,
    #[description = "Select logging channel"]
    #[autocomplete = "autocomplete_channel"]
    channel: serenity::Channel,
) -> BoxResult<()> {
    let channel_id = channel.id().get() as i64;
    let guild_id = ctx.guild_id().expect("Cannot get guild ID").get() as i64;
    
    match check_existing_log_channel(guild_id, &ctx.data().db).await {
        Ok(Some(channel)) => {
            ctx.say("Logging channel already registered").await?;
            ctx.say(format!("Deregister existing channel {} <{}>",channel.channel_id,channel.channel_name)).await?;
            return Ok(());
        }
        Ok(None) => (),
        Err(e) => {
            error!("Error checking existing logging channel: {}", e);
            ctx.say("Error checking existing logging channel").await?;
            return Err(e.into());
        }
    }
    info!(
        "Setting up logging channel: {} for guild {}",
        channel,
        ctx.guild_id().expect("Cannot get guild ID")
    );
    let mut conn = &ctx.data().db.acquire().await?;
    let x = sqlx::query!(
        "UPDATE channel SET logging_channel = $1 WHERE guild_id = $2 AND channel_id = $3",
        true,
        guild_id,
        channel_id
    )
    .execute(&ctx.data().db)
    .await;
    match x {
        Ok(affected_rows) => {
            debug!("Insertion affected {} rows", affected_rows.rows_affected());
            ctx.say("Set logging channel successfully").await?;
        }
        Err(e) => {
            ctx.say("Unable to set logging channel!").await?;
            error!("{e:#?}")
        }
    }
    Ok(())
}
