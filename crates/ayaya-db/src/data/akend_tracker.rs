//! Track Endfield pulls
//!

use std::sync::Arc;

use ayaya_core::metrics::{DataOperationType, MetricsSink};
use sea_orm::{ActiveValue, prelude::*};
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

        if let Some(_) = user {
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
}
