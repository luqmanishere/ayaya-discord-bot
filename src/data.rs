//! Manage database connection and caching
//!

use ::serenity::futures::TryFutureExt;
use entity::prelude::*;
use error::DataError;
use migration::{Migrator, MigratorTrait};
use poise::serenity_prelude as serenity;
use sea_orm::{prelude::*, ActiveValue, EntityOrSelect, IntoActiveModel, QueryOrder, QuerySelect};
use sea_orm::{Database, DatabaseConnection};
use time::UtcOffset;

pub type DataResult<T> = Result<T, DataError>;

/// Manage data connection and caching. The principle of operation is simple.
/// When data is pulled for the first time, it gets cached. Any addition later on is also added to
/// the cache after being pushed to the database. This saves some network access.
///
/// ## Why is this fine, what about concurrent access?
///
/// This bot is not designed to be a distributed software running on HA or anything like that. It's
/// a single program, if it dies it dies there is no external concurrent access. Hence, a simple
/// model that does not query the database for any changes while it is running (or only
/// periodically) is suitable for this use case.
///
/// I don't even know how to make a distributed kind of discord bot (if that is even possible).
#[derive(Clone, Debug)]
pub struct DataManager {
    db: DatabaseConnection, // this is already clone
}

impl DataManager {
    /// A new instance of the manager
    pub async fn new(url: &str) -> DataResult<Self> {
        let db: DatabaseConnection = Database::connect(url)
            .await
            .map_err(|error| DataError::DatabaseConnectionError { error })?;
        Migrator::up(&db, None)
            .await
            .map_err(|error| DataError::MigrationError { error })?; // always upgrade db to the latest version
        Ok(Self { db })
    }

    /// Log command calls to the database. Will also increment the command counter.
    pub async fn log_command_call(
        &mut self,
        guild_id: u64,
        author: &serenity::User,
        command_name: String,
    ) -> DataResult<()> {
        let db = &self.db;
        let now_odt = time::OffsetDateTime::now_utc()
            .to_offset(UtcOffset::from_hms(8, 0, 0).unwrap_or(UtcOffset::UTC));
        let call_log = entity::command_call_log::ActiveModel {
            log_id: sea_orm::ActiveValue::Set(uuid::Uuid::new_v4()),
            server_id: sea_orm::ActiveValue::Set(guild_id),
            user_id: sea_orm::ActiveValue::Set(author.id.get()),
            command: sea_orm::ActiveValue::Set(command_name.clone()),
            command_time_stamp: sea_orm::ActiveValue::Set(now_odt),
        };
        call_log
            .insert(db)
            .await
            .map_err(|error| DataError::LogCommandCallError { error })?;

        self.increment_command_counter(guild_id, author, command_name)
            .await?;
        Ok(())
    }

    /// Increments the all-time command counter for the user
    ///
    /// # Returns
    ///
    /// - An empty [`Result`] with the Error [`DataError`]
    pub async fn increment_command_counter(
        &mut self,
        guild_id: u64,
        author: &serenity::User,
        command_name: String,
    ) -> DataResult<()> {
        let db = &self.db;
        use entity::user_command_all_time_statistics;
        let user: Option<user_command_all_time_statistics::Model> =
            UserCommandAllTimeStatistics::find()
                .filter(user_command_all_time_statistics::Column::ServerId.eq(guild_id))
                .filter(user_command_all_time_statistics::Column::UserId.eq(author.id.get()))
                .filter(user_command_all_time_statistics::Column::Command.eq(command_name.clone()))
                .one(db)
                .await
                .map_err(|error| DataError::IncrementCommandCounterError { error })?;

        if let Some(stats) = user {
            let count = stats.count + 1;
            let mut model = stats.into_active_model();
            model.count = ActiveValue::set(count);
            model
                .save(db)
                .await
                .map_err(|error| DataError::IncrementCommandCounterError { error })?;
        } else {
            user_command_all_time_statistics::ActiveModel {
                server_id: sea_orm::ActiveValue::Set(guild_id),
                user_id: sea_orm::ActiveValue::Set(author.id.get()),
                command: sea_orm::ActiveValue::Set(command_name),
                count: sea_orm::ActiveValue::Set(1),
            }
            .insert(db)
            .await
            .map_err(|error| DataError::IncrementCommandCounterError { error })?;
        }
        Ok(())
    }

    /// Finds an entry for allowed user.
    ///
    /// If an entry exists, the return type will be [`Some`].
    ///
    /// # Errors
    ///
    /// This function will return an error if there is an error in accessing the database.
    pub async fn find_user_allowed(
        &mut self,
        guild_id: u64,
        user_id: u64,
        command: &str,
    ) -> DataResult<Option<entity::command_allow_user::Model>> {
        // TODO: caching
        use entity::command_allow_user;
        CommandAllowUser::find()
            .filter(command_allow_user::Column::ServerId.eq(guild_id))
            .filter(command_allow_user::Column::UserId.eq(user_id))
            .filter(command_allow_user::Column::Command.eq(command))
            .one(&self.db)
            .map_err(|error| DataError::FindAllowedUserError { error })
            .await
    }

