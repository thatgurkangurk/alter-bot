use crate::{entity, module::Module, register_module};
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
                entity::guild::Entity::get_or_initialise(crate::context::db(), guild.id()).await?;
            tracing::info!(
                guild_id = %db_guild.id,
                "new guild!"
            );
        }

        Ok(())
    }
}

register_module!(GuildModule);
