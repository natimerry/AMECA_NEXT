use crate::bot::automod::{cache_regex, cache_roles};
use crate::models::channel::{Channel, ChannelData};
use crate::models::role::{Role as DbRole, RoleData};
use crate::{BoxResult, Context, DynError};
use log::{debug, error, trace};
use poise::futures_util::Stream;
use poise::serenity_prelude as serenity;
use poise::serenity_prelude::{
    content_safe, futures, CacheHttp, Color, CreateEmbed, CreateEmbedAuthor, Emoji, ReactionType,
    Role,
};

use sqlx::error::DatabaseError;
use sqlx::Error as SqlxError;
use std::str::FromStr;
use tracing::info;

async fn autocomplete_emojis<'a>(
    ctx: Context<'_>,
    partial: &'a str,
) -> impl Stream<Item=String> + 'a {
    use poise::futures_util::StreamExt;
    let guild_id = ctx.guild_id().expect("Cannot get guild ID");

    let role_names = sqlx::query_as::<_, crate::models::role::Role>("SELECT * from reaction_role WHERE guild_id = $1")
        .bind(guild_id.get() as i64)
        .fetch_all(&ctx.data().db)
        .await
        .expect("Error getting autocomplete channels")
        .iter()
        .map(|channel| channel.name.clone())
        .collect::<Vec<_>>();

    let role_binding = role_names.clone();
    let x = futures::stream::iter(role_binding.to_owned())
        .filter(move |name| futures::future::ready(name.starts_with(partial)))
        .map(|name| name.clone().to_string());
    x
}

#[poise::command(
    slash_command,
    guild_only = true,
    required_permissions = "MANAGE_ROLES"
)]
pub async fn stop_watching_for_reactions(
    ctx: Context<'_>,
    #[description = "Name of the watch entry"]
    #[autocomplete = "autocomplete_emojis"]
    name: String,
) -> BoxResult<()> {
    let guild = ctx.guild().expect("Cannot get guild ID").id.get() as i64;
    log::info!("Removing regex entry `{}` from database ", name);
    sqlx::query!(
        "DELETE FROM reaction_role WHERE name = $1 AND guild_id=$2",
        name,
        guild
    )
        .execute(&ctx.data().db)
        .await?;
    cache_roles(&ctx.data()).await?;
    let embed = CreateEmbed::new()
        .author(CreateEmbedAuthor::new("AMECA_NEXT").url("https://github.com/AMECA_NEXT"))
        .color(Color::from_rgb(0, 244, 0))
        .title(format!("Deleting watch entry `{}`", &name));
    Channel::send_to_logging_channel(
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
    guild_only = "true",
    required_permissions = "MANAGE_ROLES"
)]
pub async fn set_role_assignment(
    ctx: Context<'_>,
    msg_id: serenity::MessageId,
    emoji: String,
    role: Role,
    name: String,
) -> BoxResult<()> {

    trace!("Received role to setup relation with {:?}", role);
    debug!("{} {}", msg_id, emoji);
    let channel = ctx.channel_id();
    match ctx
        .http()
        .create_reaction(channel, msg_id, &ReactionType::from_str(&emoji)?.into())
        .await
    {
        Ok(_) => {}
        Err(_) => {
            ctx.say("Invalid emoji provided! Use a emoji in the guild or a default emoji please!")
                .await?;
            return Ok(());
        }
    }

    match DbRole::new_reaction_role(
        &ctx.data(),
        msg_id,
        role.id,
        ctx.guild_id().unwrap(),
        name,
        emoji,
    )
        .await
    {
        Ok(_) => {
            let msg_url = format!(
                "https://discord.com/channels/{}/{}/{}",
                ctx.guild_id().unwrap().get(),
                channel.get(),
                msg_id.get()
            );
            ctx.say(format!("Watching {msg_url} for reactions!"))
                .await?;
        }
        Err(e) => {
            error!("Error in setting role-msg relation {e:#?}");
            ctx.say(("Error in setting role-msg relation. Check logs for more detail"))
                .await?;
            let embed = CreateEmbed::new()
                .author(CreateEmbedAuthor::new("AMECA_NEXT").url("https://github.com/AMECA_NEXT"))
                .color(Color::from_rgb(220, 0, 220))
                .title("Failed to parse regex")
                .field("Error", &format!("```\n{:#?}```", e), false);
            Channel::send_to_logging_channel(embed, &ctx, &ctx.data().db, ctx.guild_id().unwrap())
                .await?;
        }
    }

    Ok(())
}
