use std::cmp::{max, min};

use poise::{
    serenity_prelude::{ Color, CreateEmbed, CreateEmbedAuthor, User, UserId},
    CreateReply,
};
use sha2::{Digest, Sha256};

use crate::{BoxResult, Context};

#[poise::command(slash_command, prefix_command, guild_only = true)]
pub async fn ship<'a>(
    ctx: Context<'a>,
    #[description = "First user to ship (optional)"] user1: Option<User>,
    #[description = "User to ship with (optional)"] user2: Option<User>,
) -> BoxResult<()> {

    let (target_user_id, user_id) = if user1.is_some() && user2.is_none() {
        // If user1 is present and user2 is empty, ship the author with user1
        (user1.unwrap().id.get(), ctx.author().id.get())
    } else if user2.is_some() && user1.is_none() {
        // If user2 is present and user1 is empty, ship the author with user2
        (user2.unwrap().id.get(), ctx.author().id.get())
    } 
    else if user2.is_some() && user1.is_some(){
        (user1.unwrap().id.get(), user2.unwrap().id.get())
    }
    else {
        // If both user1 and user2 are empty, ship the author with the bot user
        let bot_user_id = std::env::var("BOT_USER")
            .expect("Unable to get bot user")
            .parse()
            .expect("Invalid BOT_USER format");
        (bot_user_id, ctx.author().id.get())
    };

    let combined_ids = format!(
        "{}{}",
        min(target_user_id, user_id),
        max(target_user_id, user_id)
    );

    let mut hasher = Sha256::new();
    hasher.update(combined_ids.as_bytes());
    let hash_result = hasher.finalize();

    // Convert the hash to an integer and get a value between 0 and 100
    let mut score = u64::from_be_bytes(hash_result[0..8].try_into().unwrap()) % 101;
    if target_user_id ^ user_id == 0x2bae94cd080001d {
        score = 100;
    }
    const RESPONSES: [&str; 5] = [
        "It's a match made in heaven! 💖",     // 80-100%
        "You two would make a great team! 😊", // 60-79%
        "There's some potential here. 🤔",     // 40-59%
        "It might be a rocky road... 😅",      // 20-39%
        "Not looking too good... 💀",          // 0-19%
    ];

    const SUB_RESPONSES: [&str; 5] = [
        "Get married.",
        "Y'all have to be bestfriends",
        "Eh... maybe?",
        "Even oil and water get along better than you two...",
        "This is a disaster waiting to happen...",
    ];

    let response = match score {
        80..=100 => RESPONSES[0],
        60..=79 => RESPONSES[1],
        40..=59 => RESPONSES[2],
        20..=39 => RESPONSES[3],
        _ => RESPONSES[4],
    };

    let sub_response = match score {
        80..=100 => SUB_RESPONSES[0],
        60..=79 => SUB_RESPONSES[1],
        40..=59 => SUB_RESPONSES[2],
        20..=39 => SUB_RESPONSES[3],
        _ => SUB_RESPONSES[4],
    };

    let user_name1 = ctx.http().get_user(UserId::from(user_id)).await?.name;
    let user_name2 = ctx
        .http()
        .get_user(UserId::from(target_user_id))
        .await?
        .name;
    let embed = CreateEmbed::new()
        .author(CreateEmbedAuthor::new("AMECA").url("https://github.com/natimerry/AMECA_NEXT"))
        .title(format!(
            "{user_name1} is {}% compatible with {user_name2}",
            score as u8,
        ))
        .field(response, sub_response, false)
        .color(Color::from_rgb(0xf4, 0xc2, 0xc2));

    ctx.send(CreateReply::default().embed(embed)).await?;
    Ok(())
}
