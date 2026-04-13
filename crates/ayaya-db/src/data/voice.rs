use std::sync::Arc;

use ayaya_core::metrics::{DataOperationType, MetricsSink};
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, DatabaseConnection, DatabaseTransaction,
    EntityTrait, IntoActiveModel, QueryFilter, QueryOrder, TransactionTrait,
};
use snafu::ResultExt;
use time::OffsetDateTime;
use uuid::Uuid;

use super::{DataResult, utils::DataTiming};
use crate::entity::{voice_sessions, voice_state_events};
use crate::error::DatabaseSnafu;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VoiceEventKind {
    Join,
    Leave,
    Move,
    StateChange,
}

impl VoiceEventKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Join => "join",
            Self::Leave => "leave",
            Self::Move => "move",
            Self::StateChange => "state_change",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VoiceSessionEndReason {
    Left,
    Moved,
    BotRestart,
    Disconnect,
    Unknown,
}

impl VoiceSessionEndReason {
    fn as_str(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Moved => "moved",
            Self::BotRestart => "bot_restart",
            Self::Disconnect => "disconnect",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Clone, Debug)]
pub struct VoiceStateUpdateInput {
    pub guild_id: i64,
    pub user_id: i64,
    pub from_channel_id: Option<i64>,
    pub to_channel_id: Option<i64>,
    pub occurred_at: OffsetDateTime,
    pub self_mute: bool,
    pub self_deaf: bool,
    pub mute: bool,
    pub deaf: bool,
    pub self_stream: bool,
    pub self_video: bool,
    pub suppress: bool,
    pub request_to_speak_at: Option<OffsetDateTime>,
    pub raw_state_json: Option<String>,
    pub start_is_estimated: bool,
}

impl VoiceStateUpdateInput {
    pub fn classify(&self) -> VoiceEventKind {
        match (self.from_channel_id, self.to_channel_id) {
            (None, Some(_)) => VoiceEventKind::Join,
            (Some(_), None) => VoiceEventKind::Leave,
            (Some(from), Some(to)) if from != to => VoiceEventKind::Move,
            _ => VoiceEventKind::StateChange,
        }
    }
}

#[derive(Clone)]
pub struct VoiceManager {
    db: DatabaseConnection,
    metrics_handler: Arc<dyn MetricsSink>,
}

impl VoiceManager {
    pub fn new(db: DatabaseConnection, metrics_handler: Arc<dyn MetricsSink>) -> Self {
        Self {
            db,
            metrics_handler,
        }
    }

    /// Persist a single voice-state transition using an event row plus session updates.
    pub async fn apply_voice_state_update(&self, input: VoiceStateUpdateInput) -> DataResult<Uuid> {
        const OP: &str = "apply_voice_state_update";
        self.metrics_handler
            .data_access(OP, DataOperationType::Write)
            .await;
        let _timing = DataTiming::new(
            OP.to_string(),
            DataOperationType::Write,
            Some(self.metrics_handler.clone()),
        );

        let txn = self
            .db
            .begin()
            .await
            .context(DatabaseSnafu { operation: OP })?;

        let event_kind = input.classify();
        let event_id = Uuid::now_v7();

        insert_voice_state_event(&txn, event_id, &input, event_kind, OP).await?;

        match event_kind {
            VoiceEventKind::Join => {
                if let Some(open_session) =
                    find_open_voice_session(&txn, input.guild_id, input.user_id, OP).await?
                {
                    close_voice_session(
                        &txn,
                        open_session,
                        event_id,
                        input.occurred_at,
                        VoiceSessionEndReason::Unknown,
                        input.raw_state_json.clone(),
                        OP,
                    )
                    .await?;
                }

                if let Some(channel_id) = input.to_channel_id {
                    insert_voice_session(
                        &txn,
                        input.guild_id,
                        input.user_id,
                        channel_id,
                        event_id,
                        input.occurred_at,
                        input.start_is_estimated,
                        input.raw_state_json.clone(),
                        OP,
                    )
                    .await?;
                }
            }
            VoiceEventKind::Leave => {
                if let Some(open_session) =
                    find_open_voice_session(&txn, input.guild_id, input.user_id, OP).await?
                {
                    close_voice_session(
                        &txn,
                        open_session,
                        event_id,
                        input.occurred_at,
                        VoiceSessionEndReason::Left,
                        input.raw_state_json.clone(),
                        OP,
                    )
                    .await?;
                }
            }
            VoiceEventKind::Move => {
                if let Some(open_session) =
                    find_open_voice_session(&txn, input.guild_id, input.user_id, OP).await?
                {
                    close_voice_session(
                        &txn,
                        open_session,
                        event_id,
                        input.occurred_at,
                        VoiceSessionEndReason::Moved,
                        input.raw_state_json.clone(),
                        OP,
                    )
                    .await?;
                }

                if let Some(channel_id) = input.to_channel_id {
                    insert_voice_session(
                        &txn,
                        input.guild_id,
                        input.user_id,
                        channel_id,
                        event_id,
                        input.occurred_at,
                        input.start_is_estimated,
                        input.raw_state_json.clone(),
                        OP,
                    )
                    .await?;
                }
            }
            VoiceEventKind::StateChange => {}
        }

        txn.commit()
            .await
            .context(DatabaseSnafu { operation: OP })?;

        Ok(event_id)
    }

