use crate::{
    consts,
    module::{CommandScope, Module, ModuleCommand, ModuleEntry},
    util::{get_bot_user, rgb_to_packed_u32},
};
use anyhow::Context;
use twilight_model::{
    application::{
        command::{Command, CommandType},
        interaction::application_command::CommandData,
    },
    gateway::payload::incoming::InteractionCreate,
    http::interaction::{InteractionResponse, InteractionResponseData, InteractionResponseType},
    util::Timestamp,
};
use twilight_util::builder::{
    command::CommandBuilder,
    embed::{EmbedBuilder, EmbedFieldBuilder, ImageSource},
};

pub struct MiscModule;

pub fn command() -> Command {
    CommandBuilder::new(
        "info",
        "get misc info about the bot",
        CommandType::ChatInput,
    )
    .build()
}

pub async fn run(
    interaction: Box<InteractionCreate>,
    _data: Box<CommandData>,
) -> anyhow::Result<()> {
    let bot_user = get_bot_user().await?;

    let timestamp =
        Timestamp::from_secs(chrono::Utc::now().timestamp()).context("valid timestamp")?;

    let avatar_url = match &bot_user.avatar {
        Some(hash) => format!(
            "https://cdn.discordapp.com/avatars/{}/{}.webp",
            bot_user.id, hash
        ),
        None => {
            let default_index = (bot_user.id.get() >> 22) % 5;
            format!("https://cdn.discordapp.com/embed/avatars/{default_index}.webp")
        }
    };

    let embed = EmbedBuilder::new()
        .title("alter bot")
        .field(
            EmbedFieldBuilder::new("version", consts::VERSION)
                .inline()
                .build(),
        )
        .field(
            EmbedFieldBuilder::new("authors", consts::AUTHORS_RAW.replace(':', ", "))
                .inline()
                .build(),
        )
        .field(EmbedFieldBuilder::new("repository", consts::REPOSITORY).build())
        .color(rgb_to_packed_u32(236, 253, 245))
        .timestamp(timestamp)
        .thumbnail(ImageSource::url(avatar_url).context("failed to get bot avatar")?)
        .build();

    let data = InteractionResponseData {
        embeds: Some(vec![embed]),
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
impl Module for MiscModule {
    fn name(&self) -> &'static str {
        "misc"
    }

    fn commands(&self) -> Vec<ModuleCommand> {
        vec![ModuleCommand {
            name: "info",
            scope: CommandScope::Global,
            builder: command,
            run: |i, d| Box::pin(run(i, d)),
            autocomplete: None,
        }]
    }
}

inventory::submit! {
    ModuleEntry {
        module: &MiscModule
    }
}
