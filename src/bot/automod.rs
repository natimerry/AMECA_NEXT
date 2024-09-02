use crate::bot::AMECA;
use crate::db::database::Database;
use crate::models::messages::MessageData;
use log::trace;
use serenity::all::{ChannelId, Context, Message, MessageId};

impl AMECA {
    pub async fn on_message(
        &self,
        ctx: Context,
        new_message: Message,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // AMECA::store_messages_in_db(vec![new_message]).await;

        let msg = new_message.channel(&ctx.http).await?;
        match msg.guild() {
            Some(channel) => {
                Database::new_message(&self.db, new_message, channel).await;
            }
            None => trace!("Message isnt in a guildchannel, ignoring"),
        }
        Ok(())
    }

    pub async fn get_msg_from_cache(
        &self,
        ctx: Context,
        channel_id: ChannelId,
        deleted_message_id: MessageId,
    ) -> Option<crate::models::messages::Message> {
        let new_ctx = ctx.clone();
        let msg = new_ctx
            .cache
            .message(channel_id.clone(), deleted_message_id.clone());

        return match msg {
            Some(message) => {
                let content = &message.content;
                Some(crate::models::messages::Message {
                    time: (&message.timestamp.to_string()).parse().unwrap(),
                    content: content.to_string(),
                })
            }
            None => None,
        };
    }
}
