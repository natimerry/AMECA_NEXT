use serenity::builder::CreateCommand;
use serenity::model::application::ResolvedOption;


pub fn run(_options: &[ResolvedOption]) -> String {
    "Hello from AMECA!!".to_string()
}

pub fn register() -> CreateCommand{
    CreateCommand::new("helloameca").description("Say hello to AMECA")
}
