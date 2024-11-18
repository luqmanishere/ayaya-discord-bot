use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "youtube_cookies")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub entry_id: i32,
    pub date: TimeDateTimeWithTimeZone,
    #[sea_orm(column_type = "Binary(16)")]
    pub cookies: Vec<u8>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
