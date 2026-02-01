use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(DashboardAllowlist::Table)
                    .if_not_exists()
                    .col(big_unsigned(DashboardAllowlist::UserId).primary_key())
                    .col(big_unsigned(DashboardAllowlist::AddedBy).not_null())
                    .col(timestamp_with_time_zone(DashboardAllowlist::AddedAt).not_null())
                    .col(string_null(DashboardAllowlist::Notes))
                    .to_owned(),
            )
            .await?;

        // Create index on added_by for faster lookups
        manager
            .create_index(
                Index::create()
                    .name("idx_dashboard_allowlist_added_by")
                    .table(DashboardAllowlist::Table)
                    .col(DashboardAllowlist::AddedBy)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(DashboardAllowlist::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum DashboardAllowlist {
    Table,
    UserId,
    AddedBy,
    AddedAt,
    Notes,
}
