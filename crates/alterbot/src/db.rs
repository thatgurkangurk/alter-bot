use crate::config::ConfigManager;
use sea_orm::{ConnectionTrait, Database, DatabaseConnection};
use tracing::info;

async fn auto_migrate_polls_to_v2(db: &sea_orm::DatabaseConnection) -> anyhow::Result<()> {
    let check = db
        .execute_unprepared("SELECT choice FROM votes LIMIT 1;")
        .await;

    if check.is_err() {
        // no migration needed!
        return Ok(());
    }

    info!("legacy polls schema detected. migrating to v2...");

    let migrate_sql = "
        BEGIN;

        CREATE TABLE IF NOT EXISTS poll_options (
            id UUID PRIMARY KEY,
            poll_id UUID NOT NULL,
            label VARCHAR NOT NULL,
            weight DOUBLE PRECISION NOT NULL
        );
        ALTER TABLE votes ADD COLUMN IF NOT EXISTS option_id UUID;

        INSERT INTO poll_options (id, poll_id, label, weight)
        SELECT gen_random_uuid(), id, 'Yes', 1.0 FROM polls
        ON CONFLICT DO NOTHING;

        INSERT INTO poll_options (id, poll_id, label, weight)
        SELECT gen_random_uuid(), id, 'No', 1.0 FROM polls
        ON CONFLICT DO NOTHING;

        INSERT INTO poll_options (id, poll_id, label, weight)
        SELECT gen_random_uuid(), id, 'HardNo', 1.5 FROM polls WHERE has_hard_no = true
        ON CONFLICT DO NOTHING;

        UPDATE votes v
        SET option_id = po.id
        FROM poll_options po
        WHERE v.poll_id = po.poll_id
          AND v.choice = po.label
          AND v.option_id IS NULL;

        ALTER TABLE votes ALTER COLUMN option_id SET NOT NULL;
        ALTER TABLE votes DROP COLUMN choice;
        ALTER TABLE polls DROP COLUMN has_hard_no;

        COMMIT;
    ";

    db.execute_unprepared(migrate_sql).await?;

    info!("Data migration to dynamic polls complete!");
    Ok(())
}

pub async fn create_db(config_manager: &ConfigManager) -> anyhow::Result<DatabaseConnection> {
    let db = Database::connect(&config_manager.get().await.db.uri).await?;

    auto_migrate_polls_to_v2(&db).await?;

    db.get_schema_registry("alter_bot::models::*")
        .sync(&db)
        .await?;

    Ok(db)
}
