//! Contains commands reserved for the bot's owner: ie me.
use poise::serenity_prelude as serenity;

use crate::{error::BotError, CommandResult, Commands, Context};

pub fn owner_commands() -> Commands {
    vec![command_log_raw(), upload_cookies()]
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

#[poise::command(
    slash_command,
    prefix_command,
    owners_only,
    ephemeral,
    hide_in_help,
    category = "Owner Commands"
)]
pub async fn upload_cookies(ctx: Context<'_>, file: serenity::Attachment) -> CommandResult {
    ctx.defer().await?;
    let data_manager = ctx.data().data_manager.clone();
    let file = match file.download().await {
        Ok(down) => {
            tracing::info!("downloaded file from discord");
            down
        }
        Err(e) => {
            tracing::error!("error downloading file");
            return Err(BotError::DownloadAttachmentError(e));
        }
    };

    let add = data_manager.add_new_cookie(file).await;
    if let Err(e) = add {
        ctx.reply("Error").await?;
        tracing::error!("{e}");
        return Err(e.into());
    } else {
        ctx.reply("Uploaded").await?;
    }

    Ok(())
}
