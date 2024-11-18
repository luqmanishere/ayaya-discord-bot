//! Contains commands reserved for the bot's owner: ie me.

use crate::{error::BotError, Commands, Context};

pub fn owner_commands() -> Commands {
    vec![command_log_raw()]
}

/// Prints command logs raw. Owner only.
#[poise::command(
    slash_command,
    prefix_command,
    owners_only,
    ephemeral,
    hide_in_help,
    category = "Owner Commands"
)]
pub async fn command_log_raw(ctx: Context<'_>) -> Result<(), BotError> {
    ctx.defer().await?;

    let data_manager = ctx.data().data_manager.clone();

    let logs = data_manager.find5_command_log().await?;

    ctx.reply(format!("```\n{:#?}\n```", logs)).await?;
    Ok(())
}
