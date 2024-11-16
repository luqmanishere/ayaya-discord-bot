use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // some command categories require roles
        manager
            .create_table(
                Table::create()
                    .table(RequireCategoryRole::Table)
                    .if_not_exists()
                    .col(pk_uuid(RequireCategoryRole::EntryId))
                    .col(big_unsigned(RequireCategoryRole::ServerId).not_null())
                    .col(big_unsigned(RequireCategoryRole::RoleId).not_null())
                    .col(string(RequireCategoryRole::Category).not_null())
                    .index(
                        Index::create()
                            .col(RequireCategoryRole::ServerId)
                            .col(RequireCategoryRole::RoleId)
                            .col(RequireCategoryRole::Category)
                            .unique(),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(RequireCategoryRole::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum RequireCategoryRole {
    Table,
    EntryId,
    ServerId,
    RoleId,
    Category,
}
