use crate::utils::check_if_author_is_bot;
use crate::{BoxResult, Context};
use poise::serenity_prelude::{
    CacheHttp, CreateMessage, EditMember, GuildId, Member, Message, MessageBuilder, UserId,
};
use regex::Regex;
use sqlx::types::chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use std::sync::LazyLock;
use tracing::{debug, info};
static USER_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    info!("Compiling user mention checking regex");
    Regex::new("<@.{18}>").expect("Unable to compile regex")
});
#[poise::command(slash_command, prefix_command, guild_only = true, category = "utility")]
pub async fn afk(ctx: Context<'_>, #[rest] reason: Option<String>) -> BoxResult<()> {
    // Fetch guild and author details
    let guild_id = ctx.guild_id().expect("Unable to get guild_id").get() as i64;
    let author = ctx.author().id.get() as i64;
    let pool = ctx.data().db.clone();
    if check_if_author_is_afk(pool, author, guild_id).await? {
        debug!("Ignore AFK command since user is already afk");
    }
    tracing::info!(
        "Received AFK command from user ID: {}, in guild ID: {}",
        author,
        guild_id
    );

    // Defer response to indicate processing has begun
    ctx.defer().await?;

    // Retrieve member information and set up nickname and reason for AFK
    let mut guild_member = ctx.author_member().await.unwrap();
    let mut guild_member = guild_member.to_mut().clone();
    tracing::debug!("Fetched guild member details: {:?}", guild_member);

    let username = guild_member
        .nick
        .clone()
        .unwrap_or_else(|| guild_member.clone().user.name);
    let reason = reason.unwrap_or("No reason provided".to_string());

    // Check if user does not already have the [AFK] prefix and set nickname if not
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

    // Insert a new AFK entry in the database with member and guild info
    sqlx::query!(
        "INSERT INTO afk_member_guild(member_id, guild_id, time_afk,previous_name,reason) VALUES ($1, $2, $3,$4,$5)",
        author,
        guild_id,
        Utc::now(),
        username,
        reason
    )
        .execute(&ctx.data().db)
        .await?;

    // Send an announcement that the user is AFK
    ctx.say(format!("{} is AFK: {}", username, reason)).await?;
    tracing::info!("Announced AFK status for user: {}", username);
    tracing::debug!(
        "AFK status logged in the database for user ID: {} in guild ID: {}",
        author,
        guild_id
    );
    tracing::trace!("AFK command processed successfully for user ID: {}", author);
    Ok(())
}

pub async fn check_afk(
    ctx: &poise::serenity_prelude::Context,
    data: &PgPool,
    new_message: &Message,
) -> BoxResult<()> {
    if check_if_author_is_bot(new_message) {
        return Ok(());
    }

    let channel = new_message.channel_id;
    let guild_id = new_message.guild_id.unwrap().get() as i64;
    let member_id = new_message.author.id.get() as i64;
    // check if author had set AFK

    let is_afk = check_if_author_is_afk(data.clone(), member_id, guild_id).await?;
    if is_afk && !new_message.content.starts_with("!afk") {
        let mut member = ctx
            .http
            .get_member(GuildId::from(guild_id as u64), new_message.author.id)
            .await?;
        unafk(&ctx, &mut member, data.clone()).await?;
        let x = MessageBuilder::new()
            .push("Welcome back ")
            .mention(&new_message.author)
            .build();
        channel
            .send_message(ctx, CreateMessage::new().content(&x))
            .await?;
    }

    let matches = USER_REGEX
        .find_iter(&new_message.content)
        .map(|m| {
            let mention = m.as_str().to_string();
            let len = m.len();
            let slice = &mention[2..len - 1];
            debug!("UserID slice matched by regex: {slice} from {}", mention);
            slice.parse::<i64>().expect("INVALID USER ID")
        })
        .collect::<Vec<_>>();

    if matches.is_empty() {
        return Ok(());
    }

    for user in matches {
        #[derive(FromRow, Debug)]
        struct Data {
            time_afk: DateTime<Utc>,
            reason: String,
        }
        let time_of_afk: Option<Data> = sqlx::query_as(
            "SELECT time_afk,reason FROM afk_member_guild WHERE guild_id = $1 AND member_id = $2",
        )
        .bind(guild_id)
        .bind(user)
        .fetch_optional(data)
        .await?;
        debug!("Time of afk = {:?}", &time_of_afk);
        if let Some(time_of_afk) = time_of_afk {
            let user = ctx
                .http
                .get_user(UserId::new(user.try_into().unwrap()))
                .await?
                .name;

            let total_seconds = (Utc::now() - time_of_afk.time_afk).num_seconds();

            let days = total_seconds / 86400;
            let hours = (total_seconds % 86400) / 3600;
            let minutes = (total_seconds % 3600) / 60;
            let seconds = total_seconds % 60;

            let content = CreateMessage::new().content(format!(
                "{} is afk for `{} Days {} Hours {} Minutes {} Seconds`\nReason: {}",
                user, days, hours, minutes, seconds, time_of_afk.reason
            ));
            new_message
                .channel(&ctx)
                .await?
                .id()
                .send_message(&ctx, content)
                .await?;
        }
    }

    Ok(())
}

