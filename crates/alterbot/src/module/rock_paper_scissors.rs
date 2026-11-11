use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::Result;
use dashmap::DashMap;
use tracing::{Instrument, info_span};

use twilight_interactions::command::{CommandModel, CreateCommand};
use twilight_model::{
    application::{
        command::Command,
        interaction::{InteractionData, application_command::CommandData},
    },
    channel::message::{
        EmojiReactionType, MessageFlags,
        component::{ActionRow, Button, ButtonStyle, Component},
    },
    gateway::event::Event,
    gateway::payload::incoming::InteractionCreate,
    http::interaction::{InteractionResponse, InteractionResponseData, InteractionResponseType},
    id::{
        Id,
        marker::{MessageMarker, UserMarker},
    },
    util::Timestamp,
};

use crate::{
    context::{self, CTX},
    module::{CommandScope, Module, ModuleCommand},
    register_module,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RpsChoice {
    Rock,
    Paper,
    Scissors,
}

impl RpsChoice {
    pub const ALL: [Self; 3] = [Self::Rock, Self::Paper, Self::Scissors];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Rock => "Rock",
            Self::Paper => "Paper",
            Self::Scissors => "Scissors",
        }
    }

    pub fn emoji_name(self) -> &'static str {
        match self {
            Self::Paper => "📄",
            Self::Rock => "🪨",
            Self::Scissors => "✂️",
        }
    }

    pub fn beats(self, other: Self) -> bool {
        matches!(
            (self, other),
            (Self::Rock, Self::Scissors)
                | (Self::Scissors, Self::Paper)
                | (Self::Paper, Self::Rock)
        )
    }

    pub fn random() -> Self {
        let mut rng = fastrand::Rng::new();
        Self::ALL[rng.usize(..Self::ALL.len())]
    }

    pub fn parse_custom_id(custom_id: &str) -> Option<(Id<UserMarker>, Id<UserMarker>, Self)> {
        let mut parts = custom_id.split(':');
        if parts.next()? != "rps" {
            return None;
        }

        let challenger_id = parts.next()?.parse::<u64>().ok()?;
        let opponent_id = parts.next()?.parse::<u64>().ok()?;
        let choice = match parts.next()? {
            "Rock" => Self::Rock,
            "Paper" => Self::Paper,
            "Scissors" => Self::Scissors,
            _ => return None,
        };

        Some((Id::new(challenger_id), Id::new(opponent_id), choice))
    }
}

#[derive(Default)]
struct GameSession {
    user1_choice: Option<RpsChoice>,
    user2_choice: Option<RpsChoice>,
}

static GAME_SESSIONS: std::sync::LazyLock<DashMap<Id<MessageMarker>, GameSession>> =
    std::sync::LazyLock::new(DashMap::new);

pub struct RpsGameLogic;

pub struct GameResult {
    pub message: String,
    pub losers: Vec<Id<UserMarker>>,
}

impl RpsGameLogic {
    pub fn evaluate(
        challenger_id: Id<UserMarker>,
        opponent_id: Id<UserMarker>,
        c1: RpsChoice,
        c2: RpsChoice,
    ) -> GameResult {
        let is_self_play = challenger_id == opponent_id;
        let mut losers = Vec::new();

        let outcome = if c1 == c2 {
            losers.push(challenger_id);
            if !is_self_play {
                losers.push(opponent_id);
            }
            if is_self_play {
                "you played yourself and tied !".to_string()
            } else {
                "it's a tie !".to_string()
            }
        } else if c1.beats(c2) {
            if !is_self_play {
                losers.push(opponent_id);
            }
            if is_self_play {
                "you played yourself and won !".to_string()
            } else {
                format!("<@{challenger_id}> wins !")
            }
        } else {
            if !is_self_play {
                losers.push(challenger_id);
            }
            if is_self_play {
                "you played yourself and lost !".to_string()
            } else {
                format!("<@{opponent_id}> wins!")
            }
        };

        let message = format!(
            "<@{challenger_id}> chose {}.\n<@{opponent_id}> chose {}.\n\n{}",
            c1.as_str(),
            c2.as_str(),
            outcome
        );

        GameResult { message, losers }
    }
}

pub struct RpsModule;

#[derive(CreateCommand, CommandModel)]
#[command(
    name = "rock-paper-scissors",
    desc = "play a game of rock paper scissors and timeout the loser !"
)]
pub struct RockPaperScissorsCommand {
    #[command(desc = "the user to play against.")]
    pub against: Id<UserMarker>,
}

pub fn command() -> Command {
    RockPaperScissorsCommand::create_command().into()
}

