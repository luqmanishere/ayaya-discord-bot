//! Manage database connection and caching
//!
pub mod akend_tracker;
pub mod dashboard;
pub mod permissions;
pub mod sounds;
pub mod stats;
mod utils;
pub mod wuwa_tracker;

use std::sync::{Arc, Mutex};

use crate::error::DataError;
use crate::{data::akend_tracker::AkEndTracker, entity::prelude::*};
use lru_mem::LruCache;
use migration::{Migrator as SqliteMigrator, MigratorTrait};
use permissions::Permissions;
use poise::serenity_prelude as serenity;
use sea_orm::{
    ActiveValue, ConnectOptions, EntityOrSelect, IntoActiveModel, QueryOrder, QuerySelect,
    prelude::*,
};
use sea_orm::{Database, DatabaseConnection};
use snafu::ResultExt;
use sounds::SoundsManager;
use stats::StatsManager;
use time::UtcOffset;
use utils::DataTiming;

use crate::data::wuwa_tracker::WuwaPullsManager;
use crate::error::{
    AddNewCookieDatabaseSnafu, DatabaseConnectionSnafu, Find5CommandCallLogDatabaseSnafu,
    FindSingleUsersSingleCommandCallSnafu, FindUserAllTimeCommandStatsDatabaseSnafu,
    GetLatestCookiesDatabaseSnafu, IncrementCommandCounterSnafu, LogCommandCallSnafu,
    MigrationSnafu,
};
use ayaya_core::metrics::{DataOperationType, MetricsSink};

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
#[derive(Clone)]
pub struct DataManager {
    db: DatabaseConnection, // this is already clone
    metrics_handler: Arc<dyn MetricsSink>,
    permissions: Permissions,
    stats: StatsManager,
    sounds: SoundsManager,
    wuwa_tracker: WuwaPullsManager,
    akend_tracker: AkEndTracker,
    autocomplete_cache: Autocomplete,
}

impl DataManager {
    /// A new instance of the manager
    pub async fn new(db_url: &str, metrics_handler: Arc<dyn MetricsSink>) -> DataResult<Self> {
        let mut connect_options = if cfg!(debug_assertions) && !cfg!(test) {
            ConnectOptions::new("sqlite://dev/stats.sqlite?mode=rwc")
        } else {
            ConnectOptions::new(db_url)
        };
        connect_options.sqlx_logging(false); // disable sqlx logging
        let db: DatabaseConnection = Database::connect(connect_options)
            .await
            .context(DatabaseConnectionSnafu)?;

        SqliteMigrator::up(&db, None)
            .await
            .context(MigrationSnafu)?;

        let permissions = Permissions::new(db.clone(), metrics_handler.clone()).await?;
        let stats = StatsManager::new(db.clone(), metrics_handler.clone());
        let sounds = SoundsManager::new(db.clone(), metrics_handler.clone());
        let wuwa_tracker = WuwaPullsManager::new(db.clone(), metrics_handler.clone());
        let akend_tracker = AkEndTracker::new(db.clone(), metrics_handler.clone());
        Ok(Self {
            db,
            metrics_handler,
            permissions,
            stats,
            sounds,
            wuwa_tracker,
            akend_tracker,
            autocomplete_cache: Arc::new(Mutex::new(LruCache::new(1000 * 1024))),
        })
    }

    pub fn permissions_mut(&mut self) -> &mut Permissions {
        &mut self.permissions
    }

    pub fn stats(&self) -> StatsManager {
        self.stats.clone()
    }

    pub fn sounds(&self) -> SoundsManager {
        self.sounds.clone()
    }

    pub fn wuwa_tracker(&self) -> WuwaPullsManager {
        self.wuwa_tracker.clone()
    }

