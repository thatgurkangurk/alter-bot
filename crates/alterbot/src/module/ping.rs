use anyhow::Result;
use twilight_interactions::command::{CommandModel, CreateCommand};
use twilight_model::{
    application::{command::Command, interaction::application_command::CommandData},
    channel::message::MessageFlags,
    gateway::payload::incoming::InteractionCreate,
    http::interaction::{InteractionResponse, InteractionResponseData, InteractionResponseType},
};

use crate::{
    module::{CommandScope, Module, ModuleCommand},
    register_module,
};

pub struct PingModule;

#[derive(CreateCommand, CommandModel)]
#[command(name = "ping", desc = "Ping the bot")]
pub struct PingCommand;

pub fn command() -> Command {
    PingCommand::create_command().into()
}

pub async fn run(interaction: Box<InteractionCreate>, data: Box<CommandData>) -> Result<()> {
    let _cmd = PingCommand::from_interaction((*data).into())?;

    let response_data = InteractionResponseData {
        content: Some("Pong!".to_owned()),
        flags: Some(MessageFlags::EPHEMERAL),
        ..Default::default()
    };

    let response = InteractionResponse {
        kind: InteractionResponseType::ChannelMessageWithSource,
        data: Some(response_data),
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
        "ping"
    }

    fn commands(&self) -> Vec<ModuleCommand> {
        vec![ModuleCommand {
            name: "ping",
            scope: CommandScope::Global,
            builder: command,
            run: |i, d| Box::pin(run(i, d)),
            autocomplete: None,
        }]
    }
}

register_module!(PingModule);
