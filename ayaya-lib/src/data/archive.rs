//! Archive module
use std::{fmt::Debug, path::PathBuf};

use async_trait::async_trait;
use axum::extract::FromRef;
use entity_archive::prelude::*;
use miette::Diagnostic;
use migration_archive::{Migrator as SqliteMigrator, MigratorTrait};
use poise::serenity_prelude as serenity;
use sea_orm::{prelude::*, ConnectOptions, Database};
use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter};
use thiserror::Error;

use crate::data::error::DataError;

#[derive(Debug, Clone)]
pub struct Service<MR>
where
    MR: ArchiveMessageRepository + Debug,
{
    message_repository: MR,
}

impl<MR> Service<MR>
where
    MR: ArchiveMessageRepository + Debug,
{
    pub fn new(message_repository: MR) -> Self {
        Self { message_repository }
    }
}

#[async_trait]
impl<MR> ArchiveService for Service<MR>
where
    MR: ArchiveMessageRepository + Debug + Send,
{
    async fn process_message(&mut self, message: serenity::Message) -> Result<(), ArchiveError> {
        self.archive_message(message).await?;
        Ok(())
    }
    async fn archive_message(&mut self, message: serenity::Message) -> Result<(), ArchiveError> {
        self.message_repository.save_message(message).await?;
        Ok(())
    }
    async fn archive_attachment(&mut self) -> Result<(), ArchiveError> {
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct SqliteMessageRepository {
    db: DatabaseConnection,
}

impl SqliteMessageRepository {
    pub async fn new(data_dir: PathBuf) -> Result<Self, DataError> {
        let mut connect_options = ConnectOptions::new(format!(
            "sqlite://{}?mode=rwc",
            data_dir.join("archive.sqlite").display()
        ));
        connect_options.sqlx_logging(false);
        let db: DatabaseConnection = Database::connect(connect_options)
            .await
            .map_err(|error| DataError::DatabaseConnectionError { error })?;

        SqliteMigrator::up(&db, None)
            .await
            .map_err(|error| DataError::MigrationError { error })?; // always upgrade db to the latest version

        Ok(Self { db })
    }
}

#[async_trait]
impl ArchiveMessageRepository for SqliteMessageRepository {
    async fn save_message(&mut self, message: serenity::Message) -> Result<(), ArchiveError> {
        use entity_archive::messages;

        let message_id = message.id.get();
        let guild_id = message.guild_id.expect("exits").get();
        let channel_id = message.channel_id.get();
        let author_id = message.author.id.get();
        let timestamp = message.timestamp.unix_timestamp();
        let timestamp = time::OffsetDateTime::from_unix_timestamp(timestamp)?;

        let exist = Messages::find()
            .filter(messages::Column::MessageId.eq(message_id))
            .one(&self.db)
            .await?;

        if exist.is_none() {
            messages::ActiveModel {
                message_id: sea_orm::ActiveValue::Set(message_id as i64),
                guild_id: sea_orm::ActiveValue::Set(guild_id as i64),
                channel_id: sea_orm::ActiveValue::Set(channel_id as i64),
                author_id: sea_orm::ActiveValue::Set(author_id as i64),
                timestamp: sea_orm::ActiveValue::Set(timestamp),
                message: sea_orm::ActiveValue::Set(serde_json::to_value(message)?),
            }
            .insert(&self.db)
            .await?;
        } else {
            return Err(ArchiveError::DuplicateError);
        }
        Ok(())
    }
    async fn get_message(
        &mut self,
        message_id: serenity::MessageId,
    ) -> Result<Option<serenity::Message>, ArchiveError> {
        use entity_archive::messages;

        let message_id = message_id.get();

        let exist = Messages::find()
            .filter(messages::Column::MessageId.eq(message_id))
            .one(&self.db)
            .await?;

        if let Some(message) = exist {
            let message = serde_json::from_value::<serenity::Message>(message.message)?;
            Ok(Some(message))
        } else {
            Ok(None)
        }
    }
    async fn get_channel_messages(
        &mut self,
        channel_id: serenity::ChannelId,
    ) -> Result<Vec<entity_archive::messages::Model>, ArchiveError> {
        use entity_archive::messages;

        let channel_id = channel_id.get();

        let messages = Messages::find()
            .filter(messages::Column::ChannelId.eq(channel_id))
            .all(&self.db)
            .await?;

        Ok(messages)
    }
}

#[async_trait]
pub trait ArchiveService {
    /// Processes the message, figuring out what else needs to be downloaded to be archived
    async fn process_message(&mut self, message: serenity::Message) -> Result<(), ArchiveError>;
    async fn archive_message(&mut self, message: serenity::Message) -> Result<(), ArchiveError>;
    async fn archive_attachment(&mut self) -> Result<(), ArchiveError>;
}

#[async_trait]
pub trait ArchiveMessageRepository {
    async fn save_message(&mut self, message: serenity::Message) -> Result<(), ArchiveError>;
    async fn get_message(
        &mut self,
        message_id: serenity::MessageId,
    ) -> Result<Option<serenity::Message>, ArchiveError>;
    async fn get_channel_messages(
        &mut self,
        channel_id: serenity::ChannelId,
    ) -> Result<Vec<entity_archive::messages::Model>, ArchiveError>;
}

pub trait ArchiveImageRepository {}

#[derive(Error, Diagnostic, Debug)]
pub enum ArchiveError {
    #[error("Generic error from database: {0}")]
    DatabaseError(#[from] sea_orm::DbErr),
    #[error("Failed doing JSON operations: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("A previous entry exists.")]
    DuplicateError,
    #[error("Out of range error while converting time: {0}")]
    TimeOutOfRangeError(#[from] time::error::ComponentRange),
}
