use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(WuwaUser::Table)
                    .if_not_exists()
                    .col(big_unsigned(WuwaUser::UserId))
                    .col(integer(WuwaUser::WuwaUserId).primary_key())
                    .to_owned(),
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .table(WuwaResource::Table)
                    .if_not_exists()
                    .col(integer(WuwaResource::ResourceId).primary_key().unique_key())
                    .col(string(WuwaResource::ResourceName).not_null().unique_key())
                    .col(string(WuwaResource::ResourceType))
                    .col(string(WuwaResource::ResourcePortraitPath))
                    .to_owned(),
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .table(WuwaPull::Table)
                    .if_not_exists()
                    .col(uuid(WuwaPull::Id).primary_key())
                    .col(integer(WuwaPull::WuwaUserId))
                    .col(integer(WuwaPull::PullType).not_null())
                    .col(integer(WuwaPull::ResourceId).not_null())
                    .col(integer(WuwaPull::QualityLevel).not_null())
                    .col(integer(WuwaPull::Count).not_null())
                    .col(timestamp_with_time_zone(WuwaPull::Time).not_null())
                    .foreign_key(
                        ForeignKeyCreateStatement::new()
                            .from_tbl(WuwaPull::Table)
                            .from_col(WuwaPull::ResourceId)
                            .to_tbl(WuwaResource::Table)
                            .to_col(WuwaResource::ResourceId),
                    )
                    .foreign_key(
                        ForeignKeyCreateStatement::new()
                            .from_tbl(WuwaPull::Table)
                            .from_col(WuwaPull::WuwaUserId)
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
            .drop_table(Table::drop().table(WuwaPull::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(WuwaResource::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(WuwaUser::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum WuwaPull {
    Table,
    Id,
    WuwaUserId,
    PullType,
    ResourceId,
    QualityLevel,
    Count,
    Time,
}

#[derive(DeriveIden)]
enum WuwaResource {
    Table,
    ResourceId,
    ResourceName,
    ResourceType,
    ResourcePortraitPath,
}

#[derive(DeriveIden)]
#[expect(clippy::enum_variant_names)]
enum WuwaUser {
    Table,
    UserId,
    WuwaUserId,
}
