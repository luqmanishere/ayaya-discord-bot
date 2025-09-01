use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Messages::Table)
                    .col(big_unsigned(Messages::MessageId).primary_key())
                    .col(big_unsigned(Messages::GuildId))
                    .col(big_unsigned(Messages::ChannelId))
                    .col(big_unsigned(Messages::AuthorId))
                    .col(timestamp_with_time_zone(Messages::Timestamp))
                    .col(json_binary(Messages::Message))
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Messages::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Messages {
    Table,
    MessageId,
    GuildId,
    ChannelId,
    AuthorId,
    Timestamp,
    Message,
}
