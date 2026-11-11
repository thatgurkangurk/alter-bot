use std::{sync::Arc, time::Duration};

use anyhow::{Context, Result};
use chrono::Utc;
use sea_orm::EntityTrait;
use tracing::{Instrument, info_span};

use twilight_interactions::command::{CommandModel, CreateCommand};
use twilight_model::{
    application::{command::Command, interaction::application_command::CommandData},
    channel::message::MessageFlags,
    gateway::{event::Event, payload::incoming::InteractionCreate},
    http::interaction::{InteractionResponse, InteractionResponseData, InteractionResponseType},
    id::{Id, marker::ChannelMarker},
};

use super::poll_manager::CreatePollParams;
use crate::{
    CTX, context,
    entity::{guild, guild_member, poll, vote},
    module::{CommandScope, Module, ModuleCommand, poll::PollManager},
    register_module,
};

pub struct PollModule;

#[derive(CreateCommand, CommandModel)]
#[command(name = "poll", desc = "create a yes/no poll")]
pub struct PollCommand {
    #[command(desc = "the question to ask")]
    pub question: String,
    #[command(desc = "duration (e.g., '10m', '2h', '1d', '6h')")]
    pub duration: String,
    #[command(desc = "optional description or context for the poll")]
    pub description: Option<String>,
    #[command(
        desc = "channel to post the poll in (defaults to current channel)",
        channel_types = "guild_text guild_announcement"
    )]
    pub channel: Option<Id<ChannelMarker>>,
}

pub fn command() -> Command {
    PollCommand::create_command().into()
}

pub async fn run(interaction: Box<InteractionCreate>, data: Box<CommandData>) -> Result<()> {
    let span = info_span!("poll_command_run", interaction_id = %interaction.id);
    async move {
        tracing::info!("handling /poll command execution");
        let cmd = PollCommand::from_interaction((*data).into())?;

        let guild_id = match interaction.guild_id {
            Some(id) => id,
            None => {
                return send_ephemeral_response(
                    &interaction,
                    "polls can only be created within a server.",
                )
                .await;
            }
        };

        let user = match interaction.author() {
            Some(user) => user,
            None => {
                return send_ephemeral_response(
                    &interaction,
                    "could not determine interaction author.",
                )
                .await;
            }
        };

        let duration = match parse_duration(&cmd.duration) {
            Ok(d) => d,
            Err(err) => {
                return send_ephemeral_response(
                    &interaction,
                    &format!("invalid duration: {}", err),
                )
                .await;
            }
        };

        if duration > Duration::from_secs(7 * 24 * 60 * 60) {
            return send_ephemeral_response(
                &interaction,
                "poll duration cannot exceed 7 days (1 week).",
            )
            .await;
        }

        if duration < Duration::from_secs(60) {
            return send_ephemeral_response(
                &interaction,
                "poll duration must be at least 1 minute.",
            )
            .await;
        }

        defer_ephemeral(&interaction).await?;

        guild::Entity::get_or_initialise(crate::context::db(), guild_id)
            .await
            .context("failed to initialize guild record")?;

        guild_member::Entity::get_or_initialise(crate::context::db(), guild_id, user.id)
            .await
            .context("failed to initialize guild member record")?;

        let end_time = Utc::now()
            + chrono::Duration::from_std(duration)
                .context("failed to convert duration to chrono format")?;

        let channel_id = match cmd.channel {
            Some(id) => id,
            None => interaction
                .channel
                .as_ref()
                .map(|c| c.id)
                .ok_or_else(|| anyhow::anyhow!("missing channel in interaction"))?,
        };

        let (author_display_name, author_avatar) = match &interaction.member {
            Some(member) => (
                member
                    .nick
                    .clone()
                    .or_else(|| user.global_name.clone())
                    .unwrap_or_else(|| user.name.clone()),
                member.avatar.or(user.avatar),
            ),
            None => (
                user.global_name
                    .clone()
                    .unwrap_or_else(|| user.name.clone()),
                user.avatar,
            ),
        };

        let create_params = CreatePollParams {
            channel_id,
            guild_id,
            author_id: user.id,
            author_name: user.name.clone(),
            author_display_name,
            author_avatar,
            question: cmd.question,
            description: cmd.description,
            end_time,
        };

        match crate::context::poll_manager()
            .create_poll(create_params)
            .await
        {
            Ok(_) => send_followup(&interaction, "poll created successfully!").await?,
            Err(err) => {
                tracing::error!(error = ?err, "failed to create poll");
                send_followup(
                    &interaction,
                    "failed to create poll due to an internal error.",
                )
                .await?;
            }
        }

        Ok(())
    }
    .instrument(span)
    .await
}

