use std::fs::read_to_string;

use clap::builder::Str;
use serenity::all::{
    CacheHttp, CommandOptionType, Context, CreateCommandOption, CreateEmbed, CreateMessage, GuildId, ResolvedValue
};
use serenity::builder::CreateCommand;
use serenity::model::application::ResolvedOption;
use serenity::model::user;

use crate::db::database::Database;
use crate::models::messages::Message;

pub async fn run<'a>(
    ctx: &Context,
    _options: &[ResolvedOption<'a>],
    db: &Database,
    guild_id: GuildId,
) -> Result<CreateEmbed, serenity::Error> {
    if let Some(ResolvedOption {
        value: ResolvedValue::User(user, _),
        ..
    }) = _options.first()
    {
        let mut res = db
            .client
            .query(read_to_string("migrations/test_command.surql").unwrap())
            .bind(("user", user.name.clone()))
            .bind(("guild_id",guild_id))
            .await
            .unwrap();
        let channels: Option<Vec<String>> = dbg!(res).take(0).unwrap();

        let to_ret = channels.unwrap().join("\n");

        let embed = CreateEmbed::new()
            .title(format!("{} has messaged in these channels", user.name))
            .field("channels", to_ret, false);

        Ok(embed)
    } else {
        unreachable!()
    }
}

pub fn register() -> CreateCommand {
    CreateCommand::new("test_command")
        .description("Test ameca database integration")
        .add_option(
            CreateCommandOption::new(CommandOptionType::User, "user", "user msg to query")
                .required(true),
        )
}
