//! Contains commands reserved for the bot's owner: ie me.
use ::serenity::futures::StreamExt;
use poise::serenity_prelude as serenity;

use crate::{
    data::archive::ArchiveService, error::BotError, utils::GuildInfo, CommandResult, Commands,
    Context,
};

pub fn owner_commands() -> Commands {
    vec![
        command_log_raw(),
        upload_cookies(),
        dep_versions(),
        history(),
    ]
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

    ctx.reply(format!("```\n{logs:#?}\n```")).await?;
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

#[poise::command(
    slash_command,
    prefix_command,
    owners_only,
    ephemeral,
    hide_in_help,
    category = "Owner Commands"
)]
pub async fn dep_versions(ctx: Context<'_>) -> CommandResult {
    ctx.defer().await?;
    let mut message = serenity::MessageBuilder::default();

    // yt-dlp version
    let yt_dlp = tokio::process::Command::new("yt-dlp")
        .arg("--version")
        .output()
        .await
        .map_err(BotError::ExternalAsyncCommandError)?;
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

    ctx.reply(message.build()).await?;
    Ok(())
}

#[poise::command(
    slash_command,
    prefix_command,
    owners_only,
    ephemeral,
    hide_in_help,
    guild_only,
    category = "Owner Commands"
)]

pub async fn history(ctx: Context<'_>) -> CommandResult {
    ctx.defer().await?;
    let guild = GuildInfo::from_ctx(ctx)?;

    let channels = guild.guild_id.channels(ctx).await?;

    let folder = std::path::PathBuf::from(format!("dev/{}_{}", guild.guild_name, guild.guild_id));
    std::fs::create_dir_all(&folder).unwrap();

    for (channel_id, _guild_channel) in channels {
        tracing::info!(
            "Streaming messages from channel {} ({channel_id}) from guild {} ({})",
            _guild_channel.name,
            guild.guild_name,
            guild.guild_id
        );

        let mut messages_buf = vec![];

        let mut messages_iter = channel_id.messages_iter(ctx).boxed();
        while let Some(messages_result) = messages_iter.next().await {
            match messages_result {
                Ok(message) => {
                    messages_buf.push(message.clone());
                    match ctx
                        .data()
                        .data_manager
                        .archive()
                        .process_message(message)
                        .await
                    {
                        Ok(_) => {}
                        Err(e) => {
                            tracing::error!("Failed to archive message: {e}");
                        }
                    }
                }
                Err(err) => tracing::error!("Error getting message: {err}"),
            }
        }

        let file_name = folder.join(format!("{}_{}.json", _guild_channel.name, channel_id.get()));
        let to_write = serde_json::to_string_pretty(&messages_buf).unwrap();
        std::fs::write(&file_name, to_write).unwrap();
        tracing::info!(
            "Wrote messages from {} ({channel_id}) to {}",
            _guild_channel.name,
            file_name.display()
        );
    }

    tracing::info!(
        "Finished downloading messages from guild {} ({})",
        guild.guild_name,
        guild.guild_id
    );
    ctx.reply(format!(
        "Finished downloading messages from guild {} ({})",
        guild.guild_name, guild.guild_id
    ))
    .await?;
    Ok(())
}
