use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(SongQueues::Table)
                    .if_not_exists()
                    .col(big_unsigned(SongQueues::ServerId))
                    .col(big_unsigned(SongQueues::UserId))
                    .col(string(SongQueues::YoutubeId))
                    .col(string_null(SongQueues::Description))
                    .col(integer(SongQueues::Count))
                    .col(timestamp_with_time_zone_null(SongQueues::LastUpdate))
                    .primary_key(
                        Index::create()
                            .col(SongQueues::ServerId)
                            .col(SongQueues::UserId)
                            .col(SongQueues::YoutubeId)
                            .unique(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(UserPlayQueries::Table)
                    .if_not_exists()
                    .col(big_unsigned(UserPlayQueries::ServerId))
                    .col(big_unsigned(UserPlayQueries::UserId))
                    .col(string(UserPlayQueries::Query))
                    .col(string(UserPlayQueries::QueryType))
                    .col(string(UserPlayQueries::Description))
                    .col(big_unsigned(UserPlayQueries::Count))
                    .primary_key(
                        Index::create()
                            .col(UserPlayQueries::ServerId)
                            .col(UserPlayQueries::UserId)
                            .col(UserPlayQueries::Query)
                            .unique(),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(SongQueues::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(UserPlayQueries::Table).to_owned())
            .await?;

        Ok(())
    }
}

/// Tracks the amount of times an id is queued.
#[derive(DeriveIden)]
enum SongQueues {
    Table,
    ServerId,
    UserId,
    YoutubeId,
    Description,
    Count,
    LastUpdate,
}

/// Counts the queries provided by users to the play command. Used to supplement autocomplete and stats.
#[derive(DeriveIden)]
enum UserPlayQueries {
    Table,
    UserId,
    ServerId,
    Query,
    Description,
    QueryType,
    Count,
}