    /// Startup reconciliation path: ensure a user already present in voice has an open session.
    pub async fn ensure_open_voice_session(
        &self,
        guild_id: i64,
        user_id: i64,
        channel_id: i64,
        observed_at: OffsetDateTime,
        raw_state_json: Option<String>,
    ) -> DataResult<Option<Uuid>> {
        const OP: &str = "ensure_open_voice_session";
        self.metrics_handler
            .data_access(OP, DataOperationType::Write)
            .await;
        let _timing = DataTiming::new(
            OP.to_string(),
            DataOperationType::Write,
            Some(self.metrics_handler.clone()),
        );

        let txn = self
            .db
            .begin()
            .await
            .context(DatabaseSnafu { operation: OP })?;

        let open_session = find_open_voice_session(&txn, guild_id, user_id, OP).await?;
        if open_session.is_some() {
            txn.commit()
                .await
                .context(DatabaseSnafu { operation: OP })?;
            return Ok(None);
        }

        let session_id = insert_voice_session(
            &txn,
            guild_id,
            user_id,
            channel_id,
            Uuid::now_v7(),
            observed_at,
            true,
            raw_state_json,
            OP,
        )
        .await?;

        txn.commit()
            .await
            .context(DatabaseSnafu { operation: OP })?;

        Ok(Some(session_id))
    }

    pub async fn close_all_open_voice_sessions(
        &self,
        observed_at: OffsetDateTime,
        reason: VoiceSessionEndReason,
    ) -> DataResult<u64> {
        const OP: &str = "close_all_open_voice_sessions";
        self.metrics_handler
            .data_access(OP, DataOperationType::Write)
            .await;
        let _timing = DataTiming::new(
            OP.to_string(),
            DataOperationType::Write,
            Some(self.metrics_handler.clone()),
        );

        let open_sessions = voice_sessions::Entity::find()
            .filter(voice_sessions::Column::LeftAt.is_null())
            .all(&self.db)
            .await
            .context(DatabaseSnafu { operation: OP })?;

        let count = open_sessions.len() as u64;

        for session in open_sessions {
            let mut active_model = session.into_active_model();
            active_model.left_at = ActiveValue::Set(Some(observed_at));
            active_model.leave_event_id = ActiveValue::Set(None);
            active_model.ended_reason = ActiveValue::Set(Some(reason.as_str().to_string()));
            active_model.leave_state_json = ActiveValue::Set(None);
            active_model
                .update(&self.db)
                .await
                .context(DatabaseSnafu { operation: OP })?;
        }

        Ok(count)
    }
}

