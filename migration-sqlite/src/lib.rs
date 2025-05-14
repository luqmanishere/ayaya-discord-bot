pub use sea_orm_migration::prelude::*;

mod m20250512_000001_create_table_songcalls;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(m20250512_000001_create_table_songcalls::Migration)]
    }
}
