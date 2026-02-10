use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(WuwaImportState::Table)
                    .if_not_exists()
                    .col(integer(WuwaImportState::WuwaUserId).not_null())
                    .col(string(WuwaImportState::PoolId).not_null())
                    .col(timestamp_with_time_zone(WuwaImportState::LastTime).not_null())
                    .col(integer(WuwaImportState::CountAtTime).not_null())
                    .primary_key(
                        Index::create()
                            .col(WuwaImportState::WuwaUserId)
                            .col(WuwaImportState::PoolId),
                    )
                    .foreign_key(
                        ForeignKeyCreateStatement::new()
                            .from_tbl(WuwaImportState::Table)
                            .from_col(WuwaImportState::WuwaUserId)
                            .to_tbl(WuwaUser::Table)
                            .to_col(WuwaUser::WuwaUserId),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(WuwaImportState::Table).to_owned())
            .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum WuwaImportState {
    Table,
    WuwaUserId,
    PoolId,
    LastTime,
    CountAtTime,
}

#[derive(DeriveIden)]
#[allow(clippy::enum_variant_names)]
enum WuwaUser {
    Table,
    WuwaUserId,
}
