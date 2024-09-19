use crate::db::database::Database;
use crate::models::channels::{Channel, ChannelData};
use crate::models::guilds::GuildData;
use crate::models::messages::MessageData;
use crate::models::users::*;
use serenity::all::standard::buckets::RateLimitInfo;
use serenity::all::{
    ChannelId, ChannelType, Context, CreateCommand, CreateInteractionResponse,
    CreateInteractionResponseMessage, EventHandler, GatewayIntents, Guild, GuildChannel, GuildId,
    Interaction, Message, MessageId, MessagePagination, RatelimitInfo, Ready, Settings, User,
};
use serenity::model::guild;
use serenity::Client;
use std::env;
use tokio::task::JoinHandle;
use tracing::{debug, error, info, span, trace, Level};
mod automod;
mod commands;
use std::cell::LazyCell;

pub(crate) struct AMECA {
    db: crate::db::database::Database,
    test: bool,
}

const GUILD_COMMANDS: LazyCell<Vec<CreateCommand>> = LazyCell::new(|| {
    info!("Lazilly initialising commands to register");
    vec![commands::test_command::register()]
});

#[serenity::async_trait]
impl EventHandler for AMECA {
    // offloadable events
    async fn guild_create(&self, ctx: Context, guild: Guild, _is_new: Option<bool>) {
        let guildid = guild.id;
        info!("{}", format!("Invited to a new server!! {}", guildid));
        Self::set_commands(&ctx, GUILD_COMMANDS.to_vec(), guildid).await;

        Database::joined_guild(
            &self.db,
            AMECA::get_members(&ctx, guildid).await.len() as u64,
            guildid,
        )
        .await;
    }
    async fn message(&self, ctx: Context, new_message: Message) {
        let _ = self.on_message(ctx, new_message).await;
    }

    async fn message_delete(
        &self,
        ctx: Context,
        channel_id: ChannelId,
        deleted_message_id: MessageId,
        _guild_id: Option<GuildId>,
    ) {
        let msg = self
            .get_msg_from_cache(ctx, channel_id, deleted_message_id)
            .await;
        match msg {
            Some(msg) => {
                debug!("{:#?}", msg);
            }
            None => {
                let msg = Database::fetch_msg(&self.db, deleted_message_id.get())
                    .await
                    .unwrap_or(crate::models::messages::Message {
                        message_id: 0,
                        time: "placeholder".to_string(),
                        content: "placeholder".to_string(),
                    });

                debug!("{:?}", msg);
            }
        }
        // TODO: MOVE ALL THIS INTO AUTOMOD.RS
        // TODO: SETUP DELETION RELATIONS IN SURREALDB
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        let span = span!(Level::DEBUG, "on_ready");
        let _enter = span.enter();
        info!("{} is connected!", ready.user.name);

        if self.test {
            let guild_token = std::env::var("GUILD_ID").unwrap().parse::<u64>().unwrap();
            let guild_id = GuildId::from(guild_token);
            debug!("Registering commands");
            Self::set_commands(&ctx, GUILD_COMMANDS.to_vec(), guild_id).await;
            return;
        }

        let fuck_me = ctx.http.get_guilds(None, None).await;

        match fuck_me {
            Ok(guilds) => {
                for guild in guilds {
                    let guild_id = guild.id;
                    let ctx_binding = ctx.clone(); // create lifetime 'b that lives long enough for the future to be awaited
                    tokio::spawn(async move {
                        // we cannot move ctx inside this scope as this moves ownerwship of 'ctx to this future, which 'ctx after it has been awaited
                        // why the fuck does ctx not implement the Copy trait
                        debug!("Registering commands from CTX");
                        Self::set_commands(&ctx_binding, GUILD_COMMANDS.to_vec(), guild_id).await;
                    });

                    info!("Starting warm-up cache.");
                    let join_handle = AMECA::warm_up_cache(ctx.clone(), guild_id.clone()).await; // create new ctx lifetime 'c since 'c gets freed after the loop iteration
                                                                                                 // 'c needs to live till the future is awaited which may be after loop iteration
                    join_handle.await.expect("Failed to join warm up threads");

                    info!("Finished warming up cache!");
                }
            }
            Err(_) => {
                error!("Bot doesnt seem to be any guild, falling back to testing env variable");
                panic!("Exit bot");
            }
        }
        // lifetime of 'ctx is ended
    }
    async fn ratelimit(&self, data: RatelimitInfo) {
        info!("AMECA has been ratelimited.");
        info!("{:?}", data);
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
            let response = match command.data.name.as_str() {
                "test_command" => {
                    Some(commands::test_command::run(&ctx, &command.data.options(), &self.db,command.guild_id.unwrap()).await)
                }
                _ => None,
            };

            if let Some(Ok(embed)) = response {
                let data = CreateInteractionResponseMessage::new().embed(embed);
                let builder = CreateInteractionResponse::Message(data);
                if let Err(why) = command.create_response(&ctx.http, builder).await {
                    error!("Cannot respond to slash command: {why}");
                }
            }
        }
    }
}

