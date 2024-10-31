use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Replace the sample below with your own migration scripts

        // table for users banned from audio controls
        manager
            .create_table(
                Table::create()
                    .table(BanUserCommandUse::Table)
                    .if_not_exists()
                    .col(pk_uuid(BanUserCommandUse::BanId))
                    .col(big_unsigned(BanUserCommandUse::UserId).not_null())
                    .col(big_unsigned(BanUserCommandUse::ServerId).not_null())
                    .col(string(BanUserCommandUse::Command).not_null())
                    .col(string(BanUserCommandUse::Reason).not_null())
                    .col(timestamp_with_time_zone(BanUserCommandUse::BanStart).not_null())
                    .col(timestamp_with_time_zone(BanUserCommandUse::BanEnd).not_null())
                    .col(unsigned(BanUserCommandUse::BanDuration).not_null())
                    .to_owned(),
            )
            .await?;

        // some commands require roles
        manager
            .create_table(
                Table::create()
                    .table(RequireCommandRole::Table)
                    .if_not_exists()
                    .col(pk_uuid(RequireCommandRole::EntryId))
                    .col(big_unsigned(RequireCommandRole::ServerId).not_null())
                    .col(big_unsigned(RequireCommandRole::RoleId).not_null())
                    .col(string(RequireCommandRole::Command).not_null())
                    .index(
                        Index::create()
                            .col(RequireCommandRole::ServerId)
                            .col(RequireCommandRole::RoleId)
                            .col(RequireCommandRole::Command)
                            .unique(),
                    )
                    .to_owned(),
            )
            .await?;

        // allow a user to use a command regardless of role
        manager
            .create_table(
                Table::create()
                    .table(CommandAllowUser::Table)
                    .if_not_exists()
                    .col(pk_uuid(CommandAllowUser::EntryId))
                    .col(big_unsigned(CommandAllowUser::ServerId).not_null())
                    .col(big_unsigned(CommandAllowUser::UserId).not_null())
                    .col(string(CommandAllowUser::Command).not_null())
                    .index(
                        Index::create()
                            .col(CommandAllowUser::ServerId)
                            .col(CommandAllowUser::UserId)
                            .col(CommandAllowUser::Command)
                            .unique(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(UserCommandAllTimeStatistics::Table)
                    .col(big_unsigned(UserCommandAllTimeStatistics::ServerId).not_null()) // commands can be call outside of guilds
                    .col(big_unsigned(UserCommandAllTimeStatistics::UserId).not_null())
                    .col(string(UserCommandAllTimeStatistics::Command).not_null())
                    .col(unsigned(UserCommandAllTimeStatistics::Count).not_null())
                    .primary_key(
                        Index::create()
                            .col(UserCommandAllTimeStatistics::ServerId)
                            .col(UserCommandAllTimeStatistics::UserId)
                            .col(UserCommandAllTimeStatistics::Command),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(BanShitMusic::Table)
                    .if_not_exists()
                    .col(pk_uuid(BanShitMusic::BanId))
                    .col(big_unsigned(BanShitMusic::ServerId).not_null())
                    .col(string(BanShitMusic::Title))
                    .col(string(BanShitMusic::Artist))
                    .col(string(BanShitMusic::YoutubeId))
                    .col(timestamp_with_time_zone(BanShitMusic::BanStart).not_null())
                    .col(timestamp_with_time_zone(BanShitMusic::BanEnd).not_null())
                    .col(unsigned(BanShitMusic::BanDuration).not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(CommandCallLog::Table)
                    .col(pk_uuid(CommandCallLog::LogId))
                    .col(big_unsigned(CommandCallLog::ServerId)) // commands can be called in dms
                    .col(big_unsigned(CommandCallLog::UserId).not_null())
                    .col(string(CommandCallLog::Command).not_null())
                    .col(timestamp_with_time_zone(CommandCallLog::CommandTimeStamp).not_null())
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Replace the sample below with your own migration scripts

        manager
            .drop_table(Table::drop().table(BanUserCommandUse::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(RequireCommandRole::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(CommandAllowUser::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(CommandCallLog::Table).to_owned())
            .await?;

        manager
            .drop_table(
                Table::drop()
                    .table(UserCommandAllTimeStatistics::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_table(Table::drop().table(BanShitMusic::Table).to_owned())
            .await
    }
}

/// Users that are banned from certain commands
#[derive(DeriveIden)]
enum BanUserCommandUse {
    Table,
    BanId,
    UserId,
    ServerId,
    Command,
    Reason,
    BanStart,
    BanEnd,
    BanDuration,
}

/// Require a role to use certain commands or category
#[derive(DeriveIden)]
enum RequireCommandRole {
    Table,
    EntryId,
    ServerId,
    RoleId,
    Command,
}

/// Permit a user to use a command that would normally require a role
#[derive(DeriveIden)]
enum CommandAllowUser {
    Table,
    EntryId,
    ServerId,
    UserId,
    Command,
}

/// Ban some shit music that keeps getting repeated
#[derive(DeriveIden)]
enum BanShitMusic {
    Table,
    BanId,
    ServerId,
    Title,
    Artist,
    YoutubeId,
    BanStart,
    BanEnd,
    BanDuration,
}

/// Keep track of command statistics per user
#[derive(DeriveIden)]
enum UserCommandAllTimeStatistics {
    Table,
    ServerId,
    UserId,
    Command,
    Count,
}

/// Command call logs
#[derive(DeriveIden)]
enum CommandCallLog {
    Table,
    LogId,
    ServerId,
    UserId,
    Command,
    CommandTimeStamp,
}
