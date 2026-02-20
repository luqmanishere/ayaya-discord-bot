//! Track Endfield pulls
//!

use std::sync::Arc;

use ayaya_core::{
    metrics::{DataOperationType, MetricsSink},
    tracker::{
        ImportBoundary,
        akend::{AkEndCharPullDto, AkEndPullDto, AkEndWeapPullDto},
    },
};
use sea_orm::{ActiveValue, IntoActiveModel, prelude::*};
use snafu::ResultExt;

use crate::{
    data::{DataResult, utils::DataTiming},
    error::DatabaseSnafu,
};
use crate::{entity::prelude::*, error::DataError};

#[derive(Clone)]
pub struct AkEndTracker {
    pub db: DatabaseConnection,
    metrics_handler: Arc<dyn MetricsSink>,
}

impl AkEndTracker {
    pub fn new(db: DatabaseConnection, metrics_handler: Arc<dyn MetricsSink>) -> Self {
        Self {
            db,
            metrics_handler,
        }
    }

    /// Insert a new endfield user registration
    pub async fn insert_akend_user(
        &self,
        user_id: u64,
        akend_user_id: i64,
        akend_user_description: &str,
    ) -> DataResult<()> {
        const OP: &str = "insert_akend_user";
        self.metrics_handler
            .data_access(OP, DataOperationType::Write)
            .await;
        let _timing = DataTiming::new(
            OP.to_string(),
            DataOperationType::Write,
            Some(self.metrics_handler.clone()),
        );

        // TODO: we need to check if account already registered to another
        use crate::entity::ak_end_user;
        let user = AkEndUser::find()
            .filter(ak_end_user::Column::UserId.eq(user_id))
            .filter(ak_end_user::Column::AkEndUserId.eq(akend_user_id))
            .one(&self.db)
            .await
            .context(DatabaseSnafu { operation: OP })?;

        if user.is_some() {
            return Err(DataError::DuplicateEntry {
                object: "akend_user".to_string(),
            });
        } else {
            ak_end_user::ActiveModel {
                user_id: ActiveValue::Set(user_id as i64),
                ak_end_user_id: ActiveValue::Set(akend_user_id),
                user_desc: ActiveValue::Set(akend_user_description.to_string()),
            }
            .insert(&self.db)
            .await
            .context(DatabaseSnafu { operation: OP })?;
        }
        Ok(())
    }

    /// Get all endfield users registered by a discord user id
    pub async fn get_akend_users_by_user_id(
        &self,
        user_id: u64,
    ) -> DataResult<Vec<crate::entity::ak_end_user::Model>> {
        const OP: &str = "get_akend_users_by_user_id";
        self.metrics_handler
            .data_access(OP, DataOperationType::Read)
            .await;
        let _timing = DataTiming::new(
            OP.to_string(),
            DataOperationType::Read,
            Some(self.metrics_handler.clone()),
        );

        use crate::entity::ak_end_user;
        let users = AkEndUser::find()
            .filter(ak_end_user::Column::UserId.eq(user_id as i64))
            .all(&self.db)
            .await
            .context(DatabaseSnafu { operation: OP })?;

        Ok(users)
    }

    /// Get the import state for a given pool id and UID
    pub async fn get_akend_import_state(
        &self,
        akend_user_id: i64,
        pool_id: &str,
    ) -> DataResult<Option<ImportBoundary>> {
        const OP: &str = "get_akend_import_state";
        self.metrics_handler
            .data_access(OP, DataOperationType::Read)
            .await;
        let _timing = DataTiming::new(
            OP.to_string(),
            DataOperationType::Read,
            Some(self.metrics_handler.clone()),
        );

        use crate::entity::ak_end_import_state;
        let state = AkEndImportState::find()
            .filter(ak_end_import_state::Column::AkEndUserId.eq(akend_user_id))
            .filter(ak_end_import_state::Column::PoolId.eq(pool_id))
            .one(&self.db)
            .await
            .context(DatabaseSnafu { operation: OP })?;

        Ok(state.map(|state| ImportBoundary {
            time: state.last_time,
            count_at_time: state.count_at_time as usize,
        }))
    }