async fn insert_voice_state_event(
    txn: &DatabaseTransaction,
    event_id: Uuid,
    input: &VoiceStateUpdateInput,
    event_kind: VoiceEventKind,
    operation: &str,
) -> DataResult<()> {
    let event = voice_state_events::ActiveModel {
        event_id: ActiveValue::Set(event_id),
        guild_id: ActiveValue::Set(input.guild_id),
        user_id: ActiveValue::Set(input.user_id),
        event_kind: ActiveValue::Set(event_kind.as_str().to_string()),
        from_channel_id: ActiveValue::Set(input.from_channel_id),
        to_channel_id: ActiveValue::Set(input.to_channel_id),
        occurred_at: ActiveValue::Set(input.occurred_at),
        session_id: ActiveValue::Set(None),
        self_mute: ActiveValue::Set(input.self_mute),
        self_deaf: ActiveValue::Set(input.self_deaf),
        mute: ActiveValue::Set(input.mute),
        deaf: ActiveValue::Set(input.deaf),
        self_stream: ActiveValue::Set(input.self_stream),
        self_video: ActiveValue::Set(input.self_video),
        suppress: ActiveValue::Set(input.suppress),
        request_to_speak_at: ActiveValue::Set(input.request_to_speak_at),
        raw_state_json: ActiveValue::Set(input.raw_state_json.clone()),
    };

    event
        .insert(txn)
        .await
        .context(DatabaseSnafu { operation })?;

    Ok(())
}

async fn insert_voice_session(
    txn: &DatabaseTransaction,
    guild_id: i64,
    user_id: i64,
    channel_id: i64,
    join_event_id: Uuid,
    joined_at: OffsetDateTime,
    start_is_estimated: bool,
    join_state_json: Option<String>,
    operation: &str,
) -> DataResult<Uuid> {
    let session_id = Uuid::now_v7();
    let session = voice_sessions::ActiveModel {
        session_id: ActiveValue::Set(session_id),
        guild_id: ActiveValue::Set(guild_id),
        user_id: ActiveValue::Set(user_id),
        channel_id: ActiveValue::Set(channel_id),
        joined_at: ActiveValue::Set(joined_at),
        start_is_estimated: ActiveValue::Set(start_is_estimated),
        left_at: ActiveValue::Set(None),
        join_event_id: ActiveValue::Set(Some(join_event_id)),
        leave_event_id: ActiveValue::Set(None),
        ended_reason: ActiveValue::Set(None),
        join_state_json: ActiveValue::Set(join_state_json),
        leave_state_json: ActiveValue::Set(None),
    };

    session
        .insert(txn)
        .await
        .context(DatabaseSnafu { operation })?;

    voice_state_events::Entity::update_many()
        .col_expr(
            voice_state_events::Column::SessionId,
            sea_orm::sea_query::Expr::value(session_id),
        )
        .filter(voice_state_events::Column::EventId.eq(join_event_id))
        .exec(txn)
        .await
        .context(DatabaseSnafu { operation })?;

    Ok(session_id)
}

async fn find_open_voice_session(
    txn: &DatabaseTransaction,
    guild_id: i64,
    user_id: i64,
    operation: &str,
) -> DataResult<Option<voice_sessions::Model>> {
    voice_sessions::Entity::find()
        .filter(voice_sessions::Column::GuildId.eq(guild_id))
        .filter(voice_sessions::Column::UserId.eq(user_id))
        .filter(voice_sessions::Column::LeftAt.is_null())
        .order_by_desc(voice_sessions::Column::JoinedAt)
        .one(txn)
        .await
        .context(DatabaseSnafu { operation })
}

async fn close_voice_session(
    txn: &DatabaseTransaction,
    session: voice_sessions::Model,
    leave_event_id: Uuid,
    left_at: OffsetDateTime,
    end_reason: VoiceSessionEndReason,
    leave_state_json: Option<String>,
    operation: &str,
) -> DataResult<()> {
    let session_id = session.session_id;
    let mut active_model = session.into_active_model();
    active_model.left_at = ActiveValue::Set(Some(left_at));
    active_model.leave_event_id = ActiveValue::Set(Some(leave_event_id));
    active_model.ended_reason = ActiveValue::Set(Some(end_reason.as_str().to_string()));
    active_model.leave_state_json = ActiveValue::Set(leave_state_json);

    active_model
        .update(txn)
        .await
        .context(DatabaseSnafu { operation })?;

    voice_state_events::Entity::update_many()
        .col_expr(
            voice_state_events::Column::SessionId,
            sea_orm::sea_query::Expr::value(session_id),
        )
        .filter(voice_state_events::Column::EventId.eq(leave_event_id))
        .exec(txn)
        .await
        .context(DatabaseSnafu { operation })?;

    Ok(())
}
