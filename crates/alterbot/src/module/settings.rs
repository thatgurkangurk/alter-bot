use crate::{
    entity,
    module::{CommandScope, Module, ModuleCommand},
    register_module,
};
use anyhow::Result;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, IntoActiveModel};
use twilight_interactions::command::{CommandModel, CreateCommand};
use twilight_model::{
    application::{command::Command, interaction::application_command::CommandData},
    gateway::payload::incoming::InteractionCreate,
    guild::Permissions,
    http::interaction::{InteractionResponse, InteractionResponseData, InteractionResponseType},
    id::{Id, marker::ChannelMarker},
};

pub struct ServerSettingsModule;

#[derive(CreateCommand, CommandModel)]
#[command(
    name = "settings",
    desc = "configure server settings",
    default_permissions = "settings_permissions",
    contexts = "guild"
)]
pub enum SettingsCommand {
    #[command(name = "set")]
    Set(SetSettingsSubcommand),
}

fn settings_permissions() -> Permissions {
    Permissions::MANAGE_GUILD
}

#[derive(CreateCommand, CommandModel)]
#[command(name = "set", desc = "modify server settings")]
pub enum SetSettingsSubcommand {
    #[command(name = "welcome_channel")]
    WelcomeChannel(WelcomeChannelOptions),
    #[command(name = "poll_log_channel")]
    PollLogChannel(PollLogChannelOptions),
}

#[derive(CreateCommand, CommandModel)]
#[command(
    name = "welcome_channel",
    desc = "set the channel for welcome messages"
)]
pub struct WelcomeChannelOptions {
    #[command(
        desc = "the channel to send welcome messages in",
        channel_types = "guild_text"
    )]
    pub channel: Id<ChannelMarker>,
}

#[derive(CreateCommand, CommandModel)]
#[command(
    name = "poll_log_channel",
    desc = "set the channel or forum thread for poll logs"
)]
pub struct PollLogChannelOptions {
    #[command(
        desc = "the channel or forum thread to post poll logs in",
        channel_types = "guild_text guild_announcement public_thread private_thread"
    )]
    pub channel: Id<ChannelMarker>,
}

pub fn command() -> Command {
    SettingsCommand::create_command().into()
}

pub async fn run(interaction: Box<InteractionCreate>, data: Box<CommandData>) -> Result<()> {
    let guild_id = match interaction.guild_id {
        Some(id) => id,
        None => {
            send_ephemeral_response(&interaction, "this command can only be used in a server.")
                .await?;
            return Ok(());
        }
    };

    let command = SettingsCommand::from_interaction((*data).into())?;

    match command {
        SettingsCommand::Set(SetSettingsSubcommand::WelcomeChannel(options)) => {
            let channel_id = options.channel;
            let db_channel_id = channel_id.get() as i64;

            let db_guild =
                entity::guild::Entity::get_or_initialise(crate::context::db(), guild_id).await?;

            let mut active_guild: entity::guild::ActiveModel = db_guild.into_active_model();
            active_guild.welcome_channel_id = Set(Some(db_channel_id));
            active_guild.update(crate::context::db()).await?;

            send_ephemeral_response(
                &interaction,
                &format!("welcome channel successfully updated to <#{channel_id}>."),
            )
            .await?;
        }
        SettingsCommand::Set(SetSettingsSubcommand::PollLogChannel(options)) => {
            let channel_id = options.channel;
            let db_channel_id = channel_id.get() as i64;

            let db_guild =
                entity::guild::Entity::get_or_initialise(crate::context::db(), guild_id).await?;

            let mut active_guild: entity::guild::ActiveModel = db_guild.into_active_model();
            active_guild.poll_log_channel_id = Set(Some(db_channel_id));
            active_guild.update(crate::context::db()).await?;

            send_ephemeral_response(
                &interaction,
                &format!("poll log channel successfully updated to <#{channel_id}>."),
            )
            .await?;
        }
    }

    Ok(())
}

async fn send_ephemeral_response(interaction: &InteractionCreate, content: &str) -> Result<()> {
    let response = InteractionResponse {
        kind: InteractionResponseType::ChannelMessageWithSource,
        data: Some(InteractionResponseData {
            content: Some(content.to_owned()),
            flags: Some(twilight_model::channel::message::MessageFlags::EPHEMERAL),
            ..Default::default()
        }),
    };

    crate::CTX
        .interaction()
        .create_response(interaction.id, &interaction.token, &response)
        .await?;

    Ok(())
}

#[async_trait::async_trait]
impl Module for ServerSettingsModule {
    fn name(&self) -> &'static str {
        "server_settings"
    }

    fn commands(&self) -> Vec<ModuleCommand> {
        vec![ModuleCommand {
            name: "settings",
            scope: CommandScope::Global,
            builder: command,
            run: |i, d| Box::pin(run(i, d)),
            autocomplete: None,
        }]
    }
}

register_module!(ServerSettingsModule);
