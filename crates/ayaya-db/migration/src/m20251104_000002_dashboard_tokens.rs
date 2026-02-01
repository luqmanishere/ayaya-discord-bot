use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(DashboardTokens::Table)
                    .if_not_exists()
                    .col(pk_uuid(DashboardTokens::TokenId))
                    .col(big_unsigned(DashboardTokens::UserId).not_null())
                    .col(string(DashboardTokens::TokenHash).not_null().unique_key())
                    .col(string(DashboardTokens::Description).not_null())
                    .col(timestamp_with_time_zone(DashboardTokens::CreatedAt).not_null())
                    .col(timestamp_with_time_zone_null(DashboardTokens::LastUsedAt))
                    .col(timestamp_with_time_zone_null(DashboardTokens::ExpiresAt))
                    .col(boolean(DashboardTokens::Active).not_null().default(true))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_dashboard_tokens_user")
                            .from(DashboardTokens::Table, DashboardTokens::UserId)
                            .to(DashboardAllowlist::Table, DashboardAllowlist::UserId)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create index on user_id for faster user token lookups
        manager
            .create_index(
                Index::create()
                    .name("idx_dashboard_tokens_user_id")
                    .table(DashboardTokens::Table)
                    .col(DashboardTokens::UserId)
                    .to_owned(),
            )
            .await?;

        // Create index on token_hash for auth validation
        manager
            .create_index(
                Index::create()
                    .name("idx_dashboard_tokens_hash")
                    .table(DashboardTokens::Table)
                    .col(DashboardTokens::TokenHash)
                    .to_owned(),
            )
            .await?;

        // Create index on active status for filtering
        manager
            .create_index(
                Index::create()
                    .name("idx_dashboard_tokens_active")
                    .table(DashboardTokens::Table)
                    .col(DashboardTokens::Active)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(DashboardTokens::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum DashboardTokens {
    Table,
    TokenId,
    UserId,
    TokenHash,
    Description,
    CreatedAt,
    LastUsedAt,
    ExpiresAt,
    Active,
}

#[derive(DeriveIden)]
enum DashboardAllowlist {
    Table,
    UserId,
}