enum DataType<'a, 'b, 'c> {
    Channel(&'a Vec<GuildChannel>, &'a GuildId),
    User(&'b Vec<User>),
    Message(&'c Vec<Message>, GuildChannel),
}

impl AMECA {
    async fn store_in_db<'l0, 'l1, 'l2>(data: DataType<'l0, 'l1, 'l2>) {
        let new_db = Database::init(env::var("SURREAL_ADDR").unwrap()).await;

        match new_db {
            Ok(_) => {
                info!("Established concurrent connection with surrealdb!");
            }
            Err(e) => {
                error!("Error in establishing concurrent connection {e:#?}");
                panic!()
            }
        };
        let new_db = new_db.unwrap();

        match data {
            DataType::Channel(channel_vec, guildid) => {
                for channel in channel_vec {
                    Database::new_channel(&new_db, channel.clone(), *guildid).await;
                }
            }
            DataType::User(user_vec) => {
                for user in user_vec {
                    Database::new_user(&new_db, user.clone()).await;
                }
            }
            DataType::Message(message_vec, channel) => {
                for message in message_vec {
                    Database::new_message(&new_db, message.clone(), channel.clone()).await;
                }
            }
        }
    }

    async fn get_members(ctx: &Context, guild_id: GuildId) -> Vec<User> {
        let members = ctx
            .http
            .get_guild_members(guild_id, None, None)
            .await
            .expect("Unable to get all members from guild");

        members
            .iter()
            .map(|member| member.user.clone())
            .collect::<Vec<User>>()
    }

    async fn get_channels(ctx: &Context, guild_id: GuildId) -> Vec<GuildChannel> {
        ctx.http
            .get_channels(guild_id)
            .await
            .expect("CANT GET NO CHANNELS OFF GUILD IMMA KMS")
    }

    async fn warm_up_cache(ctx: Context, guild_id: GuildId) -> JoinHandle<()> {
        info!("Creating new concurrency thread!");
        let t = tokio::spawn(async move {
            let channels = AMECA::get_channels(&ctx, guild_id).await;
            AMECA::store_in_db(DataType::Channel(&channels, &guild_id)).await;
            let members = AMECA::get_members(&ctx, guild_id).await;
            AMECA::store_in_db(DataType::User(&members)).await;

            for channel in &channels {
                if channel.kind == ChannelType::Text {
                    info!("Checking iterating over channel: {}", channel.name);
                    let last_msg = channel.last_message_id;
                    if let Some(last_msg) = last_msg {
                        let messages = ctx
                            .http
                            .get_messages(
                                channel.id,
                                Some(MessagePagination::Before(last_msg)),
                                Some(100),
                            )
                            .await;
                        match messages {
                            Ok(vector) => {
                                debug!(
                                    "Received {} messages before {{{}}} in channel {}!",
                                    vector.len(),
                                    last_msg,
                                    channel.name
                                );
                                let binding = channel.clone();
                                AMECA::store_in_db(DataType::Message(&vector, binding)).await;
                            }
                            Err(e) => {
                                error!("Error in receiving messages: {e:#?}");
                            }
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
        let database = Database::init(std::env::var("SURREAL_ADDR").unwrap()).await;
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
            GatewayIntents::privileged()
                | GatewayIntents::GUILD_MESSAGES
                | GatewayIntents::GUILDS
                | GatewayIntents::all(),
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
        match commands {
            Ok(commands) => {
                debug!(
                    "Registering the following commands: {commands:#?} for guild: {:#?}",
                    guild_id.get()
                );
            }
            Err(e) => {
                error!("{:?}", e);
            }
        }
    }
}
