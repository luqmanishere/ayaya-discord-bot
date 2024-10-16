use miette::Diagnostic;
use poise::serenity_prelude as serenity;
use thiserror::Error;
use tracing::error;

use crate::{voice::error::MusicCommandError, Data};

pub async fn error_handler(error: poise::FrameworkError<'_, Data, BotError>) {
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

            if let Err(e) = ctx
                .send(poise::CreateReply::default().embed(command_error_embed(cmd, error)))
                .await
            {
                error!("Error sending error message: {}", e);
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
    #[diagnostic(help("Try leaving and rejoining the voice channel."))]
    GuildCacheStale,
    #[error("Ayaya is confused, how is your guild info not matching?")]
    GuildMismatch,
    #[error("An error occured with serenity: {0}")]
    GeneralSerenityError(#[from] serenity::Error),
}

pub fn command_error_embed(command: String, error: BotError) -> serenity::CreateEmbed {
    serenity::CreateEmbed::default()
        .color(serenity::Color::DARK_RED)
        .author(
            serenity::CreateEmbedAuthor::new(format!("Error In Command | {}", command)).icon_url(
                "https://cliply.co/wp-content/uploads/2019/04/371903520_SOCIAL_ICONS_YOUTUBE.png",
            ),
        )
        .description(
            serenity::MessageBuilder::default()
                .push_line(format!("### {}", error))
                .push_line({
                    let error_help = { error.help().map(|e| e.to_string()) };
                    let error_help = match error_help {
                        Some(error_str) => error_str,
                        None => "Undescribed error, please ping @solemnattic".to_string(),
                    };
                    format!("**Help**: {}", error_help)
                })
                .to_string(),
        )
        .timestamp(serenity::Timestamp::now())
        .footer(serenity::CreateEmbedFooter::new("Ayaya Discord Bot"))
}
