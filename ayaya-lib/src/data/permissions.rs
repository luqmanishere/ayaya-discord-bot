use std::sync::Arc;

use lru_mem::{HeapSize, LruCache};
use poise::serenity_prelude as serenity;
use sea_orm::{prelude::*, DatabaseConnection};
use serenity::futures::TryFutureExt;
use tokio::sync::Mutex;

use crate::metrics::{DataOperationType, Metrics};
use entity::prelude::*;

use super::{error::DataError, utils::DataTiming, DataResult};

pub type PermissionCache = Arc<Mutex<LruCache<PermissionCacheKey, Vec<u8>>>>;

#[derive(Clone, Debug)]
pub struct Permissions {
    db: DatabaseConnection,
    cache: PermissionCache,
    metrics_handler: Metrics,
}

impl Permissions {
    /// A new instance of the manager
    pub async fn new(db: DatabaseConnection, metrics_handler: Metrics) -> DataResult<Self> {
        let cache = Arc::new(Mutex::new(LruCache::new(1024 * 1024)));
        Self::setup_cache_metrics(metrics_handler.clone(), cache.clone()).await;
        Ok(Self {
            db,
            metrics_handler,
            cache,
        })
    }
}

impl Permissions {
    /// Find a value in the permission cache. Returns a [`None`] if there is no value for the provided key
    pub async fn permission_cache_access(&mut self, key: &PermissionCacheKey) -> Option<Vec<u8>> {
        let mut cache = self.cache.lock().await;
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
        let mut cache = self.cache.lock().await;
        if let Err(error) = cache.insert(key, value) {
            tracing::error!("error inserting into permission cache: {error}");
        };
    }

    /// Invalidates the cache based on the key given. The key is broken into each part and any containing part is invalidated(removed).
    pub async fn permission_cache_invalidate(&mut self, key: PermissionCacheKey) {
        let _timing = DataTiming::new(
            "permission_cache_invalidate".to_string(),
            DataOperationType::Write,
            Some(self.metrics_handler.clone()),
        );
        let mut cache = self.cache.lock().await;
        cache.retain(|cached_key, _| {
            // remove the cache of the command for the server
            !(cached_key.comorcat == key.comorcat && cached_key.guild_id == key.guild_id)
        });
    }
}

impl Permissions {
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
        const OP: &str = "find_user_allowed";
        let _timing = DataTiming::new(
            OP.to_string(),
            DataOperationType::Read,
            Some(self.metrics_handler.clone()),
        );

        use entity::command_allow_user;
        self.metrics_handler
            .data_access(OP.to_string(), DataOperationType::Read)
            .await;

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
        const OP: &str = "find_command_roles_allowed";
        let _timing = DataTiming::new(
            OP.to_string(),
            DataOperationType::Read,
            Some(self.metrics_handler.clone()),
        );

        use entity::require_command_role;
        self.metrics_handler
            .data_access(OP.to_string(), DataOperationType::Read)
            .await;

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
        const OP: &str = "find_category_roles_allowed";
        let _timing = DataTiming::new(
            OP.to_string(),
            DataOperationType::Read,
            Some(self.metrics_handler.clone()),
        );

        use entity::require_category_role;
        self.metrics_handler
            .data_access(OP.to_string(), DataOperationType::Read)
            .await;

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
        const OP: &str = "new_command_role_restriction";
        let _timing = DataTiming::new(
            OP.to_string(),
            DataOperationType::Write,
            Some(self.metrics_handler.clone()),
        );
        self.metrics_handler
            .data_access(OP, DataOperationType::Write)
            .await;

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
        const OP: &str = "new_category_role_restriction";
        let _timing = DataTiming::new(
            OP.to_string(),
            DataOperationType::Write,
            Some(self.metrics_handler.clone()),
        );
        self.metrics_handler
            .data_access(OP, DataOperationType::Write)
            .await;

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
        const OP: &str = "new_command_user_allowed";
        let _timing = DataTiming::new(
            OP.to_string(),
            DataOperationType::Write,
            Some(self.metrics_handler.clone()),
        );
        self.metrics_handler
            .data_access(OP, DataOperationType::Write)
            .await;

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
        const OP: &str = "findall_user_allowed";
        let _timing = DataTiming::new(
            OP.to_string(),
            DataOperationType::Read,
            Some(self.metrics_handler.clone()),
        );
        self.metrics_handler
            .data_access(OP, DataOperationType::Read)
            .await;

        use entity::command_allow_user;
        CommandAllowUser::find()
            .filter(command_allow_user::Column::ServerId.eq(guild_id))
            .filter(command_allow_user::Column::Command.eq(command))
            .all(&self.db)
            .await
            .map_err(DataError::FindAllAllowedUserDatabaseError)
    }
}

impl Permissions {
    pub async fn setup_cache_metrics(metrics_handler: Metrics, permission_cache: PermissionCache) {
        tokio::spawn(async move {
            loop {
                metrics_handler
                    .cache_len("permission_cache", permission_cache.lock().await.len())
                    .await;
                tokio::time::sleep(std::time::Duration::from_secs(30)).await;
            }
        });
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
