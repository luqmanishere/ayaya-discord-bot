pub use sea_orm_migration::prelude::*;

mod m20220101_000001_create_tables_initial;
mod m20241115_044942_require_category_role;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_create_tables_initial::Migration),
            Box::new(m20241115_044942_require_category_role::Migration),
        ]
    }
}
