use miette::Diagnostic;
use poise::serenity_prelude as serenity;
use thiserror::Error;
use tracing::error;

use crate::{voice::error::MusicCommandError, Data};

pub async fn error_handler(error: poise::FrameworkError<'_, Data, BotError>) {
    // TODO: log errors and send constructive replies to users
    error!("error error error {}", error);
    match error {
        poise::FrameworkError::ArgumentParse { error, .. } => {
            if let Some(error) = error.downcast_ref::<serenity::RoleParseError>() {
                error!("Found a RoleParseError: {:?}", error);
            } else {
                error!("Not a RoleParseError :(");
            }
        }
        poise::FrameworkError::UnknownCommand {
            ctx,
            msg,
            msg_content,
            ..
        } => {
            error!("unrecognized command: {}", msg_content);
            msg.reply(ctx, format!("unrecognized command: {}", msg_content))
                .await
                .expect("no errors");
        }
        poise::FrameworkError::Command { error, ctx, .. } => {
            let cmd = ctx.command().name.clone();
            error!("Error executing command ({}): {}", cmd, error);
            // TODO: flesh out user facing error message

            let error_help = { error.help().map(|e| e.to_string()) };
            match error_help {
                Some(error_str) => {
                    ctx.reply(error_str).await.expect("ok");
                }
                None => {
                    ctx.channel_id()
                        .say(ctx, "Error running whatever you did")
                        .await
                        .expect("works");
                }
            }
        }
        other => {
            if let Err(e) = poise::builtins::on_error(other).await {
                error!("Error sending error message: {}", e);
            }
        }
    }
}

#[derive(Error, Diagnostic, Debug)]
pub enum BotError {
    #[error(transparent)]
    #[diagnostic(transparent)]
    MusicCommandError(#[from] MusicCommandError),
    #[error("Ayaya is unable to figure out her Guild ID.")]
    NoGuildId,
    #[error("Ayaya is has confused her current Guild")]
    NoGuild,
    #[error("Cache is stale, please rejoin voice channels")]
    GuildCacheStale,
    #[error("Ayaya is confused, how is your guild info not matching?")]
    GuildMismatch,
    #[error("An error occured with serenity: {0}")]
    GeneralSerenityError(#[from] serenity::Error),
}
