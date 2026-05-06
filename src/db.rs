use crate::Config;
use sea_orm::{Database, DatabaseConnection};

pub async fn create_db(config: &Config) -> anyhow::Result<DatabaseConnection> {
    let db = Database::connect(&config.db.uri).await?;

    db.get_schema_registry("alter_bot::models::*")
        .sync(&db)
        .await?;

    Ok(db)
}
