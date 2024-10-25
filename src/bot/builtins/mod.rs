use crate::{BoxResult, Context};


#[poise::command(slash_command, prefix_command)]
pub async fn servers(ctx: Context<'_>) -> BoxResult<()>{
    poise::builtins::servers(ctx).await?;
    Ok(())
}

#[poise::command(slash_command, prefix_command)]
pub async fn help<'a>(ctx: Context<'a>, command: Option<String>) -> BoxResult<()> {
    let configuration = poise::builtins::HelpConfiguration {
        include_description: true,
        ephemeral: true,
        ..Default::default()
    };
    poise::builtins::help(ctx, command.as_deref(), configuration).await?;
    Ok(())
}
