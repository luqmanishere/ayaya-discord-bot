//! handwritten

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "require_category_role")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false, column_type = "Binary(16)")]
    pub entry_id: Uuid,
    pub server_id: u64,
    pub role_id: u64,
    pub category: String,
}

#[derive(Clone, Copy, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
