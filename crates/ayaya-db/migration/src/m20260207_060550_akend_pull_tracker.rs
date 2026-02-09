use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(AkEndUser::Table)
                    .if_not_exists()
                    .col(integer(AkEndUser::UserId).not_null())
                    .col(integer(AkEndUser::AkEndUserId).primary_key())
                    .col(string(AkEndUser::UserDesc).not_null())
                    .to_owned(),
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .table(AkEndPull::Table)
                    .if_not_exists()
                    .col(uuid(AkEndPull::Id).primary_key())
                    .col(integer(AkEndPull::UserId).not_null())
                    .col(integer(AkEndPull::AkEndUserId).not_null())
                    .col(string(AkEndPull::PoolType).not_null())
                    .col(string(AkEndPull::PoolId).not_null())
                    .col(string(AkEndPull::PoolName).not_null())
                    .col(string(AkEndPull::CharId).not_null())
                    .col(string(AkEndPull::CharName).not_null())
                    .col(integer(AkEndPull::Rarity).not_null())
                    .col(boolean(AkEndPull::IsFree).not_null())
                    .col(boolean(AkEndPull::IsNew).not_null())
                    .col(timestamp_with_time_zone(AkEndPull::Time).not_null())
                    .col(string(AkEndPull::SeqId).not_null())
                    .foreign_key(
                        ForeignKeyCreateStatement::new()
                            .from_tbl(AkEndPull::Table)
                            .from_col(AkEndPull::AkEndUserId)
                            .to_tbl(AkEndUser::Table)
                            .to_col(AkEndUser::AkEndUserId),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(AkEndImportState::Table)
                    .if_not_exists()
                    .col(integer(AkEndImportState::AkEndUserId).not_null())
                    .col(string(AkEndImportState::PoolId).not_null())
                    .col(timestamp_with_time_zone(AkEndImportState::LastTime).not_null())
                    .col(integer(AkEndImportState::CountAtTime).not_null())
                    .primary_key(
                        Index::create()
                            .col(AkEndImportState::AkEndUserId)
                            .col(AkEndImportState::PoolId),
                    )
                    .foreign_key(
                        ForeignKeyCreateStatement::new()
                            .from_tbl(AkEndImportState::Table)
                            .from_col(AkEndImportState::AkEndUserId)
                            .to_tbl(AkEndUser::Table)
                            .to_col(AkEndUser::AkEndUserId),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(AkEndUser::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(AkEndPull::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(AkEndImportState::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum AkEndPull {
    Table,
    Id,
    UserId,
    AkEndUserId,
    PoolType,
    PoolId,
    PoolName,
    CharId,
    CharName,
    Rarity,
    IsFree,
    IsNew,
    Time,
    SeqId,
}

#[derive(DeriveIden)]
enum AkEndUser {
    Table,
    UserId,
    AkEndUserId,
    UserDesc,
}

#[derive(DeriveIden)]
enum AkEndImportState {
    Table,
    AkEndUserId,
    PoolId,
    LastTime,
    CountAtTime,
}
