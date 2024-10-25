
use std::ops::Deref;

use poise::serenity_prelude::{self as serenity, Reaction, RoleId, UserId};
use serenity::Context;
use tracing::{debug, info, trace};

use crate::{bot::{automod::cache_roles, AMECA}, models::role::Role as DbRole, BoxResult};


async fn is_reaction_watched(ctx: &Context,data:&AMECA,reaction:&Reaction) -> BoxResult<Option<DbRole>> {
    if data.watch_msgs.is_empty() {
        info!("Caching role reactions I have to react to!");
        cache_roles(&data).await?;
    }
    trace!("{:#?}", reaction);
    let guild = reaction.guild_id;
    if let None = guild {
        debug!(
            "Reaction {} is not in an guild",
            reaction.channel_id.name(&ctx).await?
        );
        return Ok(None);
    }
    if reaction.message_author_id
        == Some(UserId::new(
            std::env::var("BOT_USER").unwrap().parse::<u64>().unwrap(),
        ))
    {
        return Ok(None);
    }
    let guild = guild.unwrap().get() as i64;
    let guild_watchlist = data.watch_msgs.get(&guild);
    match guild_watchlist{
        Some(guild_watchlist) => {
            let guild_watchlist = guild_watchlist.deref();
            for role_for_reaction in guild_watchlist {
                if reaction.emoji.to_string() == role_for_reaction.emoji.to_string() {
                   return Ok(Some(role_for_reaction.clone()));
                }
            }
        }
        None => return Ok(None),
    }
    return Ok(None);
}


pub async fn reaction_add(ctx: &Context,data: &AMECA,add_reaction: &Reaction) -> BoxResult<()>{
    if let Some(role_for_reaction) = is_reaction_watched(ctx, data, add_reaction).await?{
        info!(
            "Updating roles for {} for reacting to watched msg!",
            &add_reaction.user_id.unwrap()
        );
        let x = ctx
            .http
            .add_member_role(
                add_reaction.guild_id.unwrap(),
                add_reaction.user_id.unwrap(),
                RoleId::new(role_for_reaction.roles_id as u64),
                Some(&format!(
                    "Assigning role for reaction to message. (WatchID: {})",
                    role_for_reaction.roles_id
                )),
            )
            .await;
        match x {
            Ok(_) => {}
            Err(e) => {
                info!("Error assigning roles {:#?}", e);
            }
        }
    }
    return Ok(())
}

pub async fn reaction_delete(ctx: &Context,data: &AMECA,delete_reaction: &Reaction) -> BoxResult<()>{
    if let Some(role_for_reaction) = is_reaction_watched(ctx, data, delete_reaction).await?{
        info!(
            "Updating roles for {} for reacting to watched msg!",
            &delete_reaction.user_id.unwrap()
        );
        let x = ctx
            .http
            .remove_member_role(
                delete_reaction.guild_id.unwrap(),
                delete_reaction.user_id.unwrap(),
                RoleId::new(role_for_reaction.roles_id as u64),
                Some(&format!(
                    "Removing role from user due to removing reaction. (WatchID: {})",
                    role_for_reaction.roles_id
                )),
            )
            .await;
        match x {
            Ok(_) => {}
            Err(e) => {
                info!("Error assigning roles {:#?}", e);
            }
        }
    }

    Ok(())
}