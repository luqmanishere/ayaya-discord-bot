//! Track WuWa pulls

use entity_sqlite::prelude::*;
use sea_orm::{ActiveValue, QuerySelect, prelude::*};

use crate::{
    data::{error::DataError, utils::DataTiming},
    metrics::{DataOperationType, Metrics},
};

use super::DataResult;

#[derive(Debug, Clone)]
pub struct WuwaPullsManager {
    db: DatabaseConnection,
    metrics_handler: Metrics,
}

impl WuwaPullsManager {
    /// Create a new instance of [`Self`]
    pub fn new(pulls_db: DatabaseConnection, metrics_handler: Metrics) -> Self {
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

        use entity_sqlite::wuwa_user;
        let user = WuwaUser::find()
            .filter(wuwa_user::Column::UserId.eq(user_id))
            .filter(wuwa_user::Column::WuwaUserId.eq(wuwa_user_id))
            .one(&self.db)
            .await
            .map_err(|error| DataError::DatabaseError {
                operation: OP.to_string(),
                error,
            })?;

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
            .map_err(|error| DataError::DatabaseError {
                operation: OP.to_string(),
                error,
            })?;
        }

        Ok(())
    }

    pub async fn get_wuwa_user_from_user_id(
        &self,
        user_id: u64,
    ) -> DataResult<Vec<entity_sqlite::wuwa_user::Model>> {
        const OP: &str = "get_wuwa_user_from_user_id";
        self.metrics_handler
            .data_access(OP, DataOperationType::Read)
            .await;
        let _timing = DataTiming::new(
            OP.to_string(),
            DataOperationType::Read,
            Some(self.metrics_handler.clone()),
        );

        use entity_sqlite::wuwa_user;
        let user = WuwaUser::find()
            .filter(wuwa_user::Column::UserId.eq(user_id))
            .all(&self.db)
            .await
            .map_err(|error| DataError::DatabaseError {
                operation: OP.to_string(),
                error,
            })?;
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

        use entity_sqlite::wuwa_user;
        let user = WuwaUser::find()
            .filter(wuwa_user::Column::WuwaUserId.eq(wuwa_user_id))
            .one(&self.db)
            .await
            .map_err(|error| DataError::DatabaseError {
                operation: OP.to_string(),
                error,
            })?;
        Ok(user.map(|user| user.user_id as u64))
    }

    pub async fn insert_wuwa_pulls(
        &self,
        wuwa_user_id: u64,
        pulls: Vec<crate::tracker::ParsedWuwaPull>,
    ) -> DataResult<usize> {
        const OP: &str = "insert_wuwa_pull";
        self.metrics_handler
            .data_access(OP, DataOperationType::Write)
            .await;
        let _timing = DataTiming::new(
            OP.to_string(),
            DataOperationType::Write,
            Some(self.metrics_handler.clone()),
        );

        use entity_sqlite::wuwa_pull;
        use std::collections::HashSet;

        // Collect all unique timestamps from incoming pulls
        let incoming_timestamps: HashSet<_> = pulls
            .iter()
            .map(|pull| pull.time.assume_offset(time::UtcOffset::UTC))
            .collect();

        // Check which timestamps already exist in the database for this user
        let existing_timestamps: HashSet<_> = WuwaPull::find()
            .filter(wuwa_pull::Column::WuwaUserId.eq(wuwa_user_id as i32))
            .filter(wuwa_pull::Column::Time.is_in(incoming_timestamps.clone()))
            .select_only()
            .column(wuwa_pull::Column::Time)
            .into_tuple::<(TimeDateTimeWithTimeZone,)>()
            .all(&self.db)
            .await
            .map_err(|error| DataError::DatabaseError {
                operation: OP.to_string(),
                error,
            })?
            .into_iter()
            .map(|(timestamp,)| timestamp)
            .collect();

        // Filter out pulls with timestamps that already exist
        let new_pulls: Vec<_> = pulls
            .into_iter()
            .filter(|pull| {
                let timestamp = pull.time.assume_offset(time::UtcOffset::UTC);
                !existing_timestamps.contains(&timestamp)
            })
            .collect();

        let new_pulls_len = new_pulls.len();
        if new_pulls.is_empty() {
            return Ok(0);
        }

        // Process new pulls and handle resource insertion
        let mut pull_models = Vec::new();
        for pull in new_pulls {
            // Try to insert resource if it doesn't exist (ignore duplicate errors)
            let _ = self
                .insert_resource(
                    pull.resource_id as i32,
                    pull.name.clone(),
                    pull.resource_type.to_string(),
                    "".to_string(), // TODO: Add actual portrait path when available
                )
                .await;

            let pull_model = wuwa_pull::ActiveModel {
                id: ActiveValue::Set(uuid::Uuid::new_v4()),
                wuwa_user_id: ActiveValue::Set(wuwa_user_id as i32),
                pull_type: ActiveValue::Set(pull.card_pool_type as i32),
                resource_id: ActiveValue::Set(pull.resource_id as i32),
                quality_level: ActiveValue::Set(pull.quality_level as i32),
                count: ActiveValue::Set(pull.count as i32),
                time: ActiveValue::Set(pull.time.assume_offset(time::UtcOffset::UTC)),
            };
            pull_models.push(pull_model);
        }

        WuwaPull::insert_many(pull_models)
            .exec(&self.db)
            .await
            .map_err(|error| DataError::DatabaseError {
                operation: OP.to_string(),
                error,
            })?;

        Ok(new_pulls_len)
    }

    pub async fn get_pulls_from_wuwa_id(
        &self,
        wuwa_user_id: u64,
    ) -> DataResult<Vec<entity_sqlite::wuwa_pull::Model>> {
        const OP: &str = "get_pulls_from_wuwa_id";
        self.metrics_handler
            .data_access(OP, DataOperationType::Write)
            .await;
        let _timing = DataTiming::new(
            OP.to_string(),
            DataOperationType::Write,
            Some(self.metrics_handler.clone()),
        );

        use entity_sqlite::wuwa_pull;
        let pulls = WuwaPull::find()
            .filter(wuwa_pull::Column::WuwaUserId.eq(wuwa_user_id))
            .all(&self.db)
            .await
            .map_err(|e| DataError::DatabaseError {
                operation: OP.to_string(),
                error: e,
            })?;
        Ok(pulls)
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

        use entity_sqlite::wuwa_resource;

        // Check if resource already exists
        let existing_resource = WuwaResource::find()
            .filter(wuwa_resource::Column::ResourceId.eq(resource_id))
            .one(&self.db)
            .await
            .map_err(|error| DataError::DatabaseError {
                operation: OP.to_string(),
                error,
            })?;

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
        .map_err(|error| DataError::DatabaseError {
            operation: OP.to_string(),
            error,
        })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{metrics::Metrics, tracker::DeserializeWrapper};
    use sea_orm::{Database, Schema};

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

    fn create_test_pull(resource_id: u64, name: &str) -> crate::tracker::ParsedWuwaPull {
        let test_time = time::PrimitiveDateTime::new(
            time::Date::from_calendar_date(2024, time::Month::January, 15).unwrap(),
            time::Time::from_hms(12, 30, 45).unwrap(),
        );

        crate::tracker::ParsedWuwaPull {
            card_pool_type: crate::tracker::CardPoolType::EventCharacterConvene,
            resource_id,
            quality_level: 5,
            resource_type: crate::tracker::ResourceType::Resonator,
            name: name.to_string(),
            count: 1,
            time: test_time,
        }
    }

    async fn setup_user_and_manager() -> (WuwaPullsManager, u64, u64) {
        let db = setup_test_db().await;
        let metrics = Metrics::new();
        let manager = WuwaPullsManager::new(db, metrics);

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
        let metrics = Metrics::new();
        let manager = WuwaPullsManager::new(db, metrics);

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
    async fn test_pull_insertion_and_deduplication() {
        let (manager, _user_id, wuwa_user_id) = setup_user_and_manager().await;

        let pull = create_test_pull(1001, "Test Resonator");

        // Test first insertion
        let result1 = manager
            .insert_wuwa_pulls(wuwa_user_id, vec![pull.clone()])
            .await
            .unwrap();
        assert_eq!(result1, 1); // One new record

        // Test duplicate insertion (should be filtered out by timestamp)
        let result2 = manager
            .insert_wuwa_pulls(wuwa_user_id, vec![pull])
            .await
            .unwrap();
        assert_eq!(result2, 0); // No new records due to timestamp filtering
    }

    #[tokio::test]
    async fn test_resource_management() {
        let (manager, _user_id, wuwa_user_id) = setup_user_and_manager().await;

        // Test automatic resource creation via pull insertion
        let pull = create_test_pull(1001, "Test Resonator");
        let inserted_count = manager
            .insert_wuwa_pulls(wuwa_user_id, vec![pull])
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
            &std::fs::read_to_string("../dev/wuwa_model_type_1.json").unwrap(),
        )
        .unwrap();

        assert!(pulls.iter().find(|e| e.name == "Carlotta").is_some());

        let inserted_count = manager
            .insert_wuwa_pulls(wuwa_user_id, pulls)
            .await
            .unwrap();
        assert!(inserted_count > 0);
    }
}
