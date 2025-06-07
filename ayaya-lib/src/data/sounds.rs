use entity_sqlite::prelude::*;
use poise::serenity_prelude as serenity;
use sea_orm::prelude::*;
use sea_orm::ActiveValue;
use sea_orm::DatabaseConnection;
use sea_orm::IntoActiveModel;

use crate::data::error::DataError;
use crate::data::utils::DataTiming;
use crate::metrics::DataOperationType;
use crate::metrics::Metrics;

use super::DataResult;

#[derive(Debug, Clone)]
pub struct SoundsManager {
    sounds_db: DatabaseConnection,
    metrics_handler: Metrics,
}

impl SoundsManager {
    /// Create a new instance of [`Self`]
    pub fn new(sounds_db: DatabaseConnection, metrics_handler: Metrics) -> Self {
        Self {
            sounds_db,
            metrics_handler,
        }
    }

    /// Add a sound into the database.
    pub async fn add_sound(
        &self,
        user_id: &serenity::UserId,
        uploaded_server_id: u64,
        sound_id: uuid::Uuid,
        sound_name: String,
        public: Option<bool>,
    ) -> DataResult<()> {
        const OP: &str = "add_sound";
        self.metrics_handler
            .data_access(OP, DataOperationType::Write)
            .await;
        let _timing = DataTiming::new(
            OP.to_string(),
            DataOperationType::Write,
            Some(self.metrics_handler.clone()),
        );

        use entity_sqlite::sounds;
        let sound = Sounds::find()
            .filter(sounds::Column::UserId.eq(user_id.get()))
            .filter(sounds::Column::SoundId.eq(sound_id))
            .one(&self.sounds_db)
            .await
            .map_err(|error| DataError::DatabaseError {
                operation: OP.to_string(),
                error,
            })?;

        if let Some(sound) = sound {
            return Err(DataError::DuplicateSoundError {
                sound_name: sound.sound_name,
                user_id: *user_id,
            });
        } else {
            sounds::ActiveModel {
                user_id: ActiveValue::set(user_id.get() as i64),
                sound_id: ActiveValue::Set(sound_id),
                uploaded_server_id: ActiveValue::Set(uploaded_server_id as i64),
                sound_name: ActiveValue::Set(sound_name),
                public: ActiveValue::Set(public.unwrap_or(true)),
            }
            .insert(&self.sounds_db)
            .await
            .map_err(|error| DataError::DatabaseError {
                operation: OP.to_string(),
                error,
            })?;
        }
        Ok(())
    }

    pub async fn get_user_sounds_and_public(
        &self,
        user_id: &serenity::UserId,
    ) -> DataResult<Vec<entity_sqlite::sounds::Model>> {
        const OP: &str = "get_user_sounds";
        self.metrics_handler
            .data_access(OP, DataOperationType::Read)
            .await;
        let _timing = DataTiming::new(
            OP.to_string(),
            DataOperationType::Read,
            Some(self.metrics_handler.clone()),
        );

        use entity_sqlite::sounds;
        let sounds = Sounds::find()
            .filter(
                sounds::Column::UserId
                    .eq(user_id.get())
                    .or(sounds::Column::Public.eq(true)),
            )
            .all(&self.sounds_db)
            .await
            .map_err(|error| DataError::DatabaseError {
                operation: OP.to_string(),
                error,
            })?;

        Ok(sounds)
    }

    pub async fn set_user_public_upload_policy(
        &self,
        user_id: &serenity::UserId,
        agreed: bool,
    ) -> DataResult<()> {
        const OP: &str = "set_user_public_upload_policy";
        self.metrics_handler
            .data_access(OP, DataOperationType::Read)
            .await;
        let _timing = DataTiming::new(
            OP.to_string(),
            DataOperationType::Read,
            Some(self.metrics_handler.clone()),
        );

        use entity_sqlite::upload_noticed;
        let model = UploadNoticed::find()
            .filter(upload_noticed::Column::UserId.eq(user_id.get()))
            .one(&self.sounds_db)
            .await
            .map_err(|error| DataError::DatabaseError {
                operation: OP.to_string(),
                error,
            })?;

        if let Some(model) = model {
            let mut active = model.into_active_model();

            active.agreed = ActiveValue::Set(agreed);
            active
                .save(&self.sounds_db)
                .await
                .map_err(|error| DataError::DatabaseError {
                    operation: OP.to_string(),
                    error,
                })?;
        } else {
            upload_noticed::ActiveModel {
                user_id: ActiveValue::Set(user_id.get() as i64),
                agreed: ActiveValue::Set(agreed),
            }
            .insert(&self.sounds_db)
            .await
            .map_err(|error| DataError::DatabaseError {
                operation: OP.to_string(),
                error,
            })?;
        }

        Ok(())
    }

    /// Returns whether the user has accepted the upload notice or not
    pub async fn get_user_public_upload_policy(
        &self,
        user_id: &serenity::UserId,
    ) -> DataResult<bool> {
        const OP: &str = "get_user_public_upload_policy";
        self.metrics_handler
            .data_access(OP, DataOperationType::Read)
            .await;
        let _timing = DataTiming::new(
            OP.to_string(),
            DataOperationType::Read,
            Some(self.metrics_handler.clone()),
        );

        use entity_sqlite::upload_noticed;
        let model = UploadNoticed::find()
            .filter(upload_noticed::Column::UserId.eq(user_id.get()))
            .one(&self.sounds_db)
            .await
            .map_err(|error| DataError::DatabaseError {
                operation: OP.to_string(),
                error,
            })?;
        let res = match model {
            Some(model) => model.agreed,
            _ => false,
        };

        Ok(res)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use migration::MigratorTrait;
    use migration_sqlite::Migrator as SqliteMigrator;
    use poise::serenity_prelude as serenity;
    use sea_orm::Database;

    const GUILD_ID: u64 = 594465820151644180;
    const USER_ID_1: serenity::UserId = serenity::UserId::new(594465820151644181);
    const USER_ID_2: serenity::UserId = serenity::UserId::new(594465820151644182);

    async fn get_manager() -> SoundsManager {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        SqliteMigrator::up(&db, None).await.unwrap();
        SoundsManager::new(db, Metrics::default())
    }

    #[tokio::test]
    async fn create_sound() {
        let manager = get_manager().await;

        let sound_id = uuid::Uuid::new_v4();

        manager
            .add_sound(
                &USER_ID_1,
                GUILD_ID,
                sound_id,
                "Example 1".to_string(),
                None,
            )
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn public_sounds() {
        let manager = get_manager().await;

        let sound_id = uuid::Uuid::new_v4();

        manager
            .add_sound(&USER_ID_1, GUILD_ID, sound_id, "Ex".to_string(), None)
            .await
            .unwrap();

        let sounds = manager
            .get_user_sounds_and_public(&USER_ID_1)
            .await
            .unwrap();

        assert!(sounds.len() > 0, "{sounds:?}");
    }

    #[tokio::test]
    async fn private_sounds() {
        let manager = get_manager().await;

        let sound_id = uuid::Uuid::new_v4();

        manager
            .add_sound(
                &USER_ID_1,
                GUILD_ID,
                sound_id,
                "Ex".to_string(),
                Some(false),
            )
            .await
            .unwrap();

        let sounds = manager
            .get_user_sounds_and_public(&USER_ID_2)
            .await
            .unwrap();

        assert!(sounds.len() == 0, "{sounds:?}");
    }
}