    pub fn akend_tracker(&self) -> AkEndTracker {
        self.akend_tracker.clone()
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
        let call_log = crate::entity::command_call_log::ActiveModel {
            log_id: sea_orm::ActiveValue::Set(uuid::Uuid::new_v4()),
            server_id: sea_orm::ActiveValue::Set(guild_id as i64),
            user_id: sea_orm::ActiveValue::Set(user_id.get() as i64),
            command: sea_orm::ActiveValue::Set(command_name.clone()),
            command_time_stamp: sea_orm::ActiveValue::Set(now_odt),
        };
        call_log.insert(db).await.context(LogCommandCallSnafu)?;

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
        use crate::entity::user_command_all_time_statistics;
        let user = UserCommandAllTimeStatistics::find()
            .filter(user_command_all_time_statistics::Column::ServerId.eq(guild_id))
            .filter(user_command_all_time_statistics::Column::UserId.eq(user_id.get()))
            .filter(user_command_all_time_statistics::Column::Command.eq(command_name.clone()))
            .one(db)
            .await
            .context(IncrementCommandCounterSnafu)?;

        if let Some(stats) = user {
            let count = stats.count + 1;
            let mut model = stats.into_active_model();
            model.count = ActiveValue::set(count);
            model.save(db).await.context(IncrementCommandCounterSnafu)?;
        } else {
            user_command_all_time_statistics::ActiveModel {
                server_id: sea_orm::ActiveValue::Set(guild_id as i64),
                user_id: sea_orm::ActiveValue::Set(user_id.get() as i64),
                command: sea_orm::ActiveValue::Set(command_name),
                count: sea_orm::ActiveValue::Set(1),
            }
            .insert(db)
            .await
            .context(IncrementCommandCounterSnafu)?;
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
    ) -> DataResult<Vec<crate::entity::command_call_log::Model>> {
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
            .context(Find5CommandCallLogDatabaseSnafu)
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
    ) -> DataResult<Option<crate::entity::user_command_all_time_statistics::Model>> {
        const OP: &str = "find_user_all_time_command_stats";
        let _timing = DataTiming::new(
            OP.to_string(),
            DataOperationType::Read,
            Some(self.metrics_handler.clone()),
        );
        self.metrics_handler
            .data_access("find_user_all_time_command_stats", DataOperationType::Read)
            .await;

        use crate::entity::user_command_all_time_statistics;
        UserCommandAllTimeStatistics::find()
            .filter(user_command_all_time_statistics::Column::ServerId.eq(guild_id))
            .filter(user_command_all_time_statistics::Column::UserId.eq(user_id))
            .filter(user_command_all_time_statistics::Column::Command.eq(command))
            .one(&self.db)
            .await
            .context(FindUserAllTimeCommandStatsDatabaseSnafu)
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
    ) -> DataResult<Vec<crate::entity::user_command_all_time_statistics::Model>> {
        const OP: &str = "rank_users_command_call";
        let _timing = DataTiming::new(
            OP.to_string(),
            DataOperationType::Read,
            Some(self.metrics_handler.clone()),
        );
        self.metrics_handler
            .data_access(OP, DataOperationType::Read)
            .await;

        use crate::entity::user_command_all_time_statistics;
        UserCommandAllTimeStatistics::find()
            .filter(user_command_all_time_statistics::Column::ServerId.eq(guild_id))
            .filter(user_command_all_time_statistics::Column::Command.eq(command_name))
            .all(&self.db)
            .await
            .context(FindSingleUsersSingleCommandCallSnafu)
    }

    pub async fn get_latest_cookies(
        &self,
    ) -> DataResult<Option<crate::entity::youtube_cookies::Model>> {
        const OP: &str = "get_latest_cookies";
        let _timing = DataTiming::new(
            OP.to_string(),
            DataOperationType::Read,
            Some(self.metrics_handler.clone()),
        );
        self.metrics_handler
            .data_access("get_latest_cookies", DataOperationType::Read)
            .await;

        use crate::entity::youtube_cookies;
        YoutubeCookies::find()
            .select()
            .order_by(youtube_cookies::Column::EntryId, sea_orm::Order::Desc)
            .limit(1)
            .one(&self.db)
            .await
            .context(GetLatestCookiesDatabaseSnafu)
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

        use crate::entity::youtube_cookies;
        let _model = youtube_cookies::ActiveModel {
            entry_id: ActiveValue::NotSet,
            date: ActiveValue::Set(time::OffsetDateTime::now_utc()),
            cookies: ActiveValue::Set(file),
        }
        .save(&self.db)
        .await
        .context(AddNewCookieDatabaseSnafu)?;

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

// DataError moved to crate::error

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::*;
    use ayaya_core::metrics::NoopMetrics;

    async fn get_data_manager() -> DataManager {
        let url = "sqlite::memory:";
        DataManager::new(url, Arc::new(NoopMetrics)).await.unwrap()
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
