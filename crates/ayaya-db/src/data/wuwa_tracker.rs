//! Track WuWa pulls

use crate::entity::prelude::*;
use sea_orm::{ActiveValue, DbErr, prelude::*};
use snafu::ResultExt;

use std::sync::Arc;

use crate::data::utils::DataTiming;
use crate::error::{DataError, DatabaseSnafu};
use ayaya_core::metrics::{DataOperationType, MetricsSink};
use ayaya_core::tracker::{ImportBoundary, wuwa::WuwaPullDto};

use super::DataResult;

#[derive(Clone)]
pub struct WuwaPullsManager {
    db: DatabaseConnection,
    metrics_handler: Arc<dyn MetricsSink>,
}

impl WuwaPullsManager {
    /// Create a new instance of [`Self`]
    pub fn new(pulls_db: DatabaseConnection, metrics_handler: Arc<dyn MetricsSink>) -> Self {
        Self {
            db: pulls_db,
            metrics_handler,
        }
    }

    /// Insert a new wuwa user into the database
    pub async fn insert_wuwa_user(&self, user_id: u64, wuwa_user_id: u64) -> DataResult<()> {
        const OP: &str = "insert_wuwa_user";
        self.metrics_handler
            .data_access(OP, DataOperationType::Write)
            .await;
        let _timing = DataTiming::new(
            OP.to_string(),
            DataOperationType::Write,
            Some(self.metrics_handler.clone()),
        );

        use crate::entity::wuwa_user;
        let user = WuwaUser::find()
            .filter(wuwa_user::Column::UserId.eq(user_id))
            .filter(wuwa_user::Column::WuwaUserId.eq(wuwa_user_id))
            .one(&self.db)
            .await
            .context(DatabaseSnafu { operation: OP })?;

        if let Some(_user) = user {
            return Err(DataError::DuplicateEntry {
                object: "wuwa_user".to_string(),
            });
        } else {
            wuwa_user::ActiveModel {
                user_id: ActiveValue::Set(user_id as i64),
                wuwa_user_id: ActiveValue::Set(wuwa_user_id as i32),
            }
            .insert(&self.db)
            .await
            .context(DatabaseSnafu { operation: OP })?;
        }

        Ok(())
    }

    pub async fn get_wuwa_user_from_user_id(
        &self,
        user_id: u64,
    ) -> DataResult<Vec<crate::entity::wuwa_user::Model>> {
        const OP: &str = "get_wuwa_user_from_user_id";
        self.metrics_handler
            .data_access(OP, DataOperationType::Read)
            .await;
        let _timing = DataTiming::new(
            OP.to_string(),
            DataOperationType::Read,
            Some(self.metrics_handler.clone()),
        );

        use crate::entity::wuwa_user;
        let user = WuwaUser::find()
            .filter(wuwa_user::Column::UserId.eq(user_id))
            .all(&self.db)
            .await
            .context(DatabaseSnafu { operation: OP })?;
        Ok(user)
    }

    pub async fn get_user_id_from_wuwa_user(&self, wuwa_user_id: u64) -> DataResult<Option<u64>> {
        // yes my naming sucks
        const OP: &str = "get_user_id_from_wuwa_user";
        self.metrics_handler
            .data_access(OP, DataOperationType::Read)
            .await;
        let _timing = DataTiming::new(
            OP.to_string(),
            DataOperationType::Read,
            Some(self.metrics_handler.clone()),
        );

        use crate::entity::wuwa_user;
        let user = WuwaUser::find()
            .filter(wuwa_user::Column::WuwaUserId.eq(wuwa_user_id))
            .one(&self.db)
            .await
            .context(DatabaseSnafu { operation: OP })?;
        Ok(user.map(|user| user.user_id as u64))
    }

