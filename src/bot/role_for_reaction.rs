use crate::bot::automod::cache_roles;
use crate::models::channel::{Channel, ChannelData};
use crate::models::role::{Role as DbRole, RoleData};
use crate::{BoxResult, Context};
use poise::futures_util::Stream;
use poise::serenity_prelude as serenity;
use poise::serenity_prelude::{
    futures, ChannelId, Color, CreateEmbed, CreateEmbedAuthor, ReactionType, Role,
};
use tracing::log::{debug, error, trace};

use tracing::info;

async fn autocomplete_roles<'a>(
    ctx: Context<'_>,
    partial: &'a str,
) -> impl Stream<Item = String> + 'a {
    use poise::futures_util::StreamExt;
    let guild_id = ctx.guild_id().expect("Cannot get guild ID");

    let role_names = sqlx::query_as::<_, crate::models::role::Role>(
        "SELECT * from reaction_role WHERE guild_id = $1",
    )
    .bind(guild_id.get() as i64)
    .fetch_all(&ctx.data().db)
    .await
    .expect("Error getting autocomplete channels")
    .iter()
    .map(|channel| channel.name.clone())
    .collect::<Vec<_>>();

    let role_binding = role_names.clone();
    futures::stream::iter(role_binding.to_owned())
        .filter(move |name| futures::future::ready(name.starts_with(partial)))
        .map(|name| name.clone().to_string())
}

#[poise::command(
    prefix_command,
    slash_command,
    guild_only = true,
    subcommands("stop", "add", "stopbyid"),
    subcommand_required,
    category = "administration"
)]
// Omit 'ctx' parameter here. It is not needed, because this function will never be called.
// TODO: Add a way to remove 'ctx' parameter, when `subcommand_required` is set
pub async fn reactionrole(_: Context<'_>) -> BoxResult<()> {
    // This will never be called, because `subcommand_required` parameter is set
    Ok(())
}

#[poise::command(
    slash_command,
    guild_only = true,
    required_permissions = "MANAGE_ROLES",
    required_bot_permissions = "MANAGE_ROLES",
    ephemeral = "true",
    category = "administration",
    description_localized("en-US", "Stop watching all reactions for a particular message")
)]
pub async fn stopbyid(
    ctx: Context<'_>,
    #[description = "ID of the message that you want to stop watching reactions for"]
    msg_id: serenity::MessageId,
) -> BoxResult<()> {
    let guild_id = ctx.guild_id().expect("Cannot get guild ID").get() as i64; // panic should be unreachable
    let msg_id_i64 = msg_id.get() as i64;
    ctx.defer_ephemeral().await?;
    info!(msg_id_i64, "Removing role entry for ");

    sqlx::query!(
        "DELETE FROM reaction_role WHERE guild_id = $1 AND msg_id = $2",
        guild_id,
        msg_id_i64
    )
    .execute(&ctx.data().db)
    .await?;

    cache_roles(&ctx.data()).await?;

    ctx.say("Stopped watching reactions!").await?;

    Ok(())
}

#[poise::command(
    slash_command,
    guild_only = true,
    required_permissions = "MANAGE_ROLES",
    required_bot_permissions = "MANAGE_ROLES",
    ephemeral = "true",
    category = "administration"
)]
pub async fn stop(
    ctx: Context<'_>,
    #[description = "Name of the watch entry"]
    #[autocomplete = "autocomplete_roles"]
    name: String,
) -> BoxResult<()> {
    let guild = ctx.guild().expect("Cannot get guild ID").id.get() as i64;
    info!("Removing role entry `{}` from database ", name);
    sqlx::query!(
        "DELETE FROM reaction_role WHERE name = $1 AND guild_id=$2",
        name,
        guild
    )
    .execute(&ctx.data().db)
    .await?;
    cache_roles(ctx.data()).await?;
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
    required_permissions = "MANAGE_ROLES",
    required_bot_permissions = "MANAGE_ROLES",
    ephemeral = "true",
    category = "administration"
)]
pub async fn add(
    ctx: Context<'_>,
    msg_id: serenity::MessageId,
    emoji: String,
    role: Role,
    name: String,
) -> BoxResult<()> {
    ctx.defer_ephemeral().await?;

    trace!("Received role to setup relation with {:?}", role);
    debug!("{} {}", msg_id, emoji);
    struct ChannelOfMsg {
        channel_id: i64,
    }

    // get the database entry of the channel message was sent in
    let channel = sqlx::query_as!(
        ChannelOfMsg,
        "SELECT channel_id as \"channel_id!\" FROM message WHERE msg_id = $1",
        msg_id.get() as i64
    )
    .fetch_one(&ctx.data().db)
    .await?;

    // we need to get the message by msgid (that is provided) and the channelid that is provided to us
    let channel = ChannelId::from(channel.channel_id as u64);
    let msg = ctx.http().get_message(channel, msg_id).await;
    if let Err(e) = msg {
        error!("Error getting message: {}", e);
        return Err(Box::new(e));
    }
    let msg = msg.unwrap();
    let reaction = msg.react(ctx, ReactionType::Unicode(emoji.clone())).await;
    match reaction {
        Ok(_) => {
            // we set reaction to a message
            ctx.say("Set reaction  to msg!").await?;
            match DbRole::new_reaction_role(
                ctx.data(),
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
                        "https://discord.com/channels/{}/{}/{}", // messsage link builder
                        ctx.guild_id().unwrap().get(),
                        channel.get(),
                        msg_id.get()
                    );
                    ctx.say(format!("Watching {msg_url} for reactions!"))
                        .await?;
                }
                Err(e) => {
                    error!("Error in setting role-msg relation {e:#?}");
                    ctx.say("Error in setting role-msg relation. Check logs for more detail")
                        .await?;
                    let embed = CreateEmbed::new()
                        .author(
                            CreateEmbedAuthor::new("AMECA_NEXT")
                                .url("https://github.com/AMECA_NEXT"),
                        )
                        .color(Color::from_rgb(220, 0, 220))
                        .title("Failed to save to database")
                        .field("Error", format!("```\n{:#?}```", e), false);
                    Channel::send_to_logging_channel(
                        embed,
                        &ctx,
                        &ctx.data().db,
                        ctx.guild_id().unwrap(),
                    )
                    .await?;
                }
            }
        }
        Err(error) => {
            ctx.say("Something went wrong reacting to the message! Check the emoji / bot perms")
                .await?;

            let msg_id = msg.id.get();
            let channel = channel.get();
            let error = error.to_string();
            tracing::error!(
                msg_id,
                channel,
                error,
                "Error in reactiong to watched message"
            );
            ctx.say(error).await?;
        }
    }

    Ok(())
}