pub async fn run(interaction: Box<InteractionCreate>, data: Box<CommandData>) -> Result<()> {
    let span = info_span!("rps_command_run", interaction_id = %interaction.id);
    async move {
        tracing::info!("handling /rock-paper-scissors command execution");
        let cmd = RockPaperScissorsCommand::from_interaction((*data).into())?;

        let challenger = match interaction.author_id() {
            Some(id) => id,
            None => {
                return send_ephemeral_response(
                    &interaction,
                    "could not determine interaction author.",
                )
                .await;
            }
        };

        let opponent = cmd.against;

        let components = vec![Component::ActionRow(ActionRow {
            id: None,
            components: RpsChoice::ALL
                .iter()
                .map(|choice| {
                    Component::Button(Button {
                        id: None,
                        custom_id: Some(format!(
                            "rps:{challenger}:{opponent}:{choice_str}",
                            choice_str = choice.as_str()
                        )),
                        disabled: false,
                        emoji: Some(EmojiReactionType::Unicode {
                            name: choice.emoji_name().to_string(),
                        }),
                        label: Some(choice.as_str().to_string()),
                        style: ButtonStyle::Primary,
                        url: None,
                        sku_id: None,
                    })
                })
                .collect(),
        })];

        let content =
            format!("<@{challenger}> has challenged <@{opponent}> to Rock Paper Scissors!");

        let response = InteractionResponse {
            kind: InteractionResponseType::ChannelMessageWithSource,
            data: Some(InteractionResponseData {
                content: Some(content),
                components: Some(components),
                ..Default::default()
            }),
        };

        CTX.interaction()
            .create_response(interaction.id, &interaction.token, &response)
            .await?;

        Ok(())
    }
    .instrument(span)
    .await
}

#[async_trait::async_trait]
impl Module for RpsModule {
    fn name(&self) -> &'static str {
        "rps"
    }

    fn commands(&self) -> Vec<ModuleCommand> {
        vec![ModuleCommand {
            name: "rock-paper-scissors",
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

        let Some(InteractionData::MessageComponent(data)) = &interaction.data else {
            return Ok(());
        };

        let Some((challenger_id, opponent_id, choice)) =
            RpsChoice::parse_custom_id(&data.custom_id)
        else {
            return Ok(());
        };

        let span = info_span!(
            "rps_button_event",
            interaction_id = %interaction.id,
            custom_id = %data.custom_id
        );

        async move {
            let Some(clicker_id) = interaction.author_id() else {
                return Ok(());
            };

            let Some(message) = &interaction.message else {
                return Ok(());
            };

            // ignore people who aren't in the game
            if clicker_id != challenger_id && clicker_id != opponent_id {
                send_ephemeral_response(interaction, "You aren't a part of this battle!").await?;
                return Ok(());
            }

            let is_self_play = challenger_id == opponent_id;
            let is_against_bot = interaction.author().is_some_and(|u| u.bot);

            let (user1_choice, user2_choice) = {
                let mut session = GAME_SESSIONS.entry(message.id).or_default();

                if clicker_id == challenger_id {
                    session.user1_choice = Some(choice);
                } else if clicker_id == opponent_id {
                    session.user2_choice = Some(choice);
                }

                // handle bot or self-play logic
                if is_self_play || is_against_bot {
                    session.user2_choice = Some(RpsChoice::random());
                }

                (session.user1_choice, session.user2_choice)
            };

            // if both have chosen, finish the game
            let (c1, c2) = match (user1_choice, user2_choice) {
                (Some(c1), Some(c2)) => {
                    GAME_SESSIONS.remove(&message.id);
                    (c1, c2)
                }
                _ => {
                    // acknowledge the click so it doesnt say that the interaction failed
                    let response = InteractionResponse {
                        kind: InteractionResponseType::ChannelMessageWithSource,
                        data: Some(InteractionResponseData {
                            content: Some(format!(
                                "your choice (**{}**) has been locked in!",
                                choice.as_str()
                            )),
                            flags: Some(MessageFlags::EPHEMERAL),
                            ..Default::default()
                        }),
                    };

                    CTX.interaction()
                        .create_response(interaction.id, &interaction.token, &response)
                        .await?;
                    return Ok(());
                }
            };

            let result = RpsGameLogic::evaluate(challenger_id, opponent_id, c1, c2);

            // timeout logic
            if let Some(guild_id) = interaction.guild_id {
                let bot_id = interaction.application_id;

                let timeout_until = SystemTime::now()
                    .checked_add(Duration::from_secs(60))
                    .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                    .map(|d| d.as_secs().cast_signed());

                if let Some(timestamp_secs) = timeout_until
                    && let Ok(timestamp) = Timestamp::from_secs(timestamp_secs)
                {
                    for loser_id in result.losers {
                        // don't try to timeout the bot itself (cuz that's silly)
                        if loser_id.get() == bot_id.get() {
                            continue;
                        }

                        let _ = context::http()
                            .update_guild_member(guild_id, loser_id)
                            .communication_disabled_until(Some(timestamp))
                            .await;
                    }
                }
            }

            // remove buttons from the original message immediately
            let response = InteractionResponse {
                kind: InteractionResponseType::UpdateMessage,
                data: Some(InteractionResponseData {
                    content: Some(result.message),
                    components: Some(vec![]),
                    ..Default::default()
                }),
            };

            CTX.interaction()
                .create_response(interaction.id, &interaction.token, &response)
                .await?;

            Ok(())
        }
        .instrument(span)
        .await
    }
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

register_module!(RpsModule);
