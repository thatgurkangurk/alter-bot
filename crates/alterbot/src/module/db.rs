use crate::module::{Module, ModuleEntry};
use sea_orm::Database;

pub struct DatabaseModule;

#[async_trait::async_trait]
impl Module for DatabaseModule {
    fn name(&self) -> &'static str {
        "database"
    }

    async fn init(&self) -> anyhow::Result<()> {
        let config = crate::context::config();

        tracing::info!("connecting to db...");
        let db = Database::connect(&config.db.uri).await?;

        let crate_name = module_path!().split("::").next().unwrap();
        let pattern = format!("{crate_name}::entity::*");

        tracing::info!(pattern = %pattern, "syncing db schema");
        db.get_schema_registry(&pattern).sync(&db).await?;
        tracing::info!("db schema successfully synced");

        crate::context::set_db(db);
        Ok(())
    }
}

inventory::submit! {
    ModuleEntry {
        module: &DatabaseModule
    }
}
