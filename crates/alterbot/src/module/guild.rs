use crate::{
    CTX, entity,
    module::{Module, ModuleEntry},
};
use twilight_model::gateway::event::Event;

pub struct GuildModule;

#[async_trait::async_trait]
impl Module for GuildModule {
    fn name(&self) -> &'static str {
        "guild"
    }

    async fn handle_event(&self, event: &Event) -> anyhow::Result<()> {
        if let Event::GuildCreate(guild) = event {
            let db_guild =
                entity::guild::Entity::get_or_initialise(&CTX.database, guild.id()).await?;
            tracing::info!(
                guild_id = %db_guild.id,
                "new guild!"
            );
        }

        Ok(())
    }
}

inventory::submit! {
    ModuleEntry {
        module: &GuildModule
    }
}