pub async fn unafk(ctx: impl CacheHttp, member: &mut Member, pool: PgPool) -> BoxResult<()> {
    // Retrieve the member's nickname or use their username if no nickname is set
    tracing::trace!(
        "Original nickname or username for user ID {}: {}",
        member.user.id,
        &member.nick.clone().unwrap()
    );

    // Remove any "[AFK] " prefix if it exists in the nickname and restore previous name
    let member_id = member.user.id.get() as i64;
    let guild_id = member.guild_id.get() as i64;
    let previous_name: (String,) = sqlx::query_as(
        "SELECT previous_name FROM afk_member_guild WHERE member_id = $1 and guild_id = $2",
    )
    .bind(member_id)
    .bind(guild_id)
    .fetch_one(&pool)
    .await?;
    debug!(
        "Previous username for user: {} in database was {:?}",
        member.guild_id, previous_name
    );
    let previous_name = previous_name.0;

    tracing::debug!("Username after removing [AFK] prefix: {}", previous_name);

    // Attempt to set the updated nickname
    if let Err(e) = member
        .edit(ctx.http(), EditMember::new().nickname(&previous_name))
        .await
    {
        tracing::error!(
            "Error in setting nickname for user ID: {}. Error: {:?}",
            member.user.id,
            e
        );
    } else {
        tracing::debug!(
            "Nickname updated to '{}' for user ID: {}",
            previous_name,
            member.user.id
        );
    }

    tracing::info!(
        "Removing AFK status for user ID: {} in guild ID: {}",
        member_id,
        guild_id
    );

    // Remove AFK status from the database for the member in the specified guild
    sqlx::query!(
        "DELETE FROM afk_member_guild WHERE member_id = $1 AND guild_id = $2",
        member_id,
        guild_id
    )
    .execute(&pool)
    .await?;
    tracing::info!(
        "AFK status successfully removed from database for user ID: {} in guild ID: {}",
        member_id,
        guild_id
    );

    tracing::trace!("Finished processing unAFK for user ID: {}", member_id);
    Ok(())
}

pub async fn check_if_author_is_afk(pool: PgPool, author: i64, guild: i64) -> BoxResult<bool> {
    // Early return if the message starts with "!afk" as the command should not trigger AFK check

    tracing::trace!(
        "Checking AFK status for user ID: {} in guild ID: {}",
        author,
        guild
    );

    // Define a structure to hold the queried member ID from the database
    #[derive(FromRow, Debug)]
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
    tracing::debug!(
        "AFK status for user ID {} in guild ID {}: {}",
        author,
        guild,
        is_afk
    );

    tracing::trace!("Finished AFK status check for user ID {}", author);
    Ok(is_afk)
}
