use crate::module::{CommandScope, Module, ModuleEntry};
use anyhow::{Context, anyhow};
use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;
use twilight_model::{
    application::interaction::{InteractionData, InteractionType},
    gateway::event::Event,
    gateway::payload::incoming::InteractionCreate,
    id::{Id, marker::GuildMarker},
};

pub static MANAGER: LazyLock<ModuleManager> = LazyLock::new(ModuleManager::new);

pub struct ModuleManager {
    global_commands: HashMap<&'static str, crate::module::ModuleCommand>,
    guild_commands: HashMap<Id<GuildMarker>, HashMap<&'static str, crate::module::ModuleCommand>>,
    modules: Vec<&'static (dyn crate::module::Module + 'static)>,
}

impl ModuleManager {
    pub fn new() -> Self {
        let mut global_commands = HashMap::new();
        let mut guild_commands: HashMap<Id<GuildMarker>, HashMap<&'static str, _>> = HashMap::new();
        let mut modules = Vec::new();

        for entry in inventory::iter::<ModuleEntry> {
            tracing::info!(
                module = %entry.module.name(),
                "registered"
            );
            modules.push(entry.module);

            for cmd in entry.module.commands() {
                match cmd.scope {
                    CommandScope::Global => {
                        global_commands.insert(cmd.name, cmd);
                    }
                    CommandScope::Guild(guild_id) => {
                        guild_commands
                            .entry(guild_id)
                            .or_default()
                            .insert(cmd.name, cmd);
                    }
                }
            }
        }

        tracing::info!(count = modules.len(), "total modules loaded");

        Self {
            global_commands,
            guild_commands,
            modules,
        }
    }

    pub async fn init(&self) -> anyhow::Result<()> {
        let entries: Vec<&'static dyn Module> =
            inventory::iter::<ModuleEntry>().map(|e| e.module).collect();

        let sorted_modules = sort_modules_topologically(entries)?;

        for module in sorted_modules {
            tracing::info!(module = %module.name(), "initialising module");
            module
                .init()
                .await
                .with_context(|| format!("failed to initialise module '{}'", module.name()))?;
        }

        Ok(())
    }

    pub async fn register_commands(&self) -> anyhow::Result<()> {
        let interaction = crate::CTX.interaction();

        let globals: Vec<_> = self
            .global_commands
            .values()
            .map(|c| (c.builder)())
            .collect();
        interaction.set_global_commands(&globals).await?;

        for (&guild_id, commands) in &self.guild_commands {
            let guild_cmds: Vec<_> = commands.values().map(|c| (c.builder)()).collect();
            interaction
                .set_guild_commands(guild_id, &guild_cmds)
                .await?;
        }

        Ok(())
    }

    pub async fn handle_interaction(
        &self,
        mut interaction: Box<InteractionCreate>,
    ) -> anyhow::Result<()> {
        let command_data = match interaction.data.take() {
            Some(InteractionData::ApplicationCommand(data)) => data,
            _ => return Ok(()),
        };

        let name = command_data.name.as_str();

        let handler = interaction
            .guild_id
            .and_then(|g_id| self.guild_commands.get(&g_id))
            .and_then(|g_map| g_map.get(name))
            .or_else(|| self.global_commands.get(name));

        if let Some(cmd) = handler {
            match interaction.kind {
                InteractionType::ApplicationCommand => (cmd.run)(interaction, command_data).await?,
                InteractionType::ApplicationCommandAutocomplete => {
                    if let Some(auto_fn) = cmd.autocomplete {
                        auto_fn(interaction, command_data).await?;
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }

    pub async fn handle_event(&self, event: &Event) {
        for module in &self.modules {
            if let Err(err) = module.handle_event(event).await {
                tracing::warn!(error = %err, "module event handler failed");
            }
        }
    }
}

fn sort_modules_topologically(
    modules: Vec<&'static dyn Module>,
) -> anyhow::Result<Vec<&'static dyn Module>> {
    let mut map: HashMap<&'static str, &'static dyn Module> = HashMap::new();
    for module in modules {
        if map.insert(module.name(), module).is_some() {
            return Err(anyhow!("duplicate module name found: '{}'", module.name()));
        }
    }

    let mut sorted = Vec::new();
    let mut visited = HashSet::new();
    let mut visiting = HashSet::new();

    fn visit<'a>(
        name: &'a str,
        map: &HashMap<&'static str, &'a dyn Module>,
        visited: &mut HashSet<&'a str>,
        visiting: &mut HashSet<&'a str>,
        sorted: &mut Vec<&'a dyn Module>,
    ) -> anyhow::Result<()> {
        if visiting.contains(name) {
            return Err(anyhow!(
                "circular module dependency detected involving: '{name}'"
            ));
        }

        if !visited.contains(name) {
            visiting.insert(name);

            let module = map
                .get(name)
                .ok_or_else(|| anyhow!("missing module dependency: '{name}'"))?;

            for &dep in module.depends_on() {
                visit(dep, map, visited, visiting, sorted)?;
            }

            visiting.remove(name);
            visited.insert(name);
            sorted.push(*module);
        }

        Ok(())
    }

    for &name in map.keys() {
        if !visited.contains(name) {
            visit(name, &map, &mut visited, &mut visiting, &mut sorted)?;
        }
    }

    Ok(sorted)
}
