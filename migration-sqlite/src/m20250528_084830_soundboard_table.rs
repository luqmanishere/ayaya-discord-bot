use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Sounds::Table)
                    .if_not_exists()
                    .col(uuid(Sounds::SoundId).primary_key())
                    .col(big_unsigned(Sounds::UserId))
                    .col(big_unsigned(Sounds::UploadedServerId))
                    .col(string(Sounds::SoundName))
                    .col(boolean(Sounds::Public))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(UploadNoticed::Table)
                    .col(big_unsigned(UploadNoticed::UserId).primary_key())
                    .col(boolean(UploadNoticed::Agreed))
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Sounds::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Sounds {
    Table,
    UserId,
    UploadedServerId,
    SoundId,
    SoundName,
    Public,
}

#[derive(DeriveIden)]
enum UploadNoticed {
    Table,
    UserId,
    Agreed,
}
