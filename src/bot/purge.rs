use crate::{BoxResult, Context};
use poise::serenity_prelude::{ MessagePagination};

#[poise::command(
    slash_command,
    required_permissions = "MANAGE_MESSAGES",
    required_bot_permissions = "MANAGE_MESSAGES",
    guild_only = "true"
)]
pub async fn purge<'a>(
    ctx: Context<'a>,
    #[description = "Select logging channel"]
    #[min = 2]
    #[max = 100]
    number_to_purge: u32,
) -> BoxResult<()> {
    // get last x msg
    let last_msg = ctx.say("Purging channel!!").await?.message().await?.id;
    let channel = ctx
        .http()
        .get_channel(ctx.channel_id())
        .await?
        .guild()
        .unwrap();
    let msgs = ctx
        .http()
        .get_messages(
            channel.id.clone(),
            Some(MessagePagination::Before(last_msg)),
            Some(number_to_purge as u8),
        )
        .await?
        .iter()
        .map(|msg| msg.id)
        .collect::<Vec<_>>();
    channel.delete_messages(&ctx.http(), msgs).await?;
    ctx.say(format!("Deleted {number_to_purge} messages!! UwU"))
        .await?;
    Ok(())
}
