//! Manage database connection and caching
//!
use std::sync::Arc;

use ::serenity::futures::TryFutureExt;
use entity::prelude::*;
use error::DataError;
use lru_mem::{HeapSize, LruCache};
use migration::{Migrator, MigratorTrait};
use poise::serenity_prelude as serenity;
use sea_orm::{
    prelude::*, ActiveValue, ConnectOptions, EntityOrSelect, IntoActiveModel, QueryOrder,
    QuerySelect,
};
use sea_orm::{Database, DatabaseConnection};
use time::UtcOffset;
use tokio::sync::Mutex;

pub type DataResult<T> = Result<T, DataError>;

/// Manage data connection and caching. Cache access, insert and invalidation are implemented as
/// methods in this struct.
///
/// ## Cache details
///
/// Cache is split into relevant parts of the data (eg: permissions). To access the cache, the key
/// consists of parts tha encode the details of the access (see [`PermissionCacheKey`]). This makes
/// it easy to remove via the `.retain` method by simply comparing fields.
#[derive(Clone, Debug)]
pub struct DataManager {
    db: DatabaseConnection, // this is already clone
    permission_cache: Arc<Mutex<LruCache<PermissionCacheKey, Vec<u8>>>>,
}

impl DataManager {
    /// A new instance of the manager
    pub async fn new(url: &str) -> DataResult<Self> {
        let mut connect_options = ConnectOptions::new(url);
        connect_options.sqlx_logging(false); // disable sqlx logging
        let db: DatabaseConnection = Database::connect(connect_options)
            .await
            .map_err(|error| DataError::DatabaseConnectionError { error })?;
        Migrator::up(&db, None)
            .await
            .map_err(|error| DataError::MigrationError { error })?; // always upgrade db to the latest version
        Ok(Self {
            db,
            permission_cache: Arc::new(Mutex::new(LruCache::new(1024 * 1024))),
        })
    }

    /// Find a value in the permission cache. Returns a [`None`] if there is no value for the provided key
    pub async fn permission_cache_access(&mut self, key: &PermissionCacheKey) -> Option<Vec<u8>> {
        let mut cache = self.permission_cache.lock().await;
        let value = cache.get(key).cloned();
        if value.is_some() {
            tracing::debug!("found in cache");
        } else {
            tracing::debug!("cache miss");
        }
        value
    }

    /// Inserts a new entry into the permission cache. The cache stores [`Vec<u8>`], which may be used directly
    /// or by encoding items to bincode. Bincode encoding is fast, and does not take up much time.
    pub async fn permission_cache_insert(&mut self, key: PermissionCacheKey, value: Vec<u8>) {
        let mut cache = self.permission_cache.lock().await;
        if let Err(error) = cache.insert(key, value) {
            tracing::error!("error inserting into permission cache: {error}");
        };
    }

    /// Invalidates the cache based on the key given. The key is broken into each part and any containing part is invalidated(removed).
    pub async fn permission_cache_invalidate(&mut self, key: PermissionCacheKey) {
        let mut cache = self.permission_cache.lock().await;
        cache.retain(|cached_key, _| {
            // remove the cache of the command for the server
            !(cached_key.comorcat == key.comorcat && cached_key.guild_id == key.guild_id)
        });
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
        use entity::command_allow_user;
        const OP: &str = "find_user_allowed";
        let cache_key = PermissionCacheKey {
            user_id: Some(user_id),
            guild_id,
            operation: OP,
            comorcat: command.to_string(),
        };

        let entry = {
            if let Some(entry) = self.permission_cache_access(&cache_key).await {
                let (decoded, _): (Option<command_allow_user::Model>, _) =
                    bincode::decode_from_slice(&entry, bincode::config::standard()).unwrap();
                decoded
            } else {
                let model = CommandAllowUser::find()
                    .filter(command_allow_user::Column::ServerId.eq(guild_id))
                    .filter(command_allow_user::Column::UserId.eq(user_id))
                    .filter(command_allow_user::Column::Command.eq(command))
                    .one(&self.db)
                    .map_err(|error| DataError::FindAllowedUserError { error })
                    .await?;
                let encode = bincode::encode_to_vec(&model, bincode::config::standard()).unwrap();
                self.permission_cache_insert(cache_key, encode).await;
                model
            }
        };
        Ok(entry)
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
        use entity::require_command_role;
        const OP: &str = "find_command_roles_allowed";
        let cache_key = PermissionCacheKey {
            user_id: None,
            guild_id,
            operation: OP,
            comorcat: command.to_string(),
        };

        let entry = {
            if let Some(entry) = self.permission_cache_access(&cache_key).await {
                let (decoded, _): (Vec<require_command_role::Model>, _) =
                    bincode::decode_from_slice(&entry, bincode::config::standard()).unwrap();
                decoded
            } else {
                let model = RequireCommandRole::find()
                    .filter(require_command_role::Column::ServerId.eq(guild_id))
                    .filter(require_command_role::Column::Command.eq(command))
                    .all(&self.db)
                    .map_err(|error| DataError::FindCommandRolesAllowedError { error })
                    .await?;
                if let Ok(encoded) = bincode::encode_to_vec(&model, bincode::config::standard()) {
                    self.permission_cache_insert(cache_key, encoded).await;
                };
                model
            }
        };
        Ok(entry)
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
        const OP: &str = "find_category_roles_allowed";
        let cache_key = PermissionCacheKey {
            user_id: None,
            guild_id,
            operation: OP,
            comorcat: command_category.to_string(),
        };

        let entry = {
            if let Some(entry) = self.permission_cache_access(&cache_key).await {
                let (decoded, _): (Vec<require_category_role::Model>, _) =
                    bincode::decode_from_slice(&entry, bincode::config::standard()).unwrap();
                decoded
            } else {
                let model = RequireCategoryRole::find()
                    .filter(require_category_role::Column::ServerId.eq(guild_id))
                    .filter(require_category_role::Column::Category.eq(command_category))
                    .all(&self.db)
                    .map_err(DataError::FindCategoryRolesAllowedDatabaseError)
                    .await?;
                if let Ok(encoded) = bincode::encode_to_vec(&model, bincode::config::standard()) {
                    self.permission_cache_insert(cache_key, encoded).await;
                };
                model
            }
        };
        Ok(entry)
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
            // invalidate the cache
            self.permission_cache_invalidate(PermissionCacheKey {
                user_id: None,
                guild_id,
                operation: "",
                comorcat: command.to_string(),
            })
            .await;
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
            // cache invalidation
            self.permission_cache_invalidate(PermissionCacheKey {
                user_id: None,
                guild_id,
                operation: "",
                comorcat: command_category.to_string(),
            })
            .await;
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
            // cache invalidation
            self.permission_cache_invalidate(PermissionCacheKey {
                user_id: Some(user_id),
                guild_id,
                operation: "",
                comorcat: command.to_string(),
            })
            .await;
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

/// The cache key for the permission cache. Each detail is split into a field for easy comparison
#[derive(Debug, Eq, Hash, PartialEq, Clone)]
pub struct PermissionCacheKey {
    /// The user id in u64, if any
    pub user_id: Option<u64>,
    /// The guild id in u64
    pub guild_id: u64,
    /// The name of the function calling
    pub operation: &'static str,
    /// The name of the bot command or command category
    pub comorcat: String,
}

impl HeapSize for PermissionCacheKey {
    fn heap_size(&self) -> usize {
        self.comorcat.heap_size() + self.operation.heap_size()
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
