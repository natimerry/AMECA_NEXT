use serenity::all::{Context, CreateInteractionResponse, CreateInteractionResponseMessage, EventHandler, GatewayIntents, GuildId, Interaction, Message, Ready};
use serenity::Client;
use tracing::{debug, error, info, trace};
use crate::db::database::Database;

mod automod;
mod commands;
pub(crate) struct AMECA{
    db: crate::db::database::Database,
}

#[serenity::async_trait]
impl EventHandler for AMECA {
    async fn ready(&self, ctx: Context, ready: Ready) {
        info!("{} is connected!", ready.user.name);


        let guild_token =  std::env::var("GUILD_ID")
            .expect("Expected GUILD_ID in environment")
            .parse()
            .expect("GUILD_ID must be an integer");

        debug!(guild_token);

        let guild_id = GuildId::new(
            guild_token
        );


        let commands = guild_id
            .set_commands(&ctx.http, vec![
                commands::hello::register(),
            ])
            .await;

        debug!("Registering the following commands: {commands:#?}");
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
                    error!("Error settind database schema! {:#?}",why);
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
        let mut client = Client::builder(token,GatewayIntents::privileged()
                                                                    | GatewayIntents::GUILD_MESSAGES)
                                        .event_handler(Self::new().await).await;
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

