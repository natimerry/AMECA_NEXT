use crate::{BoxResult, Context};
use log::{error, info};
use poise::serenity_prelude::{CacheHttp, EditMember, Member, Message};
use sqlx::types::chrono::Utc;
use sqlx::{FromRow, PgPool};

#[poise::command(slash_command, prefix_command, guild_only = true, category = "utility")]
pub async fn afk(ctx: Context<'_>, #[rest] reason: Option<String>) -> BoxResult<()> {
    let guild_id = ctx.guild_id().expect("Unable to get guild_id").get() as i64;
    let author = ctx.author().id.get() as i64;

    ctx.defer().await?;
    // create new afk relationship in db
    sqlx::query!(
        "INSERT INTO afk_member_guild(member_id,guild_id,time_afk) VALUES ($1,$2,$3)",
        author,
        guild_id,
        Utc::now()
    )
        .execute(&ctx.data().db)
        .await?;

    let mut guild_member = ctx.author_member().await.unwrap();
    let mut guild_member = guild_member.to_mut().clone();
    let username = guild_member
        .nick
        .clone()
        .unwrap_or(guild_member.clone().user.name);
    let reason = reason.unwrap_or("No reason provided".to_string());

    ctx.say(format!("{} is AFK: {}", username, reason)).await?;

    if !username.starts_with("[AFK]") {
        if let Err(e) = guild_member
            .edit(
                ctx.http(),
                EditMember::new().nickname(format!("[AFK] {username}")),
            )
            .await
        {
            error!("Error in setting nickname for user {:?}", e);
            ctx.say(format!("Couldnt set nickhame for user: {}", e)).await?;
        }
    }
    Ok(())
}

pub async fn unafk(ctx: impl CacheHttp, member: &mut Member, pool: PgPool) -> BoxResult<()> {
    let mut username = member.nick.clone().unwrap_or(member.user.name.to_string());
    while username.starts_with("[AFK] ") {
        username = username.replace("[AFK] ", "");
    }
    if let Err(e) = member
        .edit(ctx.http(), EditMember::new().nickname(username))
        .await
    {
        error!("Error in setting afk {:?}", e);
    }

    let member_id = member.user.id.get() as i64;
    let guild_id = member.guild_id.get() as i64;
    info!("Removing afk status for {} at guild {}",member_id,guild_id);

    sqlx::query!("DELETE FROM afk_member_guild WHERE member_id = $1 AND guild_id = $2",member_id,guild_id).execute(&pool).await?;
    Ok(())
}

pub async fn check_if_author_is_afk(
    msg: Message,
    pool: PgPool,
) -> BoxResult<bool> {
    if msg.content.starts_with("!afk") {
        return Ok(false);
    }
    let author = msg.author.id.get() as i64;
    let guild = msg.guild_id.unwrap().get() as i64;
    #[derive(FromRow)]
    struct DummyMember {
        member_id: i64,
    }
    let member_id: Option<DummyMember> = sqlx::query_as(
        "SELECT member_id FROM afk_member_guild WHERE member_id = $1 AND guild_id = $2",
    )
        .bind(author)
        .bind(guild)
        .fetch_optional(&pool)
        .await?;

    Ok(member_id.is_some())
}
