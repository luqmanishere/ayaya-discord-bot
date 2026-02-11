use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // rename the akendpull table
        manager
            .rename_table(
                Table::rename()
                    .table(AkEndPull::Table, AkEndCharPull::Table)
                    .to_owned(),
            )
            .await?;

        // add the new weapon table
        manager
            .create_table(
                Table::create()
                    .table(AkEndWeapPull::Table)
                    .if_not_exists()
                    .col(uuid(AkEndWeapPull::Id).primary_key())
                    .col(integer(AkEndWeapPull::UserId).not_null())
                    .col(integer(AkEndWeapPull::AkEndUserId).not_null())
                    .col(string(AkEndWeapPull::PoolType).not_null())
                    .col(string(AkEndWeapPull::PoolId).not_null())
                    .col(string(AkEndWeapPull::PoolName).not_null())
                    .col(string(AkEndWeapPull::WeaponId).not_null())
                    .col(string(AkEndWeapPull::WeaponName).not_null())
                    .col(string(AkEndWeapPull::WeaponType).not_null())
                    .col(integer(AkEndWeapPull::Rarity).not_null())
                    .col(boolean(AkEndWeapPull::IsNew).not_null())
                    .col(timestamp_with_time_zone(AkEndWeapPull::Time).not_null())
                    .col(string(AkEndWeapPull::SeqId).not_null())
                    .foreign_key(
                        ForeignKeyCreateStatement::new()
                            .from_tbl(AkEndWeapPull::Table)
                            .from_col(AkEndWeapPull::AkEndUserId)
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
            .drop_table(Table::drop().table(AkEndWeapPull::Table).to_owned())
            .await?;

        //rerename the table
        manager
            .rename_table(
                Table::rename()
                    .table(AkEndCharPull::Table, AkEndPull::Table)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum AkEndPull {
    Table,
}

#[derive(DeriveIden)]
enum AkEndCharPull {
    Table,
}

#[derive(DeriveIden)]
enum AkEndWeapPull {
    Table,
    Id,
    UserId,
    AkEndUserId,
    PoolType,
    PoolId,
    PoolName,
    WeaponId,
    WeaponName,
    WeaponType,
    Rarity,
    IsNew,
    Time,
    SeqId,
}

#[derive(DeriveIden)]
enum AkEndUser {
    Table,
    AkEndUserId,
}
