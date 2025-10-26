use poise::serenity_prelude as serenity;
use snafu::Snafu;
use tracing::error;

use crate::{
    Data,
    metrics::ErrorType,
    voice::{commands::soundboard::error::SoundboardError, error::MusicCommandError},
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
            msg.reply(ctx, format!("unrecognized command: {msg_content}"))
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

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum BotError {
    #[snafu(transparent)]
    MusicCommandError { source: MusicCommandError },

    #[snafu(transparent)]
    InitError { source: InitError },

    #[snafu(display("Ayaya is unable to figure out her Guild ID."))]
    NoGuildId,

    #[snafu(display("Ayaya is has confused her current Guild"))]
    NoGuild,

    #[snafu(display("Cache is stale, please rejoin voice channels"))]
    // #[diagnostic(help("Try leaving and rejoining the voice channel."))]
    GuildCacheStale,

    #[snafu(display("Ayaya is confused, how is your guild info not matching?"))]
    GuildMismatch,

    #[snafu(display("An error occured with serenity: {source}"))]
    GeneralSerenityError { source: serenity::Error },

    #[snafu(display("An error occured with the database: {source}"))]
    DatabaseOperationError { source: sea_orm::DbErr },

    #[snafu(display("An error occured within the data manager: {source}"))]
    DataManagerError {
        source: crate::data::error::DataError,
    },

    #[snafu(display("An error occured with the tracker: {source}"))]
    TrackerError {
        source: crate::tracker::error::TrackerError,
    },

    #[snafu(display("Error downloading attachment: {source}"))]
    DownloadAttachmentError { source: serenity::Error },

    #[snafu(display("Error executing external command: {source}"))]
    ExternalAsyncCommandError { source: tokio::io::Error },

    #[snafu(display("Error executing external command: {source}"))]
    ExternalCommandError { source: std::io::Error },

    #[snafu(display("Error accessing filesystem at: {} : {source}", path.display()))]
    FilesystemAccessError {
        source: std::io::Error,
        path: std::path::PathBuf,
    },

    #[snafu(display("Error (de)serializing JSON: {source}"))]
    UrlParseError { source: url::ParseError },

    #[snafu(display("Generic error: {source}"))]
    JsonError { source: serde_json::Error },

    #[snafu(display("Error from reqwest: {source}"))]
    ReqwestError { source: reqwest::Error },

    #[snafu(display("Error from std::io: {source}"))]
    IoError { source: std::io::Error },

    #[snafu(display("Generic error: {source}"))]
    GenericError {
        source: Box<dyn std::error::Error + Send + Sync>,
    },
}

pub trait UserFriendlyError {
    fn help_text(&self) -> &str;
    fn category(&self) -> ErrorCategory;
}

pub enum ErrorCategory {
    UserMistake,
    BotIssue,
    ExternalServiceIssue,
}

impl ErrorCategory {
    pub fn color(&self) -> serenity::Color {
        match self {
            ErrorCategory::UserMistake => serenity::Color::DARK_RED,
            ErrorCategory::BotIssue => serenity::Color::GOLD,
            ErrorCategory::ExternalServiceIssue => serenity::Color::BLUE,
        }
    }
}

impl UserFriendlyError for BotError {
    fn help_text(&self) -> &str {
        const DEFAULT: &str = "Contact @solemnattic for help";

        match self {
            BotError::MusicCommandError { source } => source.help_text(),
            BotError::NoGuildId => "Ayaya is unable to figure out her Guild ID.",
            BotError::NoGuild => "Ayaya is has confused her current Guild",
            BotError::GuildCacheStale => "Cache is stale, please rejoin voice channels",
            BotError::GuildMismatch => "Ayaya is confused, how is your guild info not matching?",
            _ => DEFAULT,
        }
    }
    fn category(&self) -> ErrorCategory {
        match self {
            BotError::MusicCommandError { source } => source.category(),
            BotError::NoGuildId => ErrorCategory::UserMistake,
            BotError::NoGuild => ErrorCategory::UserMistake,
            BotError::GuildCacheStale => ErrorCategory::UserMistake,
            BotError::GuildMismatch => ErrorCategory::UserMistake,
            _ => ErrorCategory::BotIssue,
        }
    }
}

impl ErrorName for BotError {
    fn name(&self) -> String {
        let name: &str = match self {
            BotError::MusicCommandError { source } => &source.name(),
            BotError::InitError { .. } => "init",
            BotError::NoGuildId => "no_guild_id",
            BotError::NoGuild => "no_guild",
            BotError::GuildCacheStale => "guild_cache_stale",
            BotError::GuildMismatch => "guild_mismatch",
            BotError::GeneralSerenityError { .. } => "serenity_error",
            BotError::DatabaseOperationError { .. } => "database_error",
            BotError::DataManagerError { source } => &source.name(),
            BotError::TrackerError { source } => &source.name(),
            BotError::DownloadAttachmentError { .. } => "download_attachment_error",
            BotError::ExternalAsyncCommandError { .. } => "external_async_commaand_error",
            BotError::ExternalCommandError { .. } => "external_command_error",
            BotError::FilesystemAccessError { .. } => "filesystem_access_error",
            BotError::UrlParseError { .. } => "url_parse_error",
            BotError::JsonError { .. } => "json_error",
            BotError::ReqwestError { .. } => "reqwest_error",
            BotError::IoError { .. } => "io_error",
            BotError::GenericError { .. } => "generic_error",
        };
        format!("main::{name}")
    }
}

impl From<SoundboardError> for BotError {
    fn from(source: SoundboardError) -> Self {
        Self::MusicCommandError {
            source: MusicCommandError::SoundboardError { source },
        }
    }
}

#[derive(Snafu, Debug)]
#[snafu(visibility(pub))]
pub enum InitError {
    #[snafu(display("Error building Loki metrics configuration: {source}"))]
    LokiBuilder { source: tracing_loki::Error },

    #[snafu(display("Error setting global tracing dispatcher: {source}"))]
    SetGlobalDefault {
        source: tracing::dispatcher::SetGlobalDefaultError,
    },

    TracingFilterParseError {
        source: tracing_subscriber::filter::ParseError,
    },
}

pub fn command_error_embed(command: String, error: BotError) -> serenity::CreateEmbed {
    let (error, help_text, color) = match &error {
        BotError::MusicCommandError { source } => (
            source.to_string(),
            source.help_text(),
            source.category().color(),
        ),
        BotError::NoGuildId => (
            error.to_string(),
            error.help_text(),
            serenity::Color::DARK_RED,
        ),
        BotError::GuildCacheStale => (error.to_string(), error.help_text(), serenity::Color::GOLD),
        _ => (
            error.to_string(),
            error.help_text(),
            serenity::Color::DARK_RED,
        ),
    };

    serenity::CreateEmbed::default()
        .color(color)
        .author(
            serenity::CreateEmbedAuthor::new(format!("Error In Command | {command}")).icon_url(
                "https://cliply.co/wp-content/uploads/2019/04/371903520_SOCIAL_ICONS_YOUTUBE.png",
            ),
        )
        .description(
            serenity::MessageBuilder::default()
                .push_line(format!("### {error}"))
                .push_line(format!("**Help**: {help_text}"))
                .to_string(),
        )
        .timestamp(serenity::Timestamp::now())
        .footer(serenity::CreateEmbedFooter::new("Ayaya Discord Bot"))
}
