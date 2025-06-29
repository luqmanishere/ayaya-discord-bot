//! `SeaORM` Entity, @generated by sea-orm-codegen 1.1.11

use bincode::{Decode, Encode};
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Encode, Decode)]
#[sea_orm(table_name = "require_command_role")]
pub struct Model {
    #[bincode(with_serde)]
    #[sea_orm(primary_key, auto_increment = false)]
    pub entry_id: Uuid,
    pub server_id: i64,
    pub role_id: i64,
    pub command: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
