use crate::{BoxResult, DynError};
use poise::builtins::register_globally;
use poise::serenity_prelude as serenity;
use serenity::all::{Message, Settings};
use sqlx::{PgPool, Pool, Postgres};
use tracing::{error, info};
use crate::models::messasges::MessageData;

pub struct AMECA {
    db: Pool<Postgres>,
}

impl AMECA {
    async fn event_handler(
        ctx: &serenity::Context,
        event: &serenity::FullEvent,
        _framework: poise::FrameworkContext<'_, AMECA, DynError>,
        data: &AMECA,
    ) -> BoxResult<()> {
        match event {
            serenity::FullEvent::Message { new_message } => {
                info!("New message: {} <{}>:{}", new_message.author.name,new_message.id,new_message.content);
                let channel = new_message.channel(&ctx.http).await?;
                let res = PgPool::new_message(&data.db, new_message.clone(), channel.guild().unwrap()).await;

                if let Err(e) = res{
                    error!("Unable to store message in db: {}", e);
                }
            },
                automod::on_msg(new_message.clone(), &data.db ).await?;
            }
            _ => (),
        }
        Ok(())
    }

    pub async fn stard_shard(token: String,db: Pool<Postgres>) -> BoxResult<()> {
        let mut settings = Settings::default();
        settings.max_messages = 10000;

        let framework = poise::Framework::builder()
            .setup(|ctx, _ready, _framework| {
                Box::pin(async move {
                    register_globally(ctx, &_framework.options().commands).await?;
                    Ok(AMECA { db })
                })
            })
            .options(poise::FrameworkOptions {
                commands: vec![],
                event_handler: |ctx, event, framework, data| {
                    Box::pin(AMECA::event_handler(ctx, event, framework, data))
                },
                ..Default::default()
            })
            .build();
        let intents = serenity::GatewayIntents::AUTO_MODERATION_CONFIGURATION
            | serenity::GatewayIntents::GUILD_MESSAGES
            | serenity::GatewayIntents::GUILD_MESSAGE_REACTIONS
            | serenity::GatewayIntents::AUTO_MODERATION_EXECUTION
            | serenity::GatewayIntents::privileged();

        let client = serenity::ClientBuilder::new(token, intents)
            .framework(framework)
            .cache_settings(settings)
            .await;

        client.unwrap().start().await?;

        Ok(())
    }
}
