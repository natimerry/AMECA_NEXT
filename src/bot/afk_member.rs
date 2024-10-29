use crate::{BoxResult, Context};
use poise::serenity_prelude::{CacheHttp, EditMember, Member, Message};
use sqlx::types::chrono::Utc;
use sqlx::{FromRow, PgPool};

#[poise::command(slash_command, prefix_command, guild_only = true, category = "utility")]
pub async fn afk(ctx: Context<'_>, #[rest] reason: Option<String>) -> BoxResult<()> {
    // Fetch guild and author details
    let guild_id = ctx.guild_id().expect("Unable to get guild_id").get() as i64;
    let author = ctx.author().id.get() as i64;

    tracing::info!(
        "Received AFK command from user ID: {}, in guild ID: {}",
        author,
        guild_id
    );

    // Defer response to indicate processing has begun
    ctx.defer().await?;

    // Insert a new AFK entry in the database with member and guild info
    sqlx::query!(
        "INSERT INTO afk_member_guild(member_id, guild_id, time_afk) VALUES ($1, $2, $3)",
        author,
        guild_id,
        Utc::now()
    )
    .execute(&ctx.data().db)
    .await?;
    tracing::info!(
        "AFK status logged in the database for user ID: {} in guild ID: {}",
        author,
        guild_id
    );

    // Retrieve member information and set up nickname and reason for AFK
    let mut guild_member = ctx.author_member().await.unwrap();
    let mut guild_member = guild_member.to_mut().clone();
    tracing::debug!("Fetched guild member details: {:?}", guild_member);

    let username = guild_member
        .nick
        .clone()
        .unwrap_or_else(|| guild_member.clone().user.name);
    let reason = reason.unwrap_or("No reason provided".to_string());

    // Send an announcement that the user is AFK
    ctx.say(format!("{} is AFK: {}", username, reason)).await?;
    tracing::info!("Announced AFK status for user: {}", username);

    // Check if user does not already have the [AFK] prefix and set nickname if not
    if !username.starts_with("[AFK]") {
        if let Err(e) = guild_member
            .edit(
                ctx.http(),
                EditMember::new().nickname(format!("[AFK] {username}")),
            )
            .await
        {
            tracing::error!(
                "Failed to set nickname for user ID: {}. Error: {:?}",
                author,
                e
            );
            ctx.say(format!("Couldn't set nickname for user: {}", e))
                .await?;
        } else {
            tracing::debug!(
                "Nickname successfully updated to [AFK] for user ID: {}",
                author
            );
        }
    } else {
        tracing::trace!(
            "Nickname already contains [AFK] prefix for user ID: {}",
            author
        );
    }

    tracing::info!("AFK command processed successfully for user ID: {}", author);
    Ok(())
}

pub async fn unafk(ctx: impl CacheHttp, member: &mut Member, pool: PgPool) -> BoxResult<()> {
    // Retrieve the member's nickname or use their username if no nickname is set
    let mut username = member.nick.clone().unwrap_or(member.user.name.to_string());
    tracing::trace!("Original nickname or username for user ID {}: {}", member.user.id, &username);

    // Remove any "[AFK] " prefix if it exists in the nickname
    while username.starts_with("[AFK] ") {
        username = username.replace("[AFK] ", "");
    }
    tracing::debug!("Username after removing [AFK] prefix: {}", username);

    // Attempt to set the updated nickname
    if let Err(e) = member
        .edit(ctx.http(), EditMember::new().nickname(&username))
        .await
    {
        tracing::error!("Error in setting nickname for user ID: {}. Error: {:?}", member.user.id, e);
    } else {
        tracing::info!("Nickname updated to '{}' for user ID: {}", username, member.user.id);
    }

    // Capture member and guild IDs for logging and database operations
    let member_id = member.user.id.get() as i64;
    let guild_id = member.guild_id.get() as i64;
    tracing::info!("Removing AFK status for user ID: {} in guild ID: {}", member_id, guild_id);

    // Remove AFK status from the database for the member in the specified guild
    sqlx::query!(
        "DELETE FROM afk_member_guild WHERE member_id = $1 AND guild_id = $2",
        member_id,
        guild_id
    )
        .execute(&pool)
        .await?;
    tracing::info!("AFK status successfully removed from database for user ID: {} in guild ID: {}", member_id, guild_id);

    tracing::trace!("Finished processing unAFK for user ID: {}", member_id);
    Ok(())
}


pub async fn check_if_author_is_afk(msg: Message, pool: PgPool) -> BoxResult<bool> {
    // Early return if the message starts with "!afk" as the command should not trigger AFK check
    if msg.content.starts_with("!afk") {
        tracing::trace!("Message starts with '!afk'; skipping AFK check.");
        return Ok(false);
    }

    // Get author and guild IDs for AFK status check
    let author = msg.author.id.get() as i64;
    let guild = msg.guild_id.unwrap().get() as i64;
    tracing::info!("Checking AFK status for user ID: {} in guild ID: {}", author, guild);

    // Define a structure to hold the queried member ID from the database
    #[derive(FromRow,Debug)]
    struct DummyMember {
        member_id: i64,
    }

    // Query the database to check if the author is marked as AFK in this guild
    let member_id: Option<DummyMember> = sqlx::query_as(
        "SELECT member_id FROM afk_member_guild WHERE member_id = $1 AND guild_id = $2",
    )
        .bind(author)
        .bind(guild)
        .fetch_optional(&pool)
        .await?;
    tracing::debug!("Database query result for AFK status: {:?}", member_id);

    // Return true if an AFK entry is found, otherwise false
    let is_afk = member_id.is_some();
    tracing::info!("AFK status for user ID {} in guild ID {}: {}", author, guild, is_afk);

    tracing::trace!("Finished AFK status check for user ID {}", author);
    Ok(is_afk)
}
