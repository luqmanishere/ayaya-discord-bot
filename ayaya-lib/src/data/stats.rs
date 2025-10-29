//! This module contains data for stats manipulation
//!

use entity_sqlite::prelude::*;
use poise::serenity_prelude as serenity;
use sea_orm::ActiveValue;
use sea_orm::DatabaseConnection;
use sea_orm::IntoActiveModel;
use sea_orm::prelude::*;
use time::UtcOffset;

use crate::data::error::DataError;
use crate::data::utils::DataTiming;
use crate::metrics::Metrics;

use super::DataResult;

#[derive(Debug, Clone)]
pub struct StatsManager {
    stats_db: DatabaseConnection,
    metrics_handler: Metrics,
}

impl StatsManager {
    /// Create a new instance of [`Self`]
    pub fn new(stats_db: DatabaseConnection, metrics_handler: Metrics) -> Self {
        Self {
            stats_db,
            metrics_handler,
        }
    }

    pub async fn add_song_queue_count(
        &self,
        guild_id: u64,
        user: &serenity::User,
        song_id: String,
        description: Option<String>,
    ) -> DataResult<()> {
        const OP: &str = "add_song_id_count";
        self.metrics_handler
            .data_access(OP, crate::metrics::DataOperationType::Write)
            .await;
        let _timing = DataTiming::new(
            OP.to_string(),
            crate::metrics::DataOperationType::Write,
            Some(self.metrics_handler.clone()),
        );

        use entity_sqlite::song_queues;
        let count = SongQueues::find()
            .filter(song_queues::Column::ServerId.eq(guild_id))
            .filter(song_queues::Column::UserId.eq(user.id.get()))
            .filter(song_queues::Column::YoutubeId.eq(&song_id))
            .one(&self.stats_db)
            .await
            .map_err(|error| DataError::DatabaseError {
                operation: OP.to_string(),
                error,
            })?;

        let now_odt = time::OffsetDateTime::now_utc()
            .to_offset(UtcOffset::from_hms(8, 0, 0).unwrap_or(UtcOffset::UTC));
        if let Some(count) = count {
            let new_count = count.count + 1;
            let mut model = count.into_active_model();
            model.count = ActiveValue::set(new_count);
            model.last_update = ActiveValue::set(Some(now_odt));
            if description.is_some() {
                model.description = ActiveValue::set(description);
            }
            model
                .save(&self.stats_db)
                .await
                .map_err(|error| DataError::DatabaseError {
                    operation: OP.to_string(),
                    error,
                })?;
        } else {
            song_queues::ActiveModel {
                server_id: ActiveValue::set(guild_id as i64),
                user_id: ActiveValue::set(user.id.get() as i64),
                youtube_id: ActiveValue::set(song_id),
                description: ActiveValue::Set(description),
                count: ActiveValue::set(1),
                last_update: ActiveValue::Set(Some(now_odt)),
            }
            .insert(&self.stats_db)
            .await
            .map_err(|error| DataError::DatabaseError {
                operation: OP.to_string(),
                error,
            })?;
        }

        Ok(())
    }
}

impl StatsManager {
    pub async fn add_user_play_query(
        &mut self,
        guild_id: u64,
        user: &serenity::User,
        query: String,
        query_type: String,
        description: String,
    ) -> DataResult<()> {
        tracing::info!("called");
        const OP: &str = "add_user_play_query";
        self.metrics_handler
            .data_access(OP, crate::metrics::DataOperationType::Write)
            .await;
        let _timing = DataTiming::new(
            OP.to_string(),
            crate::metrics::DataOperationType::Write,
            Some(self.metrics_handler.clone()),
        );

        use entity_sqlite::user_play_queries;
        let maybe_model = UserPlayQueries::find()
            .filter(user_play_queries::Column::UserId.eq(user.id.get()))
            .filter(user_play_queries::Column::ServerId.eq(guild_id))
            .filter(user_play_queries::Column::Query.eq(&query))
            .one(&self.stats_db)
            .await
            .map_err(|error| DataError::DatabaseError {
                operation: OP.to_string(),
                error,
            })?;

        if let Some(model) = maybe_model {
            let new_count = model.count + 1;
            let mut active = model.into_active_model();
            active.count = ActiveValue::Set(new_count);
            active
                .save(&self.stats_db)
                .await
                .map_err(|error| DataError::DatabaseError {
                    operation: OP.to_string(),
                    error,
                })?;
        } else {
            user_play_queries::ActiveModel {
                server_id: ActiveValue::Set(guild_id as i64),
                user_id: ActiveValue::Set(user.id.get() as i64),
                query: ActiveValue::Set(query),
                query_type: ActiveValue::Set(query_type),
                description: ActiveValue::Set(description),
                count: ActiveValue::Set(1),
            }
            .insert(&self.stats_db)
            .await
            .map_err(|error| DataError::DatabaseError {
                operation: OP.to_string(),
                error,
            })?;
        }
        Ok(())
    }

    pub async fn update_user_play_queries_description(
        &mut self,
        query: String,
        description: String,
    ) -> DataResult<()> {
        const OP: &str = "update_user_play_queries_description";
        self.metrics_handler
            .data_access(OP, crate::metrics::DataOperationType::Write)
            .await;
        let _timing = DataTiming::new(
            OP.to_string(),
            crate::metrics::DataOperationType::Write,
            Some(self.metrics_handler.clone()),
        );

        use entity_sqlite::user_play_queries;
        let maybe_model = UserPlayQueries::find()
            .filter(user_play_queries::Column::Query.eq(&query))
            .all(&self.stats_db)
            .await
            .map_err(|error| DataError::DatabaseError {
                operation: OP.to_string(),
                error,
            })?;

        for model in maybe_model {
            let mut active = model.into_active_model();
            active.description = ActiveValue::set(description.clone());
            active
                .update(&self.stats_db)
                .await
                .map_err(|error| DataError::DatabaseError {
                    operation: OP.to_string(),
                    error,
                })?;
        }
        Ok(())
    }

    /// Get the queries previously done by the user in a server
    ///
    /// # Errors
    ///
    /// This function will return an error if the database is inacessible
    pub async fn get_user_play_queries(
        &mut self,
        guild_id: u64,
        user: &serenity::User,
    ) -> DataResult<Vec<entity_sqlite::user_play_queries::Model>> {
        const OP: &str = "get_user_play_queries";
        self.metrics_handler
            .data_access(OP, crate::metrics::DataOperationType::Write)
            .await;
        let _timing = DataTiming::new(
            OP.to_string(),
            crate::metrics::DataOperationType::Write,
            Some(self.metrics_handler.clone()),
        );

        use entity_sqlite::user_play_queries;
        let models = UserPlayQueries::find()
            .filter(user_play_queries::Column::ServerId.eq(guild_id))
            .filter(user_play_queries::Column::UserId.eq(user.id.get()))
            .all(&self.stats_db)
            .await
            .map_err(|error| DataError::DatabaseError {
                operation: OP.to_string(),
                error,
            })?;
        Ok(models)
    }
}
