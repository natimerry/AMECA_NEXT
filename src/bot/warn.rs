use std::time::Duration;
use poise::serenity_prelude::{CacheHttp, Color, CreateEmbed, CreateMessage, GuildId, Member, Timestamp, UserId};
use sqlx::{ FromRow, PgPool};
use sqlx::types::chrono::Utc;
use tracing::{debug, info};

use crate::bot::warn::WarnTrigger::{Ban, Kick, Mute};
use crate::{
    models::channel::{Channel, ChannelData},
    BoxResult, Context,
};

#[derive(FromRow)]
struct WarningTriggerData {
    limit: i32,
    action: String,
}

async fn get_warning_count(member_id: i64, guild_id: i64, pool: &PgPool) -> BoxResult<i64> {
    let row: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM warnings_guild_member WHERE guild_id = $1 AND member_id = $2",
    )
        .bind(guild_id)
        .bind(member_id)
        .fetch_one(pool)
        .await?;

    Ok(row.0)
}

async fn process_triggers(ctx: &Context<'_>, triggers: Vec<WarningTriggerData>, member_id: i64, guild_id: i64) -> BoxResult<()> {
    for trigger in triggers {
        let limit = trigger.limit;
        let action = &trigger.action;

        let current_warnings = get_warning_count(member_id, guild_id, &ctx.data().db).await?;
        debug!("Current warning: {} Limit: {} Action: {}",current_warnings,limit,action);
        let guild_id = guild_id as u64;
        let member_id = member_id as u64;
        if limit <= current_warnings as i32 {
            match WarnTrigger::from(action.to_string()) {
                Ban => {
                    ctx.http().ban_user(GuildId::from(guild_id), UserId::from(member_id), 0, Some("Warning for bans trigger limit reached")).await?;
                }
                Mute => {
                    let mut member = ctx.http().get_member(GuildId::from(guild_id),UserId::from(member_id)).await?;
                    member.disable_communication_until_datetime(ctx,Timestamp::from(Utc::now() + Duration::from_secs(3600))).await?;
                    
                }
                Kick => {
                    ctx.http().kick_member(GuildId::from(guild_id), UserId::from(member_id), Some("Warning for kick trigger limit reached")).await?;
                }
            }
        }
    }
    Ok(())
}
pub async fn __warn(ctx: Context<'_>, member: Member, reason: Option<String>) -> BoxResult<()> {
    let user = member.clone().user;
    let user_id = user.id.get() as i64;
    let guild_id = ctx.guild_id().unwrap().get() as i64;
    // issue a new warning to dude

    ctx.defer().await?;

    let serenity_http = ctx.serenity_context();
    let perms = member
        .permissions(serenity_http)
        .expect("Unable to get permisssions for user");

    if perms.administrator() || perms.manage_guild() || perms.manage_messages() {
        ctx.say("You are not allowed to warn this user").await?;
        return Ok(());
    }

    sqlx::query!(
        "INSERT INTO warnings_guild_member(guild_id, member_id) VALUES ($1,$2)",
        guild_id,
        user_id
    )
        .execute(&ctx.data().db)
        .await?;
    info!(user_id, guild_id, "Set warning relationship");

    let count = get_warning_count(user_id, guild_id, &ctx.data().db).await?;

    let data: Vec<WarningTriggerData> = sqlx::query_as(
        "SELECT number_of_warns as limit,action FROM warn_triggers_guild WHERE guild_id = $1",
    )
        .bind(guild_id)
        .fetch_all(&ctx.data().db)
        .await?;
    if !data.is_empty() {
        process_triggers(&ctx, data,user_id,guild_id).await?;
    }

    debug!("{} {}", count, "Received warnings from count query...");
    ctx.say(format!(
        "<@{}> you have been warned. You have been warned {} times.\nReason: {}",
        user_id,
        count,
        reason.clone().unwrap_or("None provided".to_string())
    ))
        .await?;

    let embed = CreateEmbed::new()
        .title("Warning issued")
        .field("User", user.name, false)
        .field("Total Warnings", count.to_string(), false)
        .field(
            "Reason",
            reason.unwrap_or("None provided".to_string()),
            false,
        )
        .color(Color::from_rgb(255, 255, 0));

    Channel::send_to_logging_channel(embed, ctx, &ctx.data().db, ctx.guild_id().unwrap()).await?;
    Ok(())
}