    /// Finds the allowed roles for the command, if any. Returns an empty [`Vec`] if no allowed
    /// roles exist.
    ///
    /// # Errors
    ///
    /// This function will return an error if there is an error accessing the database.
    pub async fn find_command_roles_allowed(
        &mut self,
        guild_id: u64,
        command: &str,
    ) -> DataResult<Vec<entity::require_command_role::Model>> {
        // TODO: caching
        use entity::require_command_role;
        RequireCommandRole::find()
            .filter(require_command_role::Column::ServerId.eq(guild_id))
            .filter(require_command_role::Column::Command.eq(command))
            .all(&self.db)
            .map_err(|error| DataError::FindCommandRolesAllowedError { error })
            .await
    }

    /// Finds the allowed roles for the category, if any. Return an empty [`Vec`] if no allowed
    /// roles exist.
    ///
    /// # Errors
    ///
    /// This function will return an error if there is an error accessing the database.
    pub async fn find_category_roles_allowed(
        &mut self,
        guild_id: u64,
        command_category: &str,
    ) -> DataResult<Vec<entity::require_category_role::Model>> {
        use entity::require_category_role;
        RequireCategoryRole::find()
            .filter(require_category_role::Column::ServerId.eq(guild_id))
            .filter(require_category_role::Column::Category.eq(command_category))
            .all(&self.db)
            .map_err(DataError::FindCategoryRolesAllowedDatabaseError)
            .await
    }

    /// Inserts a new command role restriction into the database. A check is done before insertion
    /// to determine if the restriction already exists, which will be reflected in the error that
    /// will be returned
    ///
    /// # Errors
    ///
    /// This function will return an error if the entry already exists or an error is returned from
    /// the database.
    pub async fn new_command_role_restriction(
        &mut self,
        guild_id: u64,
        role: &serenity::Role,
        command: &str,
    ) -> DataResult<entity::require_command_role::Model> {
        use entity::require_command_role;
        let existing = self
            .find_command_roles_allowed(guild_id, command)
            .await?
            .into_iter()
            .find(|e| e.role_id == role.id.get());
        if existing.is_some() {
            Err(DataError::NewCommandRoleRestrictionDuplicate)
        } else {
            let model = require_command_role::ActiveModel {
                entry_id: sea_orm::ActiveValue::Set(Uuid::now_v7()),
                server_id: sea_orm::ActiveValue::Set(guild_id),
                role_id: sea_orm::ActiveValue::Set(role.id.get()),
                command: sea_orm::ActiveValue::Set(command.to_string()),
            }
            .insert(&self.db)
            .await
            .map_err(DataError::NewCommandRoleRestrictionDatabaseError)?;
            Ok(model)
        }
    }

    /// Inserts a new category role restriction into the database. A check is done before insertion
    /// to determine if the restriction already exists, which will be reflected in the error that
    /// will be returned
    ///
    /// # Errors
    ///
    /// This function will return an error if the entry already exists or an error is returned from
    /// the database.
    pub async fn new_category_role_restriction(
        &mut self,
        guild_id: u64,
        role: &serenity::Role,
        command_category: &str,
    ) -> DataResult<entity::require_category_role::Model> {
        use entity::require_category_role;
        let existing = self
            .find_category_roles_allowed(guild_id, command_category)
            .await?
            .into_iter()
            .find(|e| e.role_id == role.id.get());
        if existing.is_some() {
            Err(DataError::NewCategoryRoleRestrictionDuplicate)
        } else {
            let model = require_category_role::ActiveModel {
                entry_id: sea_orm::ActiveValue::Set(Uuid::now_v7()),
                server_id: sea_orm::ActiveValue::Set(guild_id),
                role_id: sea_orm::ActiveValue::Set(role.id.get()),
                category: sea_orm::ActiveValue::Set(command_category.to_string()),
            }
            .insert(&self.db)
            .await
            .map_err(DataError::NewCategoryRoleRestrictionDatabaseError)?;
            Ok(model)
        }
    }

    pub async fn new_command_user_allowed(
        &mut self,
        guild_id: u64,
        user_id: u64,
        command: &str,
    ) -> DataResult<entity::command_allow_user::Model> {
        use entity::command_allow_user;
        let existing = self.find_user_allowed(guild_id, user_id, command).await?;
        if existing.is_some() {
            Err(DataError::NewCommandAllowedUserDuplicate)
        } else {
            let model = command_allow_user::ActiveModel {
                entry_id: sea_orm::ActiveValue::Set(Uuid::now_v7()),
                server_id: sea_orm::ActiveValue::Set(guild_id),
                user_id: sea_orm::ActiveValue::Set(user_id),
                command: sea_orm::ActiveValue::Set(command.to_string()),
            }
            .insert(&self.db)
            .await
            .map_err(DataError::NewCommandAllowedUserDatabaseError)?;
            Ok(model)
        }
    }