    pub async fn insert_wuwa_pull_records(
        &self,
        wuwa_user_id: u64,
        pulls: Vec<WuwaPullDto>,
    ) -> DataResult<usize> {
        const OP: &str = "insert_wuwa_pull_records";
        self.metrics_handler
            .data_access(OP, DataOperationType::Write)
            .await;
        let _timing = DataTiming::new(
            OP.to_string(),
            DataOperationType::Write,
            Some(self.metrics_handler.clone()),
        );

        use crate::entity::wuwa_pull;

        if pulls.is_empty() {
            return Ok(0);
        }

        let mut pull_models = Vec::new();
        for pull in pulls {
            let resource_id =
                i32::try_from(pull.resource_id).map_err(|_| DataError::DatabaseError {
                    operation: OP.to_string(),
                    source: DbErr::Custom("resource_id out of range".to_string()),
                })?;

            let pull_type =
                wuwa_pool_type_to_i32(&pull.pool_id).ok_or_else(|| DataError::DatabaseError {
                    operation: OP.to_string(),
                    source: DbErr::Custom("unknown pool_id".to_string()),
                })?;

            let _ = self
                .insert_resource(
                    resource_id,
                    pull.resource_name.clone(),
                    pull.resource_type.clone(),
                    "".to_string(), // TODO: Add actual portrait path when available
                )
                .await;

            let pull_model = wuwa_pull::ActiveModel {
                id: ActiveValue::Set(uuid::Uuid::new_v4()),
                wuwa_user_id: ActiveValue::Set(wuwa_user_id as i32),
                pull_type: ActiveValue::Set(pull_type),
                resource_id: ActiveValue::Set(resource_id),
                quality_level: ActiveValue::Set(pull.quality as i32),
                count: ActiveValue::Set(pull.count as i32),
                time: ActiveValue::Set(pull.time),
            };
            pull_models.push(pull_model);
        }

        let pull_models_len = pull_models.len();
        WuwaPull::insert_many(pull_models)
            .exec(&self.db)
            .await
            .context(DatabaseSnafu { operation: OP })?;

        Ok(pull_models_len)
    }

    pub async fn get_pulls_from_wuwa_id(
        &self,
        wuwa_user_id: u64,
    ) -> DataResult<Vec<crate::entity::wuwa_pull::Model>> {
        const OP: &str = "get_pulls_from_wuwa_id";
        self.metrics_handler
            .data_access(OP, DataOperationType::Write)
            .await;
        let _timing = DataTiming::new(
            OP.to_string(),
            DataOperationType::Write,
            Some(self.metrics_handler.clone()),
        );

        use crate::entity::wuwa_pull;
        let pulls = WuwaPull::find()
            .filter(wuwa_pull::Column::WuwaUserId.eq(wuwa_user_id))
            .all(&self.db)
            .await
            .context(DatabaseSnafu { operation: OP })?;
        Ok(pulls)
    }

    pub async fn get_wuwa_import_state(
        &self,
        wuwa_user_id: u64,
        pool_id: &str,
    ) -> DataResult<Option<ImportBoundary>> {
        const OP: &str = "get_wuwa_import_state";
        self.metrics_handler
            .data_access(OP, DataOperationType::Read)
            .await;
        let _timing = DataTiming::new(
            OP.to_string(),
            DataOperationType::Read,
            Some(self.metrics_handler.clone()),
        );

        use crate::entity::wuwa_import_state;

        let state = WuwaImportState::find()
            .filter(wuwa_import_state::Column::WuwaUserId.eq(wuwa_user_id as i32))
            .filter(wuwa_import_state::Column::PoolId.eq(pool_id))
            .one(&self.db)
            .await
            .context(DatabaseSnafu { operation: OP })?;

        Ok(state.map(|state| ImportBoundary {
            time: state.last_time,
            count_at_time: state.count_at_time as usize,
        }))
    }

    pub async fn upsert_wuwa_import_state(
        &self,
        wuwa_user_id: u64,
        pool_id: &str,
        boundary: ImportBoundary,
    ) -> DataResult<()> {
        const OP: &str = "upsert_wuwa_import_state";
        self.metrics_handler
            .data_access(OP, DataOperationType::Write)
            .await;
        let _timing = DataTiming::new(
            OP.to_string(),
            DataOperationType::Write,
            Some(self.metrics_handler.clone()),
        );

        use crate::entity::wuwa_import_state;

        let existing = WuwaImportState::find()
            .filter(wuwa_import_state::Column::WuwaUserId.eq(wuwa_user_id as i32))
            .filter(wuwa_import_state::Column::PoolId.eq(pool_id))
            .one(&self.db)
            .await
            .context(DatabaseSnafu { operation: OP })?;

        if let Some(existing) = existing {
            let mut model: wuwa_import_state::ActiveModel = existing.into();
            model.last_time = ActiveValue::Set(boundary.time);
            model.count_at_time = ActiveValue::Set(boundary.count_at_time as i32);
            model
                .update(&self.db)
                .await
                .context(DatabaseSnafu { operation: OP })?;
        } else {
            wuwa_import_state::ActiveModel {
                wuwa_user_id: ActiveValue::Set(wuwa_user_id as i32),
                pool_id: ActiveValue::Set(pool_id.to_string()),
                last_time: ActiveValue::Set(boundary.time),
                count_at_time: ActiveValue::Set(boundary.count_at_time as i32),
            }
            .insert(&self.db)
            .await
            .context(DatabaseSnafu { operation: OP })?;
        }

        Ok(())
    }

