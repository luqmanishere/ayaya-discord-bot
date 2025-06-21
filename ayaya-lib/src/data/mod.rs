//! Manage database connection and caching
//!
pub mod permissions;
pub mod sounds;
pub mod stats;
mod utils;

use std::sync::{Arc, Mutex};

use entity_sqlite::prelude::*;
use error::DataError;
use lru_mem::LruCache;
use migration_sqlite::{Migrator as SqliteMigrator, MigratorTrait};
use permissions::Permissions;
use poise::serenity_prelude as serenity;
use sea_orm::{
    prelude::*, ActiveValue, ConnectOptions, EntityOrSelect, IntoActiveModel, QueryOrder,
    QuerySelect,
};
use sea_orm::{Database, DatabaseConnection};
use sounds::SoundsManager;
use stats::StatsManager;
use time::UtcOffset;
use utils::DataTiming;

use crate::metrics::{DataOperationType, Metrics};

pub type DataResult<T> = Result<T, DataError>;
pub type Autocomplete = Arc<Mutex<LruCache<String, String>>>;

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
    metrics_handler: Metrics,
    permissions: Permissions,
    stats: StatsManager,
    sounds: SoundsManager,
    autocomplete_cache: Autocomplete,
}

impl DataManager {
    /// A new instance of the manager
    pub async fn new(db_url: &str, metrics_handler: Metrics) -> DataResult<Self> {
        let mut connect_options = if cfg!(debug_assertions) && !cfg!(test) {
            ConnectOptions::new("sqlite://dev/stats.sqlite?mode=rwc")
        } else {
            ConnectOptions::new(db_url)
        };
        connect_options.sqlx_logging(false); // disable sqlx logging
        let db: DatabaseConnection = Database::connect(connect_options)
            .await
            .map_err(|error| DataError::DatabaseConnectionError { error })?;
        SqliteMigrator::up(&db, None)
            .await
            .map_err(|error| DataError::MigrationError { error })?; // always upgrade db to the latest version

        let permissions = Permissions::new(db.clone(), metrics_handler.clone()).await?;
        let stats = StatsManager::new(db.clone(), metrics_handler.clone());
        let sounds = SoundsManager::new(db.clone(), metrics_handler.clone());
        Ok(Self {
            db,
            metrics_handler,
            permissions,
            stats,
            sounds,
            autocomplete_cache: Arc::new(Mutex::new(LruCache::new(1000 * 1024))),
        })
    }

    pub fn permissions_mut(&mut self) -> &mut Permissions {
        &mut self.permissions
    }

    #[expect(dead_code)]
    pub fn stats_mut(&mut self) -> &mut StatsManager {
        &mut self.stats
    }

    pub fn stats(&self) -> StatsManager {
        self.stats.clone()
    }

    #[expect(dead_code)]
    pub fn sounds_mut(&mut self) -> &mut SoundsManager {
        &mut self.sounds
    }

    pub fn sounds(&self) -> SoundsManager {
        self.sounds.clone()
    }

