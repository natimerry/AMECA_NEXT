use crate::db::database::Database;
use crate::models::channels::ChannelData;
use crate::models::guilds::GuildData;
use crate::models::messages::MessageData;
use crate::models::users::*;
use serenity::all::{
    Cache, Channel, ChannelId, ChannelType, Context, CreateCommand, CreateInteractionResponse, CreateInteractionResponseMessage, EventHandler, GatewayIntents, GuildChannel, GuildId, Http, Interaction, Message, MessageId, MessagePagination, Ready, Settings, User
};
use serenity::Client;
use std::any::Any;
use std::{env, thread};
use surrealdb::rpc::Data;
use tokio::runtime::Handle;
use tokio::task::JoinHandle;
use tracing::{debug, error, info, span, trace, Level};
mod automod;
mod commands;
pub(crate) struct AMECA {
    db: crate::db::database::Database,
    test: bool,
}

#[serenity::async_trait]
impl EventHandler for AMECA {
    // offloadable events

    async fn message(&self, ctx: Context, new_message: Message) {
        self.on_message(ctx, new_message).await;
    }

    async fn message_delete(
        &self,
        ctx: Context,
        channel_id: ChannelId,
        deleted_message_id: MessageId,
        guild_id: Option<GuildId>,
    ) {
        let msg = self
            .get_msg_from_cache(ctx, channel_id, deleted_message_id, guild_id)
            .await;
        match msg {
            Some(msg) => {
                debug!("{:#?}", msg);
            }
            None => {
                let msg = Database::fetch_msg(&self.db, deleted_message_id.get())
                    .await
                    .unwrap_or(crate::models::messages::Message {
                        time: "placeholder".to_string(),
                        content: "placeholder".to_string(),
                    });

                debug!("{:?}", msg);
            }
        }
        // TODO: MOVE ALL THIS INTO AUTOMOD.RS
        // TODO: SETUP DELETION RELATIONS IN SURREALDB
    }

    // bot startup events
    async fn ready(&self, ctx: Context, ready: Ready) {
        let span = span!(Level::DEBUG, "on_ready");
        let _enter = span.enter();
        info!("{} is connected!", ready.user.name);

        if self.test {
            let guild_token = std::env::var("GUILD_ID").unwrap().parse::<u64>().unwrap();
            let guild_id = GuildId::from(guild_token);
            Self::set_commands(&ctx, vec![commands::hello::register()], guild_id).await;
            return;
        }
        Database::joined_guild(&self.db, 0, GuildId::from(785898278083362857)).await;
        let guilds = Database::get_all_guilds(&self.db).await;
        match guilds {
            Some(guilds) => {
                for guild in guilds {
                    let guild_id = GuildId::from(guild.guild_id.parse::<u64>().unwrap());
                    Self::set_commands(&ctx, vec![commands::hello::register()], guild_id).await;
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
    async fn store_messages_in_db(messages: Vec<Message>,channel: GuildChannel) {
        // since this is going to get called through a different thread, we will be using a different
        // parallel thread to store messages on the DB
        // concurrency should be handled by SurrealDB without us having to manage Mutexes

        let new_db = Database::init(env::var("SURREAL_ADDR").unwrap()).await;

        match new_db {
            Ok(ref some_db) => {
                info!("Established concurrent connection with surrealdb!");
            }
            Err(e) => {
                error!("Error in establishing concurrent connection {e:#?}");
                panic!()
            }
        };
        let new_db = new_db.unwrap();

        for message in messages {
            Database::new_message(&new_db, message,channel.clone()).await;
        }
    }

    async fn store_members_in_db(users: Vec<User>) {
        // since this is going to get called through a different thread, we will be using a different
        // parallel thread to store messages on the DB
        // concurrency should be handled by SurrealDB without us having to manage Mutexes

        trace!("Storing members in database");
        let new_db = Database::init(env::var("SURREAL_ADDR").unwrap()).await;

        match new_db {
            Ok(ref some_db) => {
                info!("Established concurrent connection with surrealdb!");
            }
            Err(e) => {
                error!("Error in establishing concurrent connection {e:#?}");
                panic!()
            }
        };
        let new_db = new_db.unwrap();

        for user in users {
            Database::new_user(&new_db, user).await;
        }
    }

    async fn store_channels_in_db(channels: &Vec<GuildChannel>) {
        // since this is going to get called through a different thread, we will be using a different
        // parallel thread to store messages on the DB
        // concurrency should be handled by SurrealDB without us having to manage Mutexes

        trace!("Storing members in database");
        let new_db = Database::init(env::var("SURREAL_ADDR").unwrap()).await;

        match new_db {
            Ok(ref some_db) => {
                info!("Established concurrent connection with surrealdb!");
            }
            Err(e) => {
                error!("Error in establishing concurrent connection {e:#?}");
                panic!()
            }
        };
        let new_db = new_db.unwrap();

        for channel in channels {
            Database::new_channel(&new_db, channel.clone()).await;
        }
    }


    async fn warm_up_cache(ctx: Context, guild_id: GuildId) -> JoinHandle<()> {
        info!("Creating new concurrency thread!");
        let t = tokio::spawn(async move {
            let channels = ctx
                .http
                .get_channels(guild_id)
                .await
                .expect("CANT GET NO CHANNELS OFF GUILD IMMA KMS");



            AMECA::store_channels_in_db(&channels).await;
            let members = ctx.http.get_guild_members(guild_id, None, None).await.expect("Unable to get all members from guild");

            let x = members.iter().map(|member| member.user.clone()).collect::<Vec<User>>();
            AMECA::store_members_in_db(x).await;
            for channel in &channels {
                if channel.kind == ChannelType::Text {
                    info!("Checking iterating over channel: {}", channel.name);
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

                            AMECA::store_messages_in_db(vector,channel.clone()).await;
                        }
                        Err(e) => {
                            error!("Error in receiving messages: {e:#?}");
                        }
                    }
                }
            }
        });

        return t;

        /* TODO: Is it better to retrieve the last message stored in the DB and then fetch 100 messages post
        or better to focus on more recent messages */
    }
    #[allow(unreachable_code)]
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
        let client = Client::builder(
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

    async fn set_commands(ctx: &Context, commands: Vec<CreateCommand>, guild_id: GuildId) {
        let commands = guild_id.set_commands(&ctx.http, commands).await;

        debug!(
            "Registering the following commands: {commands:#?} for guild: {:#?}",
            guild_id.get()
        );
        info!("Starting warm-up cache.");
        let join_handle = AMECA::warm_up_cache(ctx.clone(), guild_id.clone()).await;
        join_handle.await.expect("Failed to join warm up threads");

        info!("Finished warming up cache!");
    }
}
