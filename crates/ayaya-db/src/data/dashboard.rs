use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter,
};
use time::OffsetDateTime;
use uuid::Uuid;

use super::{DataManager, DataResult};
use crate::entity::{dashboard_allowlist, dashboard_tokens};
use crate::error::DataError;

impl DataManager {
    // ==================== Allowlist Methods ====================

    /// Add a user to the dashboard allowlist.
    pub async fn add_to_allowlist(
        &self,
        user_id: i64,
        added_by: i64,
        notes: Option<String>,
    ) -> DataResult<bool> {
        let now = OffsetDateTime::now_utc();

        let allowlist_entry = dashboard_allowlist::ActiveModel {
            user_id: ActiveValue::Set(user_id),
            added_by: ActiveValue::Set(added_by),
            added_at: ActiveValue::Set(now),
            notes: ActiveValue::Set(notes),
        };

        match allowlist_entry.insert(&self.db).await {
            Ok(_) => Ok(true),
            Err(e) => {
                tracing::error!("Failed to add user {} to allowlist: {}", user_id, e);
                Err(DataError::DatabaseError {
                    source: e,
                    operation: "add_to_allowlist".to_string(),
                })
            }
        }
    }

    /// Remove a user from the dashboard allowlist
    /// This will also cascade delete all their tokens
    pub async fn remove_from_allowlist(&self, user_id: i64) -> DataResult<bool> {
        match dashboard_allowlist::Entity::delete_by_id(user_id)
            .exec(&self.db)
            .await
        {
            Ok(result) => Ok(result.rows_affected > 0),
            Err(e) => {
                tracing::error!("Failed to remove user {} from allowlist: {}", user_id, e);
                Err(DataError::DatabaseError {
                    source: e,
                    operation: "remove_from_allowlist".to_string(),
                })
            }
        }
    }

    /// Check if a user is in the allowlist
    pub async fn is_allowlisted(&self, user_id: i64) -> DataResult<bool> {
        match dashboard_allowlist::Entity::find_by_id(user_id)
            .one(&self.db)
            .await
        {
            Ok(result) => Ok(result.is_some()),
            Err(e) => {
                tracing::error!("Failed to check allowlist for user {}: {}", user_id, e);
                Err(DataError::DatabaseError {
                    source: e,
                    operation: "is_allowlisted".to_string(),
                })
            }
        }
    }

    /// List all users in the allowlist
    pub async fn list_allowlist(&self) -> DataResult<Vec<dashboard_allowlist::Model>> {
        match dashboard_allowlist::Entity::find().all(&self.db).await {
            Ok(list) => Ok(list),
            Err(e) => {
                tracing::error!("Failed to list allowlist: {}", e);
                Err(DataError::DatabaseError {
                    source: e,
                    operation: "list_allowlist".to_string(),
                })
            }
        }
    }

    // ==================== Token Methods ====================

    /// Create a new dashboard token hash for a user
    pub async fn create_dashboard_token_hash(
        &self,
        user_id: i64,
        token_hash: String,
        description: String,
    ) -> DataResult<Uuid> {
        if !self.is_allowlisted(user_id).await? {
            return Err(DataError::NotAllowlisted { user_id });
        }

        let now = OffsetDateTime::now_utc();
        let token_id = Uuid::now_v7();

        let token_entry = dashboard_tokens::ActiveModel {
            token_id: ActiveValue::Set(token_id),
            user_id: ActiveValue::Set(user_id),
            token_hash: ActiveValue::Set(token_hash),
            description: ActiveValue::Set(description),
            created_at: ActiveValue::Set(now),
            last_used_at: ActiveValue::NotSet,
            expires_at: ActiveValue::NotSet,
            active: ActiveValue::Set(true),
        };

        match token_entry.insert(&self.db).await {
            Ok(_) => Ok(token_id),
            Err(e) => {
                tracing::error!("Failed to create token for user {}: {}", user_id, e);
                Err(DataError::DatabaseError {
                    source: e,
                    operation: "create_dashboard_token_hash".to_string(),
                })
            }
        }
    }

    /// List all active tokens
    pub async fn list_active_tokens(&self) -> DataResult<Vec<dashboard_tokens::Model>> {
        dashboard_tokens::Entity::find()
            .filter(dashboard_tokens::Column::Active.eq(true))
            .all(&self.db)
            .await
            .map_err(|e| DataError::DatabaseError {
                source: e,
                operation: "list_active_tokens".to_string(),
            })
    }

    /// Update the last_used_at timestamp for a token by id
    pub async fn update_token_last_used_by_id(&self, token_id: Uuid) -> DataResult<()> {
        let token = dashboard_tokens::Entity::find_by_id(token_id)
            .one(&self.db)
            .await
            .map_err(|e| DataError::DatabaseError {
                source: e,
                operation: "update_token_last_used_find".to_string(),
            })?;

        let Some(token) = token else {
            return Ok(());
        };

        let mut active_model = token.into_active_model();
        active_model.last_used_at = ActiveValue::Set(Some(OffsetDateTime::now_utc()));

        active_model
            .update(&self.db)
            .await
            .map_err(|e| DataError::DatabaseError {
                source: e,
                operation: "update_token_last_used_update".to_string(),
            })?;

        Ok(())
    }

    /// List all tokens for a user
    pub async fn list_user_tokens(&self, user_id: i64) -> DataResult<Vec<dashboard_tokens::Model>> {
        match dashboard_tokens::Entity::find()
            .filter(dashboard_tokens::Column::UserId.eq(user_id))
            .all(&self.db)
            .await
        {
            Ok(tokens) => Ok(tokens),
            Err(e) => {
                tracing::error!("Failed to list tokens for user {}: {}", user_id, e);
                Err(DataError::DatabaseError {
                    source: e,
                    operation: "list_user_tokens".to_string(),
                })
            }
        }
    }

    /// List all tokens (owner only)
    pub async fn list_all_tokens(&self) -> DataResult<Vec<dashboard_tokens::Model>> {
        match dashboard_tokens::Entity::find().all(&self.db).await {
            Ok(tokens) => Ok(tokens),
            Err(e) => {
                tracing::error!("Failed to list all tokens: {}", e);
                Err(DataError::DatabaseError {
                    source: e,
                    operation: "list_all_tokens".to_string(),
                })
            }
        }
    }

    /// Revoke a token by ID
    pub async fn revoke_token(&self, token_id: Uuid) -> DataResult<bool> {
        match dashboard_tokens::Entity::find_by_id(token_id)
            .one(&self.db)
            .await
        {
            Ok(Some(token)) => {
                let mut active_model = token.into_active_model();
                active_model.active = ActiveValue::Set(false);

                match active_model.update(&self.db).await {
                    Ok(_) => Ok(true),
                    Err(e) => {
                        tracing::error!("Failed to revoke token {}: {}", token_id, e);
                        Err(DataError::DatabaseError {
                            source: e,
                            operation: "revoke_token_update".to_string(),
                        })
                    }
                }
            }
            Ok(None) => Ok(false),
            Err(e) => {
                tracing::error!("Failed to find token {} for revocation: {}", token_id, e);
                Err(DataError::DatabaseError {
                    source: e,
                    operation: "revoke_token_find".to_string(),
                })
            }
        }
    }
}
