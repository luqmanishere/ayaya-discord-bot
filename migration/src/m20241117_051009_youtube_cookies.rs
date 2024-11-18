use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Replace the sample below with your own migration scripts

        manager
            .create_table(
                Table::create()
                    .table(YoutubeCookies::Table)
                    .if_not_exists()
                    .col(pk_auto(YoutubeCookies::EntryId))
                    .col(timestamp_with_time_zone(YoutubeCookies::Date))
                    .col(blob(YoutubeCookies::Cookies))
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Replace the sample below with your own migration scripts

        manager
            .drop_table(Table::drop().table(YoutubeCookies::Table).to_owned())
            .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum YoutubeCookies {
    Table,
    EntryId,
    Date,
    Cookies,
}
