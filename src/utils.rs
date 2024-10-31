use poise::serenity_prelude::Message;

pub fn check_if_author_is_bot(msg: &Message) -> bool{
    return if msg.author.id.get() == std::env::var("BOT_USER").unwrap().parse::<u64>().unwrap() {
        true
    } else {
        false
    }
}