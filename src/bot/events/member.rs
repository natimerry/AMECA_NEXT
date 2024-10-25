use poise::serenity_prelude::{
    self as serenity, Color, CreateEmbed, CreateEmbedAuthor, GuildId, Member, User,
};

use serenity::Context;
use sqlx::types::chrono::Utc;
use sqlx::PgPool;
use tracing::info;

use crate::models::channel::ChannelData;
use crate::models::member::MemberData;
use crate::{bot::AMECA, models::channel::Channel, BoxResult};
pub async fn on_user_join(ctx: &Context, data: &AMECA, new_member: &Member) -> BoxResult<()> {
    let guild_id = new_member.guild_id;

    let embed = CreateEmbed::new()
        .author(CreateEmbedAuthor::new("AMECA").url("https://github.com/natimerry/AMECA_NEXT"))
        .title(format!("{} joined!", new_member.user.name))
        .color(Color::from_rgb(0, 255, 0))
        .field("Join Time", format!("`{}`", Utc::now()), false)
        .field(
            "User Details",
            format!("id: {}", new_member.user.id),
            false,
        );

    let username = new_member.user.name.clone();
    info!("User {username} has joined the guild {guild_id}");
    Channel::send_to_logging_channel(embed, ctx, &data.db, guild_id).await?;
    PgPool::mark_user_in_guild(&data.db, new_member.user.clone(), guild_id, Utc::now()).await?;
    Ok(())
}

pub async fn user_leave(
    ctx: &Context,
    data: &AMECA,
    guild_id: GuildId,
    user: &User,
) -> BoxResult<()> {
    let time_of_join = PgPool::get_user_join_time(&data.db, user.clone(), guild_id).await?;
    let total_seconds = (Utc::now() - time_of_join).num_seconds();

    let days = total_seconds / 86400;
    let hours = (total_seconds % 86400) / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;

    let embed = CreateEmbed::new()
        .author(CreateEmbedAuthor::new("AMECA").url("https://github.com/natimerry/AMECA_NEXT"))
        .title(format!("{} left", user.name))
        .color(Color::from_rgb(255, 0, 0))
        .field("Join Time", format!("`{}`", time_of_join), false)
        .field("Leave Time", format!("`{}`", Utc::now()), false)
        .field("Time of stay ", format!("`{} Days {} Hours {} Minutes {} Seconds`", days,hours,minutes,seconds), false)
        .field(
            "User Details",
            format!("id: {}", user.id),
            false,
        );

    let username = user.name.clone();
    info!("User {username} has joined the guild {guild_id}");
    Channel::send_to_logging_channel(embed, ctx, &data.db, guild_id).await?;
    Ok(())
}
