use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(VoiceSessions::Table)
                    .if_not_exists()
                    .col(pk_uuid(VoiceSessions::SessionId))
                    .col(big_unsigned(VoiceSessions::GuildId).not_null())
                    .col(big_unsigned(VoiceSessions::UserId).not_null())
                    .col(big_unsigned(VoiceSessions::ChannelId).not_null())
                    .col(timestamp_with_time_zone(VoiceSessions::JoinedAt).not_null())
                    .col(
                        boolean(VoiceSessions::StartIsEstimated)
                            .not_null()
                            .default(false),
                    )
                    .col(timestamp_with_time_zone_null(VoiceSessions::LeftAt))
                    .col(uuid_null(VoiceSessions::JoinEventId))
                    .col(uuid_null(VoiceSessions::LeaveEventId))
                    .col(string_null(VoiceSessions::EndedReason))
                    .col(string_null(VoiceSessions::JoinStateJson))
                    .col(string_null(VoiceSessions::LeaveStateJson))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(VoiceStateEvents::Table)
                    .if_not_exists()
                    .col(pk_uuid(VoiceStateEvents::EventId))
                    .col(big_unsigned(VoiceStateEvents::GuildId).not_null())
                    .col(big_unsigned(VoiceStateEvents::UserId).not_null())
                    .col(string(VoiceStateEvents::EventKind).not_null())
                    .col(big_unsigned_null(VoiceStateEvents::FromChannelId))
                    .col(big_unsigned_null(VoiceStateEvents::ToChannelId))
                    .col(timestamp_with_time_zone(VoiceStateEvents::OccurredAt).not_null())
                    .col(uuid_null(VoiceStateEvents::SessionId))
                    .col(
                        boolean(VoiceStateEvents::SelfMute)
                            .not_null()
                            .default(false),
                    )
                    .col(
                        boolean(VoiceStateEvents::SelfDeaf)
                            .not_null()
                            .default(false),
                    )
                    .col(boolean(VoiceStateEvents::Mute).not_null().default(false))
                    .col(boolean(VoiceStateEvents::Deaf).not_null().default(false))
                    .col(
                        boolean(VoiceStateEvents::SelfStream)
                            .not_null()
                            .default(false),
                    )
                    .col(
                        boolean(VoiceStateEvents::SelfVideo)
                            .not_null()
                            .default(false),
                    )
                    .col(
                        boolean(VoiceStateEvents::Suppress)
                            .not_null()
                            .default(false),
                    )
                    .col(timestamp_with_time_zone_null(
                        VoiceStateEvents::RequestToSpeakAt,
                    ))
                    .col(string_null(VoiceStateEvents::RawStateJson))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_voice_state_events_session_id")
                            .from(VoiceStateEvents::Table, VoiceStateEvents::SessionId)
                            .to(VoiceSessions::Table, VoiceSessions::SessionId)
                            .on_delete(ForeignKeyAction::SetNull)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_voice_sessions_guild_user_joined_at")
                    .table(VoiceSessions::Table)
                    .col(VoiceSessions::GuildId)
                    .col(VoiceSessions::UserId)
                    .col(VoiceSessions::JoinedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_voice_sessions_guild_channel_joined_at")
                    .table(VoiceSessions::Table)
                    .col(VoiceSessions::GuildId)
                    .col(VoiceSessions::ChannelId)
                    .col(VoiceSessions::JoinedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_voice_sessions_guild_user_left_at")
                    .table(VoiceSessions::Table)
                    .col(VoiceSessions::GuildId)
                    .col(VoiceSessions::UserId)
                    .col(VoiceSessions::LeftAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_voice_state_events_guild_occurred_at")
                    .table(VoiceStateEvents::Table)
                    .col(VoiceStateEvents::GuildId)
                    .col(VoiceStateEvents::OccurredAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_voice_state_events_guild_user_occurred_at")
                    .table(VoiceStateEvents::Table)
                    .col(VoiceStateEvents::GuildId)
                    .col(VoiceStateEvents::UserId)
                    .col(VoiceStateEvents::OccurredAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_voice_state_events_session_occurred_at")
                    .table(VoiceStateEvents::Table)
                    .col(VoiceStateEvents::SessionId)
                    .col(VoiceStateEvents::OccurredAt)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("idx_voice_state_events_session_occurred_at")
                    .table(VoiceStateEvents::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("idx_voice_state_events_guild_user_occurred_at")
                    .table(VoiceStateEvents::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("idx_voice_state_events_guild_occurred_at")
                    .table(VoiceStateEvents::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("idx_voice_sessions_guild_user_left_at")
                    .table(VoiceSessions::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("idx_voice_sessions_guild_channel_joined_at")
                    .table(VoiceSessions::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("idx_voice_sessions_guild_user_joined_at")
                    .table(VoiceSessions::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_table(Table::drop().table(VoiceStateEvents::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(VoiceSessions::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum VoiceSessions {
    Table,
    SessionId,
    GuildId,
    UserId,
    ChannelId,
    JoinedAt,
    StartIsEstimated,
    LeftAt,
    JoinEventId,
    LeaveEventId,
    EndedReason,
    JoinStateJson,
    LeaveStateJson,
}

#[derive(DeriveIden)]
enum VoiceStateEvents {
    Table,
    EventId,
    GuildId,
    UserId,
    EventKind,
    FromChannelId,
    ToChannelId,
    OccurredAt,
    SessionId,
    SelfMute,
    SelfDeaf,
    Mute,
    Deaf,
    SelfStream,
    SelfVideo,
    Suppress,
    RequestToSpeakAt,
    RawStateJson,
}
