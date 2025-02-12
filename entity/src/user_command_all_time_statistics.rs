//! `SeaORM` Entity, @generated by sea-orm-codegen 1.1.0

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "user_command_all_time_statistics")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub server_id: u64,
    #[sea_orm(primary_key, auto_increment = false)]
    pub user_id: u64,
    #[sea_orm(primary_key, auto_increment = false)]
    pub command: String,
    pub count: u32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