    /// Finds all allowed users for a command within the guild. Returns a [`Vec`] with the matching
    /// Models
    ///
    /// # Errors
    ///
    /// This function will return an error if an error occured with the database.
    pub async fn findall_user_allowed(
        &self,
        guild_id: u64,
        command: &str,
    ) -> DataResult<Vec<entity::command_allow_user::Model>> {
        use entity::command_allow_user;
        CommandAllowUser::find()
            .filter(command_allow_user::Column::ServerId.eq(guild_id))
            .filter(command_allow_user::Column::Command.eq(command))
            .all(&self.db)
            .await
            .map_err(DataError::FindAllAllowedUserDatabaseError)
    }

    /// Finds the last 5 entries of the command call log
    ///
    /// # Errors
    ///
    /// This function will return an error if there is an error with the database.
    pub async fn find5_command_log(&self) -> DataResult<Vec<entity::command_call_log::Model>> {
        CommandCallLog::find()
            .limit(5)
            .all(&self.db)
            .await
            .map_err(DataError::Find5CommandCallLogDatabaseError)
    }

    /// Find the user command all time statistics for a user in a guild. Returns [`None`] if the
    /// user has never called the command before
    ///
    /// # Errors
    ///
    /// This function will return an error if .
    pub async fn find_user_all_time_command_stats(
        &self,
        guild_id: u64,
        user_id: u64,
        command: &str,
    ) -> DataResult<Option<entity::user_command_all_time_statistics::Model>> {
        use entity::user_command_all_time_statistics;
        UserCommandAllTimeStatistics::find()
            .filter(user_command_all_time_statistics::Column::ServerId.eq(guild_id))
            .filter(user_command_all_time_statistics::Column::UserId.eq(user_id))
            .filter(user_command_all_time_statistics::Column::Command.eq(command))
            .one(&self.db)
            .await
            .map_err(DataError::FindUserAllTimeCommandStatsDatabaseError)
    }

    pub async fn get_latest_cookies(&self) -> DataResult<Option<entity::youtube_cookies::Model>> {
        use entity::youtube_cookies;
        YoutubeCookies::find()
            .select()
            .order_by(youtube_cookies::Column::EntryId, sea_orm::Order::Desc)
            .limit(1)
            .one(&self.db)
            .await
            .map_err(DataError::GetLatestCookiesDatabaseError)
    }

    pub async fn add_new_cookie(&self, file: Vec<u8>) -> DataResult<()> {
        use entity::youtube_cookies;
        let _model = youtube_cookies::ActiveModel {
            entry_id: ActiveValue::NotSet,
            date: ActiveValue::Set(time::OffsetDateTime::now_utc()),
            cookies: ActiveValue::Set(file),
        }
        .save(&self.db)
        .await
        .map_err(DataError::AddNewCookieDatabaseError)?;
        Ok(())
    }
}

pub mod error {
    use sea_orm::DbErr;

    #[derive(thiserror::Error, miette::Diagnostic, Debug)]
    pub enum DataError {
        #[error("Error connecting to database: {error}")]
        DatabaseConnectionError { error: DbErr },
        #[error("Error performing migration: {error}")]
        MigrationError { error: DbErr },
        #[error("Error incrementing command counter: {error}")]
        IncrementCommandCounterError { error: DbErr },
        #[error("Error logging command call: {error}")]
        LogCommandCallError { error: DbErr },
        #[error("Error finding allowed user: {error}")]
        FindAllowedUserError { error: DbErr },
        #[error("Error finding allowed command roles: {error}")]
        FindCommandRolesAllowedError { error: DbErr },
        #[error("Error finding allowed category roles: {0}")]
        FindCategoryRolesAllowedDatabaseError(DbErr),
        #[error("Database error while creating new command role restriction: {0}")]
        NewCommandRoleRestrictionDatabaseError(DbErr),
        #[error("A duplicate entry is found while creating new command role restriction")]
        NewCommandRoleRestrictionDuplicate,
        #[error("Database error while creating new category role restriction: {0}")]
        NewCategoryRoleRestrictionDatabaseError(DbErr),
        #[error("A duplicate entry is found while creating new category role restriction")]
        NewCategoryRoleRestrictionDuplicate,
        #[error("A duplicate entry is found while creating new allowed user")]
        NewCommandAllowedUserDuplicate,
        #[error("Database error while creating new command allowed user: {0}")]
        NewCommandAllowedUserDatabaseError(DbErr),
        #[error("Database error while finding all allowed users: {0}")]
        FindAllAllowedUserDatabaseError(DbErr),
        #[error("Database error while finding command call log: {0}")]
        Find5CommandCallLogDatabaseError(DbErr),
        #[error("Database error while finding user all time command stats: {0}")]
        FindUserAllTimeCommandStatsDatabaseError(DbErr),
        #[error("Database error while getting cookies: {0}")]
        GetLatestCookiesDatabaseError(DbErr),
        #[error("Database error while adding cookies: {0}")]
        AddNewCookieDatabaseError(DbErr),
    }
}
