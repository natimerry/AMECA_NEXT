use std::env;
use serenity::all::{Cache, ChannelId, ChannelType, Context, CreateInteractionResponse, CreateInteractionResponseMessage, EventHandler, GatewayIntents, GuildId, Interaction, Message, MessageId, MessagePagination, Ready, Settings};
use serenity::Client;
use tracing::{debug, error, info, trace};
use crate::db::database::Database;
use crate::models::guilds::GuildData;
mod automod;
mod commands;
pub(crate) struct AMECA{
    db: crate::db::database::Database,
}

#[serenity::async_trait]
impl EventHandler for AMECA {
    async fn message_delete(&self, ctx: Context, channel_id: ChannelId, deleted_message_id: MessageId, guild_id: Option<GuildId>) {
        let msg = &ctx.cache.message(channel_id,deleted_message_id).unwrap().content;
        debug!("{:?}",msg);

    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        info!("{} is connected!", ready.user.name);


        // let guild_token =  std::env::var("GUILD_ID")
        //     .expect("Expected GUILD_ID in environment")
        //     .parse()
        //     .expect("GUILD_ID must be an integer");
        //
        // debug!(guild_token);
        //
        // let guild_id = GuildId::new(
        //     guild_token
        // );

        let guilds = Database::get_all_guilds(&self.db).await;
        let duh= Database::joined_guild(&self.db,0,GuildId::from(env::var("GUILD_ID").unwrap().parse::<u64>().unwrap())).await;
        match guilds{
            Some(guilds) => {
                for guild in guilds{
                    let guild_id = GuildId::new(guild.guild_id);
                    let commands = guild_id
                        .set_commands(&ctx.http, vec![
                            commands::hello::register(),
                        ])
                        .await;

                    debug!("Registering the following commands: {commands:#?} for guild: {guild:#?}");
                    info!("Starting warm-up cache.");
                }
            },
            None => {
                error!("Bot doesnt seem to be any guild, falling back to testing env variable");
            }
        }

    }


    async fn interaction_create(&self, ctx: Context,interaction: Interaction){
        if let Interaction::Command(command) = interaction {
            trace!("{}",format!("Received interaction : {command:#?} from {}",command.user.name));
            let content = match command.data.name.as_str() {
                "helloameca" => Some(commands::hello::run(&command.data.options())),
                _ => Some("ACHIEVMENT: how did we get here?".to_string()),
            };

            if let Some(content) = content {
                let data = CreateInteractionResponseMessage::new().content(content);
                let builder = CreateInteractionResponse::Message(data);
                if let Err(why) = command.create_response(&ctx.http, builder).await {
                    error!("Cannot respond to slash command: {why}");
                }
            }
        }
    }

}
impl AMECA {

    async fn new() -> Self{
        let mut database = Database::init(std::env::var("SURREAL_ADDR").unwrap()).await;

        return match database{
            Ok(mut db) => {
                let schema = std::fs::read_to_string("migrations/schema.surql")
                    .expect("Couldnt read string");

                // debug!("{}",schema);
                if let Err(why) = db.set_schema(schema).await{
                    error!("Error setting database schema! {:#?}",why);
                    panic!()
                }
                return AMECA{
                    db
                }

            },
            Err(error) => {
                error!("Error setting up database: {:#?}",error);
                panic!()
            }
        }

    }

    pub async fn start_shard(token: &str){
        let mut settings = Settings::default();
        settings.max_messages=10000;
        // let cache = Cache::new_with_settings(settings);
        let mut client = Client::builder(token,GatewayIntents::privileged()
                                                                    | GatewayIntents::GUILD_MESSAGES)
                                        .event_handler(Self::new().await)
                                        .cache_settings(settings)
                                        .await;
        // TODO: setup database migrations
        match client{
            Ok(mut client) => {
                if let Err(why) = client.start().await {
                    error!("Client error: {why:?}");
                }
            },
            Err(err) =>{
                error!("{}",format!("Error in creating bot: {:?}",err));
                panic!();
            }
        }
    }
}