    pub async fn insert_resource(
        &self,
        resource_id: i32,
        resource_name: String,
        resource_type: String,
        resource_portrait_path: String,
    ) -> DataResult<()> {
        const OP: &str = "insert_resource";
        self.metrics_handler
            .data_access(OP, DataOperationType::Write)
            .await;
        let _timing = DataTiming::new(
            OP.to_string(),
            DataOperationType::Write,
            Some(self.metrics_handler.clone()),
        );

        use crate::entity::wuwa_resource;

        // Check if resource already exists
        let existing_resource = WuwaResource::find()
            .filter(wuwa_resource::Column::ResourceId.eq(resource_id))
            .one(&self.db)
            .await
            .context(DatabaseSnafu { operation: OP })?;

        if existing_resource.is_some() {
            return Err(DataError::DuplicateEntry {
                object: "wuwa_resource".to_string(),
            });
        }

        // Insert new resource
        wuwa_resource::ActiveModel {
            resource_id: ActiveValue::Set(resource_id),
            resource_name: ActiveValue::Set(resource_name),
            resource_type: ActiveValue::Set(resource_type),
            resource_portrait_path: ActiveValue::Set(resource_portrait_path),
        }
        .insert(&self.db)
        .await
        .context(DatabaseSnafu { operation: OP })?;

        Ok(())
    }
}