#[poise::command(
    prefix_command,
    slash_command,
    guild_only = true,
    required_permissions = "KICK_MEMBERS",
    required_bot_permissions = "KICK_MEMBERS",
    category = "moderation",
    aliases("warn"),
    subcommands("show_warnings", "clear_warnings", "warn","warn_trigger")
)]
pub async fn warnings<'a>(
    ctx: Context<'a>,
    member: Member,
    #[rest] reason: Option<String>,
) -> BoxResult<()> {
    __warn(ctx, member, reason).await?;
    Ok(())
}
#[poise::command(
    prefix_command,
    slash_command,
    guild_only = true,
    required_permissions = "KICK_MEMBERS",
    required_bot_permissions = "KICK_MEMBERS",
    category = "moderation"
)]
pub async fn warn(ctx: Context<'_>, member: Member, reason: Option<String>) -> BoxResult<()> {
    __warn(ctx, member, reason).await?;
    Ok(())
}

#[poise::command(
    prefix_command,
    slash_command,
    guild_only = true,
    required_permissions = "KICK_MEMBERS",
    required_bot_permissions = "KICK_MEMBERS",
    category = "moderation"
)]
pub async fn show_warnings(ctx: Context<'_>, member: Member) -> BoxResult<()> {
    ctx.say("Fetching data...").await?;

    let member_id = member.user.id.get() as i64;
    let guild_id = ctx.guild_id().unwrap().get() as i64;
    let warnings = get_warning_count(member_id, guild_id, &ctx.data().db).await?;

    let username = member.nick.unwrap_or(member.user.name);

    let embed = CreateEmbed::new()
        .title("Warning issued")
        .field("User", username, false)
        .field("Total Warnings", warnings.to_string(), false)
        .color(Color::from_rgb(255, 255, 0));
    let message = CreateMessage::new().embed(embed);
    ctx.channel_id()
        .send_message(ctx.serenity_context(), message)
        .await?;
    Ok(())
}

#[poise::command(
    prefix_command,
    slash_command,
    guild_only = true,
    required_permissions = "KICK_MEMBERS",
    required_bot_permissions = "KICK_MEMBERS",
    category = "moderation"
)]
pub async fn clear_warnings(ctx: Context<'_>, member: Member) -> BoxResult<()> {
    ctx.defer().await?;

    let member_id = member.user.id.get() as i64;
    let guild_id = ctx.guild_id().unwrap().get() as i64;

    sqlx::query!(
        "DELETE FROM warnings_guild_member WHERE member_id = $1 AND guild_id = $2",
        member_id,
        guild_id
    )
        .execute(&ctx.data().db)
        .await?;

    ctx.say("Removed any warnings issued to the user").await?;
    Ok(())
}

#[derive(sqlx::Type, Debug, poise::ChoiceParameter)]
pub enum WarnTrigger {
    #[name = "Ban the user"]
    Ban,
    #[name = "Timeout the user"]
    Mute,
    #[name = "Kick the user"]
    Kick,
}
#[poise::command(
    prefix_command,
    slash_command,
    guild_only = true,
    required_permissions = "BAN_MEMBERS",
    required_bot_permissions = "BAN_MEMBERS",
    category = "moderation"
)]
pub async fn warn_trigger(
    ctx: Context<'_>,
    choice: WarnTrigger,
    #[min = 2]
    #[max = 100]
    limit: i32,
) -> BoxResult<()> {
    ctx.defer().await?;
    let guild_id = ctx.guild_id().unwrap().get() as i64;
    info!(
        "Setting new warnings trigger for guild: {} at limit: {}; do: {:?}",
        guild_id, limit, choice
    );

    let db_choice = String::from(choice);
    sqlx::query!(
        "INSERT INTO warn_triggers_guild(guild_id,action,number_of_warns) VALUES($1,$2,$3)",
        guild_id,
        db_choice,
        limit
    )
        .execute(&ctx.data().db)
        .await?;
    ctx.reply("Set triggers").await?;
    Ok(())
}

impl From<WarnTrigger> for String {
    fn from(value: WarnTrigger) -> Self {
        match value {
            Ban => String::from("ban"),
            Mute => String::from("mute"),
            Kick => String::from("kick"),
        }
    }
}

fn warn_enum_to_str<S: Into<String>>(value: S) -> WarnTrigger {
    let value_str = value.into();
    return match value_str.to_lowercase().as_str() {
        "ban" => Ban,
        "mute" => Mute,
        "kick" => Kick,
        _ => Mute,
    };
}
impl From<String> for WarnTrigger {
    fn from(value: String) -> Self {
        warn_enum_to_str(value)
    }
}
impl From<&str> for WarnTrigger {
    fn from(value: &str) -> Self {
        warn_enum_to_str(value)
    }
}
