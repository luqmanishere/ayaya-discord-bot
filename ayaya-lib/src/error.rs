use miette::Diagnostic;
use poise::serenity_prelude as serenity;
use thiserror::Error;
use tracing::error;

use crate::{
    metrics::ErrorType,
    voice::{commands::soundboard::error::SoundboardError, error::MusicCommandError},
    Data,
};

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
            ctx.data()
                .metrics
                .error(error.name(), ErrorType::Command)
                .await;
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

pub trait ErrorName {
    fn name(&self) -> String;
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
    #[error("An error occured with the database: {0}")]
    DatabaseOperationError(#[from] sea_orm::DbErr),
    #[error("An error occured within the data manager: {0}")]
    DataManagerError(#[from] crate::data::error::DataError),
    #[error("Error downloading attachment: {0}")]
    DownloadAttachmentError(serenity::Error),
    #[error("Error executing external command: {0}")]
    ExternalAsyncCommandError(tokio::io::Error),
    #[error("Error executing external command: {0}")]
    ExternalCommandError(std::io::Error),
    #[error("Error accessing filesystem at: {} : {error}", path.display())]
    FilesystemAccessError {
        error: std::io::Error,
        path: std::path::PathBuf,
    },
    #[error("Other error: {0}")]
    OtherError(Box<std::io::Error>),
}

impl ErrorName for BotError {
    fn name(&self) -> String {
        let name = match self {
            BotError::MusicCommandError(music_command_error) => &music_command_error.name(),
            BotError::NoGuildId => "no_guild_id",
            BotError::NoGuild => "no_guild",
            BotError::GuildCacheStale => "guild_cache_stale",
            BotError::GuildMismatch => "guild_mismatch",
            BotError::GeneralSerenityError(..) => "serenity_error",
            BotError::DatabaseOperationError(..) => "database_error",
            BotError::DataManagerError(data_error) => &data_error.name(),
            BotError::DownloadAttachmentError(..) => "download_attachment_error",
            BotError::ExternalAsyncCommandError(..) => "external_async_commaand_error",
            BotError::ExternalCommandError(..) => "external_command_error",
            BotError::FilesystemAccessError { .. } => "filesystem_access_error",
            BotError::OtherError(_) => "other_error",
        };
        format!("main::{name}")
    }
}

impl From<SoundboardError> for BotError {
    fn from(value: SoundboardError) -> Self {
        MusicCommandError::SoundboardError(value).into()
    }
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