fn wuwa_pool_type_to_i32(pool_id: &str) -> Option<i32> {
    match pool_id {
        "Resonators Accurate Modulation" => Some(0),
        "Resonators Accurate Modulation - 2" => Some(1),
        "Weapons Accurate Modulation" => Some(2),
        "Full-Range Modualtion" => Some(3),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ayaya_core::metrics::NoopMetrics;
    use sea_orm::{Database, Schema};
    use serde::Deserialize;

    async fn setup_test_db() -> DatabaseConnection {
        let db = Database::connect("sqlite::memory:").await.unwrap();

        let builder = db.get_database_backend();
        let schema = Schema::new(builder);

        // Create tables
        let stmt = builder.build(schema.create_table_from_entity(WuwaUser).if_not_exists());
        db.execute(stmt).await.unwrap();

        let stmt = builder.build(schema.create_table_from_entity(WuwaPull).if_not_exists());
        db.execute(stmt).await.unwrap();

        let stmt = builder.build(
            schema
                .create_table_from_entity(WuwaResource)
                .if_not_exists(),
        );
        db.execute(stmt).await.unwrap();

        db
    }

    fn create_test_pull(resource_id: u64, name: &str) -> WuwaPullDto {
        let test_time = time::PrimitiveDateTime::new(
            time::Date::from_calendar_date(2024, time::Month::January, 15).unwrap(),
            time::Time::from_hms(12, 30, 45).unwrap(),
        );

        WuwaPullDto {
            pool_id: "Resonators Accurate Modulation".to_string(),
            resource_id: resource_id as i64,
            resource_name: name.to_string(),
            resource_type: "Resonator".to_string(),
            quality: 5,
            count: 1,
            time: test_time.assume_offset(time::UtcOffset::UTC),
        }
    }

    async fn setup_user_and_manager() -> (WuwaPullsManager, u64, u64) {
        let db = setup_test_db().await;
        let manager = WuwaPullsManager::new(db, Arc::new(NoopMetrics));

        let user_id = 12345u64;
        let wuwa_user_id = 67890u64;

        manager
            .insert_wuwa_user(user_id, wuwa_user_id)
            .await
            .unwrap();

        (manager, user_id, wuwa_user_id)
    }

    macro_rules! assert_duplicate_error {
        ($result:expr, $object:expr) => {
            match $result.unwrap_err() {
                DataError::DuplicateEntry { object } => assert_eq!(object, $object),
                _ => panic!("Expected DuplicateEntry error for {}", $object),
            }
        };
    }

    #[tokio::test]
    async fn test_user_management() {
        let db = setup_test_db().await;
        let manager = WuwaPullsManager::new(db, Arc::new(NoopMetrics));

        let user_id = 12345u64;
        let wuwa_user_id = 67890u64;

        // Test user insertion
        manager
            .insert_wuwa_user(user_id, wuwa_user_id)
            .await
            .unwrap();

        // Test user retrieval by user_id
        let retrieved_user = manager.get_wuwa_user_from_user_id(user_id).await.unwrap();
        assert!(!retrieved_user.is_empty());
        let user = retrieved_user.first().unwrap();
        assert_eq!(user.user_id, user_id as i64);
        assert_eq!(user.wuwa_user_id, wuwa_user_id as i32);

        // Test user retrieval by wuwa_user_id
        let retrieved_user_id = manager
            .get_user_id_from_wuwa_user(wuwa_user_id)
            .await
            .unwrap();
        assert!(retrieved_user_id.is_some());
        assert_eq!(retrieved_user_id.unwrap(), user_id);

        // Test duplicate user insertion fails
        let result = manager.insert_wuwa_user(user_id, wuwa_user_id).await;
        assert!(result.is_err());
        assert_duplicate_error!(result, "wuwa_user");
    }

    #[tokio::test]
    async fn test_pull_insertion() {
        let (manager, _user_id, wuwa_user_id) = setup_user_and_manager().await;

        let pull = create_test_pull(1001, "Test Resonator");

        // Test first insertion
        let result1 = manager
            .insert_wuwa_pull_records(wuwa_user_id, vec![pull.clone()])
            .await
            .unwrap();
        assert_eq!(result1, 1); // One new record

        // Test duplicate insertion (still inserts since dedup is not handled here)
        let result2 = manager
            .insert_wuwa_pull_records(wuwa_user_id, vec![pull])
            .await
            .unwrap();
        assert_eq!(result2, 1);
    }

    #[tokio::test]
    async fn test_resource_management() {
        let (manager, _user_id, wuwa_user_id) = setup_user_and_manager().await;

        // Test automatic resource creation via pull insertion
        let pull = create_test_pull(1001, "Test Resonator");
        let inserted_count = manager
            .insert_wuwa_pull_records(wuwa_user_id, vec![pull])
            .await
            .unwrap();
        assert_eq!(inserted_count, 1);

        // Test duplicate resource insertion fails
        let result = manager
            .insert_resource(
                1001,
                "Test Resonator".to_string(),
                "Resonator".to_string(),
                "".to_string(),
            )
            .await;

        assert!(result.is_err());
        assert_duplicate_error!(result, "wuwa_resource");

        // Test manual resource insertion
        let result = manager
            .insert_resource(
                1002,
                "Another Resonator".to_string(),
                "Resonator".to_string(),
                "/path/to/portrait.png".to_string(),
            )
            .await;
        assert!(result.is_ok());

        // Test duplicate manual resource insertion fails
        let result = manager
            .insert_resource(
                1002,
                "Another Resonator".to_string(),
                "Resonator".to_string(),
                "/path/to/portrait.png".to_string(),
            )
            .await;
        assert_duplicate_error!(result, "wuwa_resource");
    }

    #[tokio::test]
    async fn test_real_data_integration() {
        let (manager, _user_id, wuwa_user_id) = setup_user_and_manager().await;

        let DeserializeWrapper { data: pulls } = serde_json::from_str(
            &std::fs::read_to_string("../../dev/wuwa_model_type_1.json").unwrap(),
        )
        .unwrap();

        assert!(pulls.iter().find(|e| e.name == "Carlotta").is_some());

        let records = pulls
            .into_iter()
            .map(|pull| WuwaPullDto {
                pool_id: pull.card_pool_type,
                resource_id: pull.resource_id as i64,
                resource_name: pull.name,
                resource_type: pull.resource_type,
                quality: pull.quality_level as i32,
                count: pull.count as i32,
                time: pull.time.assume_offset(time::UtcOffset::UTC),
            })
            .collect();

        let inserted_count = manager
            .insert_wuwa_pull_records(wuwa_user_id, records)
            .await
            .unwrap();
        assert!(inserted_count > 0);
    }

    #[derive(Debug, Deserialize)]
    struct DeserializeWrapper {
        data: Vec<RawPull>,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct RawPull {
        card_pool_type: String,
        resource_id: u64,
        quality_level: u64,
        resource_type: String,
        name: String,
        count: u64,
        #[serde(deserialize_with = "deserialize_time")]
        time: time::PrimitiveDateTime,
    }

    fn deserialize_time<'de, D>(deserializer: D) -> Result<time::PrimitiveDateTime, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        static FORMAT: &[time::format_description::BorrowedFormatItem] = time::macros::format_description!(
            "[year]-[month repr:numerical]-[day] [hour repr:24]:[minute]:[second]"
        );

        let s: &str = Deserialize::deserialize(deserializer)?;
        let ti = time::PrimitiveDateTime::parse(s, FORMAT).map_err(serde::de::Error::custom)?;
        Ok(ti)
    }
}
