#[cfg_attr(coverage_nightly, coverage(off))]
pub use sea_orm_migration::prelude::*;

mod m20250512_000001_create_table_songcalls;
mod m20250528_084830_soundboard_table;
mod m20250610_042820_remote_migration;
mod m20250901_092422_wuwa_pull_tracker;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20250512_000001_create_table_songcalls::Migration),
            Box::new(m20250528_084830_soundboard_table::Migration),
            Box::new(m20250610_042820_remote_migration::Migration),
            Box::new(m20250901_092422_wuwa_pull_tracker::Migration),
        ]
    }
}