    /// Upsert the current state of the import for a given pool id and UID
    pub async fn upsert_akend_import_state(
        &self,
        akend_user_id: i64,
        pool_id: &str,
        boundary: ImportBoundary,
    ) -> DataResult<()> {
        const OP: &str = "upsert_akend_import_state";
        self.metrics_handler
            .data_access(OP, DataOperationType::Write)
            .await;
        let _timing = DataTiming::new(
            OP.to_string(),
            DataOperationType::Write,
            Some(self.metrics_handler.clone()),
        );

        use crate::entity::ak_end_import_state;
        let state = AkEndImportState::find()
            .filter(ak_end_import_state::Column::AkEndUserId.eq(akend_user_id))
            .filter(ak_end_import_state::Column::PoolId.eq(pool_id))
            .one(&self.db)
            .await
            .context(DatabaseSnafu { operation: OP })?;

        if let Some(state) = state {
            let mut model = state.into_active_model();
            model.last_time = ActiveValue::Set(boundary.time);
            model.count_at_time = ActiveValue::Set(boundary.count_at_time as i32);
            model
                .update(&self.db)
                .await
                .context(DatabaseSnafu { operation: OP })?;
        } else {
            ak_end_import_state::ActiveModel {
                ak_end_user_id: ActiveValue::Set(akend_user_id),
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

    pub async fn insert_akend_char_pull_records(
        &self,
        user_id: u64,
        akend_user_id: i64,
        records: Vec<AkEndCharPullDto>,
    ) -> DataResult<usize> {
        const OP: &str = "insert_akend_char_pull_records";
        self.metrics_handler
            .data_access(OP, DataOperationType::Write)
            .await;
        let _timing = DataTiming::new(
            OP.to_string(),
            DataOperationType::Write,
            Some(self.metrics_handler.clone()),
        );

        use crate::entity::ak_end_char_pull;

        if records.is_empty() {
            return Ok(0);
        }

        let mut pull_models = Vec::new();
        for record in records {
            let pull_model = ak_end_char_pull::ActiveModel {
                id: ActiveValue::Set(uuid::Uuid::new_v4()),
                user_id: ActiveValue::Set(user_id as i64),
                ak_end_user_id: ActiveValue::Set(akend_user_id),
                pool_type: ActiveValue::Set(record.pool_type),
                pool_id: ActiveValue::Set(record.pool_id),
                pool_name: ActiveValue::Set(record.pool_name),
                char_id: ActiveValue::Set(record.char_id),
                char_name: ActiveValue::Set(record.char_name),
                rarity: ActiveValue::Set(record.rarity),
                is_free: ActiveValue::Set(record.is_free),
                is_new: ActiveValue::Set(record.is_new),
                time: ActiveValue::Set(record.time),
                seq_id: ActiveValue::Set(record.seq_id),
            };
            pull_models.push(pull_model);
        }

        let pull_models_len = pull_models.len();
        AkEndCharPull::insert_many(pull_models)
            .exec(&self.db)
            .await
            .context(DatabaseSnafu { operation: OP })?;

        Ok(pull_models_len)
    }

    pub async fn insert_akend_weap_pull_records(
        &self,
        user_id: u64,
        akend_user_id: i64,
        records: Vec<AkEndWeapPullDto>,
    ) -> DataResult<usize> {
        const OP: &str = "insert_akend_weap_pull_records";
        self.metrics_handler
            .data_access(OP, DataOperationType::Write)
            .await;
        let _timing = DataTiming::new(
            OP.to_string(),
            DataOperationType::Write,
            Some(self.metrics_handler.clone()),
        );

        use crate::entity::ak_end_weap_pull;

        if records.is_empty() {
            return Ok(0);
        }

        let pull_models = records
            .into_iter()
            .map(|record| ak_end_weap_pull::ActiveModel {
                id: ActiveValue::Set(Uuid::new_v4()),
                user_id: ActiveValue::Set(user_id as i64),
                ak_end_user_id: ActiveValue::Set(akend_user_id),
                pool_type: ActiveValue::Set(record.pool_type),
                pool_id: ActiveValue::Set(record.pool_id),
                pool_name: ActiveValue::Set(record.pool_name),
                weapon_id: ActiveValue::Set(record.weapon_id),
                weapon_name: ActiveValue::Set(record.weapon_name),
                weapon_type: ActiveValue::Set(record.weapon_type),
                rarity: ActiveValue::Set(record.rarity),
                is_new: ActiveValue::Set(record.is_new),
                time: ActiveValue::set(record.time),
                seq_id: ActiveValue::Set(record.seq_id),
            })
            .collect::<Vec<_>>();

        let pulls_models_len = pull_models.len();
        AkEndWeapPull::insert_many(pull_models)
            .exec(&self.db)
            .await
            .context(DatabaseSnafu { operation: OP })?;

        Ok(pulls_models_len)
    }

    /// Insert pull records from the Dto, which is then split up to the proper methods
    pub async fn insert_akend_pull_records(
        &self,
        user_id: u64,
        akend_user_id: i64,
        records: Vec<AkEndPullDto>,
    ) -> DataResult<usize> {
        let (char_records, weap_records): (Vec<_>, Vec<_>) = records
            .into_iter()
            .partition(|record| matches!(record, AkEndPullDto::Character(_)));
        let char_records = char_records
            .into_iter()
            .filter_map(|record| {
                if let AkEndPullDto::Character(record) = record {
                    Some(record)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        let weap_records = weap_records
            .into_iter()
            .filter_map(|record| {
                if let AkEndPullDto::Weapon(record) = record {
                    Some(record)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        let count = self
            .insert_akend_char_pull_records(user_id, akend_user_id, char_records)
            .await?
            + self
                .insert_akend_weap_pull_records(user_id, akend_user_id, weap_records)
                .await?;

        Ok(count)
    }

    pub async fn get_all_char_pulls_from_akend_id(
        &self,
        akend_user_id: i64,
    ) -> DataResult<Vec<AkEndCharPullModel>> {
        const OP: &str = "get_pulls_from_akend_id";
        self.metrics_handler
            .data_access(OP, DataOperationType::Write)
            .await;
        let _timing = DataTiming::new(
            OP.to_string(),
            DataOperationType::Write,
            Some(self.metrics_handler.clone()),
        );

        use crate::entity::ak_end_char_pull;
        let pulls = AkEndCharPull::find()
            .filter(ak_end_char_pull::Column::AkEndUserId.eq(akend_user_id))
            .all(&self.db)
            .await
            .context(DatabaseSnafu { operation: OP })?;
        Ok(pulls)
    }
}
