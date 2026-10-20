pub mod fun;
pub mod ping;

use std::pin::Pin;

use async_trait::async_trait;
use twilight_model::{
    application::{command::Command, interaction::application_command::CommandData},
    gateway::{event::Event, payload::incoming::InteractionCreate},
    id::{Id, marker::GuildMarker},
};

pub enum CommandScope {
    Global,
    #[allow(unused)]
    Guild(Id<GuildMarker>),
}

pub type CommandDataBox = Box<CommandData>;
pub type InteractionBox = Box<InteractionCreate>;

pub type BoxFuture<'a, T = anyhow::Result<()>> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub type CommandFn = fn(InteractionBox, CommandDataBox) -> BoxFuture<'static>;

pub struct ModuleCommand {
    pub name: &'static str,
    pub scope: CommandScope,
    pub builder: fn() -> Command,
    pub run: CommandFn,
    pub autocomplete: Option<CommandFn>,
}

#[async_trait]
pub trait Module: Send + Sync {
    fn name(&self) -> &'static str;

    fn commands(&self) -> Vec<ModuleCommand> {
        Vec::new()
    }

    async fn handle_event(&self, _event: &Event) -> anyhow::Result<()> {
        Ok(())
    }
}

pub struct ModuleEntry {
    pub module: &'static (dyn Module + 'static),
}

inventory::collect!(ModuleEntry);
