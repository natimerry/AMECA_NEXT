use crate::db::database::Database;
use crate::models::guilds::GuildData;
use serenity::all::{
    Cache, ChannelId, ChannelType, Context, CreateInteractionResponse,
    CreateInteractionResponseMessage, EventHandler, GatewayIntents, GuildChannel, GuildId,
    Interaction, Message, MessageId, MessagePagination, Ready, Settings,
};
use serenity::Client;
use std::any::Any;
use std::{env, thread};
use tokio::runtime::Handle;
use tracing::{debug, error, info, trace};
mod automod;
mod commands;
pub(crate) struct AMECA {
    db: crate::db::database::Database,
    test: bool,
}

#[serenity::async_trait]
impl EventHandler for AMECA {
    async fn message_delete(
        &self,
        ctx: Context,
        channel_id: ChannelId,
        deleted_message_id: MessageId,
        guild_id: Option<GuildId>,
    ) {
        let msg = &ctx
            .cache
            .message(channel_id, deleted_message_id)
            .unwrap()
            .content;
        debug!("{:?}", msg);
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        info!("{} is connected!", ready.user.name);

        if self.test {
            let guild_token = std::env::var("GUILD_ID").unwrap().parse::<u64>().unwrap();
            let guild_id = GuildId::from(guild_token);
            let commands = guild_id
                .set_commands(&ctx.http, vec![commands::hello::register()])
                .await;

            debug!(
                "Registering the following commands: {commands:#?} for test guild: {guild_id:#?}"
            );
            info!("Starting warm-up cache.");
            self.warm_up_cache(ctx.clone(), guild_id.clone()).await;
            return;
        }

        let guild = crate::db::database::Database::joined_guild(
            &self.db,
            0,
            GuildId::from(785898278083362857),
        )
        .await;
        let guilds = Database::get_all_guilds(&self.db).await;
        match guilds {
            Some(guilds) => {
                for guild in guilds {
                    let guild_id = GuildId::from(guild.guild_id.parse::<u64>().unwrap());
                    let commands = guild_id
                        .set_commands(&ctx.http, vec![commands::hello::register()])
                        .await;

                    debug!(
                        "Registering the following commands: {commands:#?} for guild: {guild:#?}"
                    );
                    info!("Starting warm-up cache.");
                    self.warm_up_cache(ctx.clone(), guild_id.clone()).await;
                }
            }
            None => {
                error!("Bot doesnt seem to be any guild, falling back to testing env variable");
                panic!("Exit bot");
            }
        }
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::Command(command) = interaction {
            trace!(
                "{}",
                format!(
                    "Received interaction : {command:#?} from {}",
                    command.user.name
                )
            );
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
    async fn store_messages_in_db(&self, messages: Vec<Message>) {
        // since this is going to get called through a different thread, we will be using a different
        // parallel thread to store messages on the DB
        // concurrency should be handled by SurrealDB without us having to manage Mutexes

        todo!();
    }
    async fn warm_up_cache(&self, ctx: Context, guild_id: GuildId) {
        info!("Creating new concurrency thread!");
        let t = tokio::spawn(async move {
            let channels = ctx
                .http
                .get_channels(guild_id)
                .await
                .expect("CANT GET NO CHANNELS OFF GUILD IMMA KMS");
            info!("Creating new database connection to store messages!");
            // let db = crate::db::database::Database::init(env::var("SURREAL_ADDR").unwrap()).await.unwrap();
            for channel in &channels {
                if channel.kind == ChannelType::Text {
                    let last_msg = channel.last_message_id;
                    let messages = ctx
                        .http
                        .get_messages(
                            channel.id,
                            Some(MessagePagination::Before(last_msg.unwrap())),
                            Some(100),
                        )
                        .await;
                    match messages {
                        Ok(vector) => {
                            debug!(
                                "Received {} messages before {{{}}} in channel {}!",
                                vector.len(),
                                last_msg.unwrap(),
                                channel.name
                            );
                        }
                        Err(e) => {
                            error!("Error in receiving messages: {e:#?}");
                        }
                    }
                }
            }
            return channels;
        });

        /* TODO: Is it better to retrieve the last message stored in the DB and then fetch 100 messages post
        or better to focus on more recent messages */
    }
    async fn new(test: bool) -> Self {
        let mut database = Database::init(std::env::var("SURREAL_ADDR").unwrap()).await;

        return match database {
            Ok(mut db) => {
                let schema = std::fs::read_to_string("migrations/schema.surql")
                    .expect("Couldnt read string");

                // debug!("{}",schema);
                if let Err(why) = db.set_schema(schema).await {
                    error!("Error setting database schema! {:#?}", why);
                    panic!()
                }
                return AMECA { db, test };
            }
            Err(error) => {
                error!("Error setting up database: {:#?}", error);
                panic!()
            }
        };
    }

    pub async fn start_shard(token: &str, test: bool) {
        let mut settings = Settings::default();
        settings.max_messages = 10000;
        // let cache = Cache::new_with_settings(settings);
        let mut client = Client::builder(
            token,
            GatewayIntents::privileged() | GatewayIntents::GUILD_MESSAGES,
        )
        .event_handler(Self::new(test).await)
        .cache_settings(settings)
        .await;
        // TODO: setup database migrations
        match client {
            Ok(mut client) => {
                if let Err(why) = client.start().await {
                    error!("Client error: {why:?}");
                }
            }
            Err(err) => {
                error!("{}", format!("Error in creating bot: {:?}", err));
                panic!();
            }
        }
    }
}
