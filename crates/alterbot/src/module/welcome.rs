use crate::{
    CTX, entity,
    module::{Module, ModuleEntry},
};
use twilight_model::{gateway::event::Event, id::Id};

pub struct WelcomerModule;

#[async_trait::async_trait]
impl Module for WelcomerModule {
    fn name(&self) -> &'static str {
        "welcomer"
    }

    async fn handle_event(&self, event: &Event) -> anyhow::Result<()> {
        let Event::MemberAdd(member) = event else {
            return Ok(());
        };

        let db_guild =
            entity::guild::Entity::get_or_initialise(&CTX.database, member.guild_id).await?;
        tracing::debug!(
            guild_id = %db_guild.id,
            member_id = %member.user.id,
            "new member in guild"
        );

        let Some(channel_id) = db_guild.welcome_channel_id else {
            return Ok(());
        };

        let Some(guild) = CTX.cache.guild(member.guild_id) else {
            tracing::warn!(guild_id = %member.guild_id, "failed to resolve guild for welcome message");
            return Ok(());
        };

        let channel_id_u64 = channel_id as u64;
        let content = format!("welcome to {}, <@{}>!", guild.name(), member.user.id);

        match CTX
            .http
            .create_message(Id::new(channel_id_u64))
            .content(&content)
            .await
        {
            Ok(_) => tracing::info!("welcome message sent successfully"),
            Err(err) => tracing::error!(?err, %channel_id_u64, "failed to send welcome message"),
        }

        Ok(())
    }
}

inventory::submit! {
    ModuleEntry {
        module: &WelcomerModule
    }
}
