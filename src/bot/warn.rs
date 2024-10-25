use poise::serenity_prelude::{Color, CreateEmbed, User};
use tracing::{debug, info};

use crate::{
    models::channel::{Channel, ChannelData},
    BoxResult, Context,
};

#[poise::command(
    prefix_command,
    slash_command,
    guild_only = true,
    required_permissions = "KICK_MEMBERS",
    required_bot_permissions = "KICK_MEMBERS"
)]
pub async fn warn<'a>(ctx: Context<'a>, user: User, reason: Option<String>) -> BoxResult<()> {
    let user_id = user.id.get() as i64;
    let guild_id = ctx.guild_id().unwrap().get() as i64;
    // issue a new warning to dude

    ctx.defer().await?;


    sqlx::query!(
        "INSERT INTO warnings_guild_member(guild_id, member_id) VALUES ($1,$2)",
        guild_id,
        user_id
    )
    .execute(&ctx.data().db)
    .await?;
    info!(user_id, guild_id, "Set warning relationship");
    
    let row: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM warnings_guild_member WHERE guild_id = $1 AND member_id = $2",
    )
    .bind(guild_id)
    .bind(user_id)
    .fetch_one(&ctx.data().db)
    .await?;

    debug!("{} {}", row.0, "Received warnings from count query...");
    ctx.say(format!(
        "<@{}> you have been warned. You have been warned {} times.\n Reason: {}",
        user_id,
        row.0,
        reason.clone().unwrap_or("None provided".to_string())
    ))
    .await?;

    let embed = CreateEmbed::new()
        .title("Warning issued")
        .field("User", user.name, false)
        .field("Total Warnings", row.0.to_string(), false)
        .field(
            "Reason",
            reason.unwrap_or("None provided".to_string()),
            false,
        ).color(Color::from_rgb(255,255,0));

    Channel::send_to_logging_channel(embed, ctx, &ctx.data().db, ctx.guild_id().unwrap()).await?;

    Ok(())
}