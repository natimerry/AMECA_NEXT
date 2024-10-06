use crate::bot::automod::{cache_regex, log_msg_delete};
use crate::models::channel::{Channel as DbChannel, Channel, ChannelData};
use crate::{BoxResult, Context};
use log::{error, info};
use poise::futures_util::Stream;
use poise::serenity_prelude::RoleAction::Create;
use poise::serenity_prelude::{
    futures, CacheHttp, Color, CreateEmbed, CreateEmbedAuthor, CreateMessage,
};
use poise::futures_util::StreamExt;
use regex::Regex;
use sqlx::Row;
use std::ops::Deref;
use tracing::{debug, trace};
use tracing::field::debug;
use tracing_subscriber::fmt::format;

async fn autocomplete_pattern<'a>(
    ctx: Context<'_>,
    partial: &'a str,
) -> impl Stream<Item = String> + 'a {
    let guild_id = ctx.guild_id().expect("Cannot get guild ID");
    #[derive(Clone)]
    struct Name {
        name: Option<String>,
    }
    let pattern = sqlx::query_as!(
        Name,
        "SELECT name from prohibited_words_for_guild WHERE guild_id = $1",
        guild_id.get() as i64
    )
    // .bind(guild_id.get() as i64)
    .fetch_all(&ctx.data().db)
    .await
    .expect("Error getting autocomplete channels")
    .iter()
    .map(|s| s.clone().name.unwrap())
    .collect::<Vec<String>>();

    let pattern_binding = pattern.clone();
    let x = futures::stream::iter(pattern_binding.to_owned())
        .filter(move |name| futures::future::ready(name.starts_with(partial)))
        .map(|name| name.clone().to_string());
    x
}

#[poise::command(
    slash_command,
    guild_only = true,
    required_permissions = "MANAGE_MESSAGES",
    required_bot_permissions = "MANAGE_MESSAGES"
)]
pub async fn remove_banned_pattern(
    ctx: Context<'_>,
    #[description = "Name of the rule"]
    #[autocomplete = "autocomplete_pattern"]
    name: String,
) -> BoxResult<()> {
    let guild = ctx.guild().expect("Cannot get guild ID").id.get() as i64;
    info!("Removing regex entry `{}` from database ", name);
    sqlx::query!(
        "DELETE FROM prohibited_words_for_guild WHERE name = $1 AND guild_id=$2",
        name,
        guild
    ).execute(&ctx.data().db).await?;
    cache_regex(&ctx.data().db, &ctx.data()).await?;
    trace!("Cached map:{:#?}",ctx.data().db);
    let embed = CreateEmbed::new()
        .author(CreateEmbedAuthor::new("AMECA_NEXT").url("https://github.com/AMECA_NEXT"))
        .color(Color::from_rgb(0, 244, 0))
        .title(format!("Deleting regex entry `{}`", &name));
    DbChannel::send_to_logging_channel(
        embed,
        &ctx.serenity_context(),
        &ctx.data().db,
        ctx.guild_id().unwrap(),
    )
    .await?;
    ctx.say("Removed rule from database").await?;
    Ok(())
}

#[poise::command(
    slash_command,
    guild_only = true,
    required_permissions = "MANAGE_MESSAGES",
    required_bot_permissions = "MANAGE_MESSAGES"
)]
pub async fn ban_pattern(
    ctx: Context<'_>,
    #[description = "Name of the rule"] name: String,
    #[description = "Regular expression pattern"] pattern: String,
) -> BoxResult<()> {
    let guild = ctx.guild_id().expect("GuildID not found"); // unreachable error
    let guild = guild.get() as i64;
    let author = ctx.author().id.get() as i64;
    let mut regex = Regex::new(&pattern);

    let uh = pattern.clone();
    match regex {
        Ok(_re) => {
            ctx.say("Regex compiled! Enforcing pattern starting from now!")
                .await?;
            debug!("Applying regex pattern for {}", guild);
            struct DbGuildId(i64);
            sqlx::query(
                "INSERT INTO prohibited_words_for_guild(name, pattern, author, guild_id) VALUES ($1,$2,$3,$4) RETURNING id",
            ).bind(&name).bind(&pattern).bind(&author).bind(&guild)
                .fetch_one(&ctx.data().db).await?;

            ctx.data().cached_regex
                .entry(guild)
                .and_modify(|list| list.push(_re.clone()))
                .or_insert(vec![_re]);
            info!("Stored new regex entry `{}` for `{}`", name, guild);
            let embed = CreateEmbed::new()
                .author(CreateEmbedAuthor::new("AMECA_NEXT").url("https://github.com/AMECA_NEXT"))
                .color(Color::from_rgb(0, 244, 0))
                .title(format!("Storing regex entry `{}`", &name))
                .field("Pattern", format!("```\n{pattern}```"), false);
            DbChannel::send_to_logging_channel(
                embed,
                &ctx.serenity_context(),
                &ctx.data().db,
                ctx.guild_id().unwrap(),
            )
            .await?;
        }
        Err(e) => {
            debug!(
                "Error compiling regex pattern {} for guild {}",
                &pattern, &guild
            );
            ctx.say("Error in compiling regular expression pattern")
                .await?;
            let embed = CreateEmbed::new()
                .author(CreateEmbedAuthor::new("AMECA_NEXT").url("https://github.com/AMECA_NEXT"))
                .color(Color::from_rgb(220, 0, 220))
                .title("Failed to parse regex")
                .field(
                    "Error",
                    &format!("Pattern: {} ```\n{}```", pattern, e),
                    true,
                );
            let msg = CreateMessage::new().embed(embed);
            let channel = ctx.channel_id().send_message(&ctx, msg).await?;
            return Ok(());
        }
    }
    Ok(())
}