    /// Log command calls to the database. Will also increment the command counter.
    pub async fn log_command_call(
        &mut self,
        guild_id: u64,
        user_id: &serenity::UserId,
        command_name: String,
    ) -> DataResult<()> {
        const OP: &str = "log_command_call";
        self.metrics_handler
            .data_access(OP, DataOperationType::Write)
            .await;
        let _timing = DataTiming::new(
            OP.to_string(),
            DataOperationType::Write,
            Some(self.metrics_handler.clone()),
        );

        let db = &self.db;
        let now_odt = time::OffsetDateTime::now_utc()
            .to_offset(UtcOffset::from_hms(8, 0, 0).unwrap_or(UtcOffset::UTC));
        let call_log = entity_sqlite::command_call_log::ActiveModel {
            log_id: sea_orm::ActiveValue::Set(uuid::Uuid::new_v4()),
            server_id: sea_orm::ActiveValue::Set(guild_id as i64),
            user_id: sea_orm::ActiveValue::Set(user_id.get() as i64),
            command: sea_orm::ActiveValue::Set(command_name.clone()),
            command_time_stamp: sea_orm::ActiveValue::Set(now_odt),
        };
        call_log
            .insert(db)
            .await
            .map_err(|error| DataError::LogCommandCallError { error })?;

        self.increment_command_counter(guild_id, user_id, command_name)
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
        user_id: &serenity::UserId,
        command_name: String,
    ) -> DataResult<()> {
        self.metrics_handler
            .data_access("increment_command_counter", DataOperationType::Write)
            .await;
        let db = &self.db;
        use entity_sqlite::user_command_all_time_statistics;
        let user = UserCommandAllTimeStatistics::find()
            .filter(user_command_all_time_statistics::Column::ServerId.eq(guild_id))
            .filter(user_command_all_time_statistics::Column::UserId.eq(user_id.get()))
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
                server_id: sea_orm::ActiveValue::Set(guild_id as i64),
                user_id: sea_orm::ActiveValue::Set(user_id.get() as i64),
                command: sea_orm::ActiveValue::Set(command_name),
                count: sea_orm::ActiveValue::Set(1),
            }
            .insert(db)
            .await
            .map_err(|error| DataError::IncrementCommandCounterError { error })?;
        }
        Ok(())
    }

    /// Finds the last 5 entries of the command call log
    ///
    /// # Errors
    ///
    /// This function will return an error if there is an error with the database.
    pub async fn find5_command_log(
        &self,
    ) -> DataResult<Vec<entity_sqlite::command_call_log::Model>> {
        const OP: &str = "find5_command_log";
        let _timing = DataTiming::new(
            OP.to_string(),
            DataOperationType::Read,
            Some(self.metrics_handler.clone()),
        );
        self.metrics_handler
            .data_access(OP, DataOperationType::Read)
            .await;

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
    pub async fn find_single_user_single_all_time_command_stats(
        &self,
        guild_id: u64,
        user_id: u64,
        command: &str,
    ) -> DataResult<Option<entity_sqlite::user_command_all_time_statistics::Model>> {
        const OP: &str = "find_user_all_time_command_stats";
        let _timing = DataTiming::new(
            OP.to_string(),
            DataOperationType::Read,
            Some(self.metrics_handler.clone()),
        );
        self.metrics_handler
            .data_access("find_user_all_time_command_stats", DataOperationType::Read)
            .await;

        use entity_sqlite::user_command_all_time_statistics;
        UserCommandAllTimeStatistics::find()
            .filter(user_command_all_time_statistics::Column::ServerId.eq(guild_id))
            .filter(user_command_all_time_statistics::Column::UserId.eq(user_id))
            .filter(user_command_all_time_statistics::Column::Command.eq(command))
            .one(&self.db)
            .await
            .map_err(DataError::FindUserAllTimeCommandStatsDatabaseError)
    }

    /// Finds all the users data for a single command call
    ///
    /// # Errors
    ///
    /// This function will return an error if the database errors.
    pub async fn find_all_users_single_command_call(
        &self,
        guild_id: u64,
        command_name: String,
    ) -> DataResult<Vec<entity_sqlite::user_command_all_time_statistics::Model>> {
        const OP: &str = "rank_users_command_call";
        let _timing = DataTiming::new(
            OP.to_string(),
            DataOperationType::Read,
            Some(self.metrics_handler.clone()),
        );
        self.metrics_handler
            .data_access(OP, DataOperationType::Read)
            .await;

        use entity_sqlite::user_command_all_time_statistics;
        UserCommandAllTimeStatistics::find()
            .filter(user_command_all_time_statistics::Column::ServerId.eq(guild_id))
            .filter(user_command_all_time_statistics::Column::Command.eq(command_name))
            .all(&self.db)
            .await
            .map_err(DataError::FindSingleUsersSingleCommandCallError)
    }

    pub async fn get_latest_cookies(
        &self,
    ) -> DataResult<Option<entity_sqlite::youtube_cookies::Model>> {
        const OP: &str = "get_latest_cookies";
        let _timing = DataTiming::new(
            OP.to_string(),
            DataOperationType::Read,
            Some(self.metrics_handler.clone()),
        );
        self.metrics_handler
            .data_access("get_latest_cookies", DataOperationType::Read)
            .await;

        use entity_sqlite::youtube_cookies;
        YoutubeCookies::find()
            .select()
            .order_by(youtube_cookies::Column::EntryId, sea_orm::Order::Desc)
            .limit(1)
            .one(&self.db)
            .await
            .map_err(DataError::GetLatestCookiesDatabaseError)
    }

    pub async fn add_new_cookie(&self, file: Vec<u8>) -> DataResult<()> {
        const OP: &str = "add_new_cookie";
        let _timing = DataTiming::new(
            OP.to_string(),
            DataOperationType::Write,
            Some(self.metrics_handler.clone()),
        );
        self.metrics_handler
            .data_access("add_new_cookie", DataOperationType::Write)
            .await;

        use entity_sqlite::youtube_cookies;
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

impl DataManager {
    /// Add autocomplete entry
    pub fn add_autocomplete(&mut self, key: String, value: String) {
        self.autocomplete_cache
            .lock()
            .expect("is lock poisoned?")
            .insert(key, value)
            .expect("entries should not be too large");
    }

    /// Get the autocomplete entry
    pub fn get_autocomplete(&mut self, key: String) -> Option<String> {
        self.autocomplete_cache
            .lock()
            .expect("is lock poisoned")
            .get(&key)
            .cloned()
    }
}

pub mod error {
    use sea_orm::DbErr;

    use crate::error::ErrorName;

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
        #[error("Database error while getting user command stats: {0}")]
        FindSingleUsersSingleCommandCallError(DbErr),
        #[error("Database error in operation {operation}: {error}")]
        DatabaseError { operation: String, error: DbErr },
        #[error("This sound is already present in the database for the user {user_id}. OP: {sound_name}")]
        DuplicateSoundError {
            sound_name: String,
            user_id: poise::serenity_prelude::UserId,
        },
        #[error("Not found in database. Input error? : {0}")]
        NotFound(String),
        #[error(transparent)]
        BincodeDecodeError(#[from] bincode::error::DecodeError),
        #[error(transparent)]
        BincodeEncodeError(#[from] bincode::error::EncodeError),
    }

    impl ErrorName for DataError {
        fn name(&self) -> String {
            let name = match self {
                DataError::DatabaseConnectionError { .. } => "database_connection_error",
                DataError::MigrationError { .. } => "migration_error",
                DataError::IncrementCommandCounterError { .. } => "increment_command_counter_error",
                DataError::LogCommandCallError { .. } => "log_command_call_error",
                DataError::FindAllowedUserError { .. } => "find_user_allowed_error",
                DataError::FindCommandRolesAllowedError { .. } => {
                    "find_command_roles_allowed_error"
                }
                DataError::FindCategoryRolesAllowedDatabaseError(..) => {
                    "find_category_roles_allowed_database_error"
                }
                DataError::NewCommandRoleRestrictionDatabaseError(..) => {
                    "new_command_role_restriction_database_error"
                }
                DataError::NewCommandRoleRestrictionDuplicate => {
                    "new_command_role_restriction_duplicate"
                }
                DataError::NewCategoryRoleRestrictionDatabaseError(..) => {
                    "new_category_role_restriction_database_error"
                }
                DataError::NewCategoryRoleRestrictionDuplicate => {
                    "new_category_role_restriction_duplicate"
                }
                DataError::NewCommandAllowedUserDuplicate => "new_command_allowed_user_duplicate",
                DataError::NewCommandAllowedUserDatabaseError(..) => {
                    "new_command_allowed_user_database_error"
                }
                DataError::FindAllAllowedUserDatabaseError(..) => {
                    "find_all_allowed_user_database_error"
                }
                DataError::Find5CommandCallLogDatabaseError(..) => {
                    "find_5_command_call_log_database_error"
                }
                DataError::FindUserAllTimeCommandStatsDatabaseError(..) => {
                    "find_user_all_time_command_stats_database_error"
                }
                DataError::GetLatestCookiesDatabaseError(..) => "get_latest_cookies_database_error",
                DataError::AddNewCookieDatabaseError(..) => "add_new_cookie_database_error",
                DataError::FindSingleUsersSingleCommandCallError(..) => {
                    "find_single_user_single_all_time_command_stats"
                }
                DataError::DatabaseError { operation, .. } => operation,
                DataError::DuplicateSoundError { .. } => "duplicate_sound_error",
                DataError::NotFound(_) => "not_found",
                DataError::BincodeDecodeError(..) => "bincode_decode_error",
                DataError::BincodeEncodeError(..) => "bincode_encode_error",
            };
            format!("data::{name}")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::*;

    async fn get_data_manager() -> DataManager {
        let url = "sqlite::memory:";
        DataManager::new(url, Metrics::default()).await.unwrap()
    }

    async fn simulate_command_call(dm: &mut DataManager, count: i64) {
        for _ in 0..count {
            dm.log_command_call(GUILD_ID_1, &USER_ID_1, COMMAND_1.to_string())
                .await
                .unwrap();
        }
    }

    #[tokio::test]
    async fn log_command_call() {
        let mut manager = get_data_manager().await;

        simulate_command_call(&mut manager, 10).await;
    }

    #[tokio::test]
    async fn increment_command_counter() {
        let mut manager = get_data_manager().await;

        manager
            .increment_command_counter(GUILD_ID_1, &USER_ID_1, COMMAND_1.to_string())
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn find5_command_log() {
        let mut manager = get_data_manager().await;
        simulate_command_call(&mut manager, 10).await;

        let res = manager.find5_command_log().await.unwrap();
        assert!(res.len() == 5);
    }

    #[tokio::test]
    async fn find_single_user_single_all_time_command_stats() {
        let mut manager = get_data_manager().await;
        simulate_command_call(&mut manager, 10).await;

        let res = manager
            .find_single_user_single_all_time_command_stats(GUILD_ID_1, USER_ID_1.get(), COMMAND_1)
            .await
            .unwrap();

        assert!(res.unwrap().count == 10);
    }

    #[tokio::test]
    async fn find_all_users_single_command_call() {
        let mut manager = get_data_manager().await;
        simulate_command_call(&mut manager, 10).await;

        let res = manager
            .find_all_users_single_command_call(GUILD_ID_1, COMMAND_1.to_string())
            .await
            .unwrap();

        // TODO: test with many users
        assert!(res.len() == 1);
    }

    // TODO: test cookies
}
