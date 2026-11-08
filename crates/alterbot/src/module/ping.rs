use crate::module::{CommandScope, Module, ModuleCommand, ModuleEntry};
use twilight_model::{
    application::{
        command::{Command, CommandType},
        interaction::application_command::CommandData,
    },
    channel::message::MessageFlags,
    gateway::payload::incoming::InteractionCreate,
    http::interaction::{InteractionResponse, InteractionResponseData, InteractionResponseType},
};
use twilight_util::builder::command::CommandBuilder;

pub struct PingModule;

pub const NAME: &str = "ping";

pub fn command() -> Command {
    CommandBuilder::new(NAME, "Ping the bot", CommandType::ChatInput).build()
}

pub async fn run(
    interaction: Box<InteractionCreate>,
    _data: Box<CommandData>,
) -> anyhow::Result<()> {
    let data = InteractionResponseData {
        content: Some("Pong!".to_owned()),
        flags: Some(MessageFlags::EPHEMERAL),
        ..Default::default()
    };

    let response = InteractionResponse {
        kind: InteractionResponseType::ChannelMessageWithSource,
        data: Some(data),
    };

    crate::CTX
        .interaction()
        .create_response(interaction.id, &interaction.token, &response)
        .await?;

    Ok(())
}

#[async_trait::async_trait]
impl Module for PingModule {
    fn name(&self) -> &'static str {
        NAME
    }

    fn commands(&self) -> Vec<ModuleCommand> {
        vec![ModuleCommand {
            name: NAME,
            scope: CommandScope::Global,
            builder: command,
            run: |i, d| Box::pin(run(i, d)),
            autocomplete: None,
        }]
    }
}

inventory::submit! {
    ModuleEntry {
        module: &PingModule
    }
}
