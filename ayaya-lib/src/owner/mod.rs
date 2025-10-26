//! Contains commands reserved for the bot's owner: ie me.
use poise::serenity_prelude as serenity;
use snafu::ResultExt;

use crate::{
    CommandResult, Commands, Context,
    error::{
        BotError, DataManagerSnafu, DownloadAttachmentSnafu, ExternalAsyncCommandSnafu,
        GeneralSerenitySnafu,
    },
};

pub fn owner_commands() -> Commands {
    vec![command_log_raw(), upload_cookies(), dep_versions()]
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
    ctx.defer().await.context(GeneralSerenitySnafu)?;

    let data_manager = ctx.data().data_manager.clone();

    let logs = data_manager
        .find5_command_log()
        .await
        .context(DataManagerSnafu)?;

    ctx.reply(format!("```\n{logs:#?}\n```"))
        .await
        .context(GeneralSerenitySnafu)?;
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
    ctx.defer().await.context(GeneralSerenitySnafu)?;
    let data_manager = ctx.data().data_manager.clone();
    let file = match file.download().await {
        Ok(down) => {
            tracing::info!("downloaded file from discord");
            down
        }
        Err(e) => {
            tracing::error!("error downloading file");
            return Err(e).context(DownloadAttachmentSnafu);
        }
    };

    let add = data_manager.add_new_cookie(file).await;
    if let Err(e) = add {
        ctx.reply("Error").await.context(GeneralSerenitySnafu)?;
        tracing::error!("{e}");
        return Err(e).context(DataManagerSnafu);
    } else {
        ctx.reply("Uploaded").await.context(GeneralSerenitySnafu)?;
    }

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
pub async fn dep_versions(ctx: Context<'_>) -> CommandResult {
    ctx.defer().await.context(GeneralSerenitySnafu)?;
    let mut message = serenity::MessageBuilder::default();

    // yt-dlp version
    let yt_dlp = tokio::process::Command::new("yt-dlp")
        .arg("--version")
        .output()
        .await
        .context(ExternalAsyncCommandSnafu)?;
    let yt_dlp_stdout = String::from_utf8(yt_dlp.stdout).unwrap_or_default();
    let yt_dlp_stderr = String::from_utf8(yt_dlp.stderr).unwrap_or_default();
    message.push_line("## yt-dlp");
    if !yt_dlp_stdout.is_empty() {
        message.push_line("### stdout");
        message.push_codeblock(yt_dlp_stdout, Some("sh"));
    }
    if !yt_dlp_stderr.is_empty() {
        message.push_line("### stderr");
        message.push_codeblock(yt_dlp_stderr, Some("sh"));
    }

    // TODO: add other external programs version

    ctx.reply(message.build())
        .await
        .context(GeneralSerenitySnafu)?;
    Ok(())
}