#[async_trait::async_trait]
impl Module for PollModule {
    fn name(&self) -> &'static str {
        "poll"
    }

    fn depends_on(&self) -> &'static [&'static str] {
        &["database"]
    }

    async fn init(&self) -> Result<()> {
        let db = crate::context::db().clone();
        let poll_manager = Arc::new(PollManager::new(db));

        crate::context::set_poll_manager(Arc::clone(&poll_manager));

        poll_manager.start_tasks();

        tracing::info!("poll expiration checker started");

        Ok(())
    }

    fn commands(&self) -> Vec<ModuleCommand> {
        vec![ModuleCommand {
            name: "poll",
            scope: CommandScope::Global,
            builder: command,
            run: |i, d| Box::pin(run(i, d)),
            autocomplete: None,
        }]
    }

    async fn handle_event(&self, event: &Event) -> Result<()> {
        let Event::InteractionCreate(interaction) = event else {
            return Ok(());
        };

        let Some(twilight_model::application::interaction::InteractionData::MessageComponent(data)) =
            &interaction.data
        else {
            return Ok(());
        };

        let choice = match data.custom_id.as_str() {
            "poll_vote_yes" => true,
            "poll_vote_no" => false,
            _ => return Ok(()),
        };

        let span = info_span!(
            "poll_vote_event",
            interaction_id = %interaction.id,
            custom_id = %data.custom_id,
            choice = choice
        );

        async move {
            let defer_update = InteractionResponse {
                kind: InteractionResponseType::DeferredUpdateMessage,
                data: None,
            };

            if let Err(err) = CTX
                .interaction()
                .create_response(interaction.id, &interaction.token, &defer_update)
                .await
            {
                tracing::error!(error = ?err, "failed to acknowledge interaction");
                return Ok(());
            }

            let (Some(user_id), Some(guild_id), Some(message)) = (
                interaction.author_id(),
                interaction.guild_id,
                interaction.message.as_ref(),
            ) else {
                return Ok(());
            };

            guild::Entity::get_or_initialise(context::db(), guild_id)
                .await
                .context("failed to initialise guild")?;

            guild_member::Entity::get_or_initialise(context::db(), guild_id, user_id)
                .await
                .context("failed to initialise guild member")?;

            let message_id = message.id.get() as i64;
            let poll_option = poll::Entity::find_by_id(message_id)
                .one(context::db())
                .await
                .context("failed to query poll")?;

            let Some(poll_model) = poll_option.filter(|p| !p.is_closed) else {
                send_followup(interaction, "this poll is closed or no longer exists.").await?;
                return Ok(());
            };

            if poll_model.is_expired() {
                send_followup(interaction, "this poll has expired.").await?;
                return Ok(());
            }

            vote::Entity::cast_vote(context::db(), message.id, user_id, choice)
                .await
                .context("failed to cast vote")?;

            if let Err(err) = context::poll_manager().update_poll_embed(message_id).await {
                tracing::error!(error = ?err, "failed to update poll embed after vote");
            }

            let vote_str = if choice { "Yes" } else { "No" };
            send_followup(
                interaction,
                &format!("your vote (**{}**) has been recorded !", vote_str),
            )
            .await?;

            Ok(())
        }
        .instrument(span)
        .await
    }
}

fn parse_duration(input: &str) -> Result<Duration> {
    humantime::parse_duration(input.trim())
        .with_context(|| format!("failed to parse duration '{}'", input))
}

async fn defer_ephemeral(interaction: &InteractionCreate) -> Result<()> {
    let defer_response = InteractionResponse {
        kind: InteractionResponseType::DeferredChannelMessageWithSource,
        data: Some(InteractionResponseData {
            flags: Some(MessageFlags::EPHEMERAL),
            ..Default::default()
        }),
    };

    CTX.interaction()
        .create_response(interaction.id, &interaction.token, &defer_response)
        .await?;

    Ok(())
}

async fn send_ephemeral_response(interaction: &InteractionCreate, content: &str) -> Result<()> {
    let response_data = InteractionResponseData {
        content: Some(content.to_owned()),
        flags: Some(MessageFlags::EPHEMERAL),
        ..Default::default()
    };

    let response = InteractionResponse {
        kind: InteractionResponseType::ChannelMessageWithSource,
        data: Some(response_data),
    };

    CTX.interaction()
        .create_response(interaction.id, &interaction.token, &response)
        .await?;

    Ok(())
}

async fn send_followup(interaction: &InteractionCreate, content: &str) -> Result<()> {
    CTX.interaction()
        .create_followup(&interaction.token)
        .content(content)
        .flags(MessageFlags::EPHEMERAL)
        .await?;

    Ok(())
}

register_module!(PollModule);
