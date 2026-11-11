use anyhow::Context;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use twilight_interactions::command::{CommandModel, CreateCommand};
use twilight_model::{
    application::{
        command::Command,
        interaction::{Interaction, application_command::CommandData},
    },
    channel::message::Embed,
};
use twilight_util::builder::{
    embed::{EmbedBuilder, ImageSource},
    interaction_response::ChannelMessageBuilder,
};

use crate::{
    CTX,
    module::{CommandScope, ModuleCommand},
};

#[derive(CreateCommand, CommandModel)]
#[command(name = "xkcd", desc = "get xkcd comics")]
pub enum XkcdCommand {
    #[command(name = "latest")]
    Latest(XkcdLatestCommand),

    #[command(name = "number")]
    Number(XkcdNumberCommand),

    #[command(name = "random")]
    Random(XkcdRandomCommand),
}

pub fn command() -> Command {
    XkcdCommand::create_command().into()
}

pub const XKCD_COMMAND: ModuleCommand = ModuleCommand {
    name: "xkcd",
    scope: CommandScope::Global,
    builder: command,
    run: |i, d| Box::pin(XkcdCommand::handle(i.0, *d)),
    autocomplete: None,
};

impl XkcdCommand {
    pub async fn handle(interaction: Interaction, data: CommandData) -> anyhow::Result<()> {
        let command =
            XkcdCommand::from_interaction(data.into()).context("could not parse command data")?;

        match command {
            XkcdCommand::Latest(command) => command.run(interaction).await,
            XkcdCommand::Number(command) => command.run(interaction).await,
            XkcdCommand::Random(command) => command.run(interaction).await,
        }
    }
}

#[derive(CommandModel, CreateCommand, Debug)]
#[command(name = "latest", desc = "show the latest xkcd comic")]
pub struct XkcdLatestCommand;

impl XkcdLatestCommand {
    pub async fn run(&self, interaction: Interaction) -> anyhow::Result<()> {
        let comic = XkcdEntry::get_latest(crate::context::http_client()).await?;
        let embed = crate_comic_embed(comic)?;

        let response = ChannelMessageBuilder::new().embeds([embed]).build();

        CTX.interaction()
            .create_response(interaction.id, &interaction.token, &response)
            .await?;

        Ok(())
    }
}

#[derive(CommandModel, CreateCommand, Debug)]
#[command(name = "random", desc = "get a random xkcd comic")]
pub struct XkcdRandomCommand;

impl XkcdRandomCommand {
    pub async fn run(&self, interaction: Interaction) -> anyhow::Result<()> {
        let comic = XkcdEntry::get_random(crate::context::http_client()).await?;
        let embed = crate_comic_embed(comic)?;

        let response = ChannelMessageBuilder::new().embeds([embed]).build();

        CTX.interaction()
            .create_response(interaction.id, &interaction.token, &response)
            .await?;

        Ok(())
    }
}

#[derive(CommandModel, CreateCommand, Debug)]
#[command(name = "number", desc = "get a specific xkcd comic")]
pub struct XkcdNumberCommand {
    #[command(min_value = 1, desc = "comic number")]
    pub number: i64,
}

impl XkcdNumberCommand {
    pub async fn run(&self, interaction: Interaction) -> anyhow::Result<()> {
        let comic =
            XkcdEntry::get_number(crate::context::http_client(), self.number.try_into()?).await?;

        let response = match comic {
            Some(comic) => ChannelMessageBuilder::new().embeds([crate_comic_embed(comic)?]),
            None => ChannelMessageBuilder::new()
                .content(format!("no comic was found for number {}", self.number)),
        }
        .build();

        CTX.interaction()
            .create_response(interaction.id, &interaction.token, &response)
            .await?;

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XkcdEntry {
    #[serde(rename = "num")]
    pub number: u32,
    pub title: String,
    pub year: String,
    pub month: String,
    pub day: String,
    pub alt: String,
    #[serde(rename = "img")]
    pub image_url: String,
    #[serde(default)]
    pub news: String,
    #[serde(default)]
    pub transcript: String,
}

impl XkcdEntry {
    pub async fn get_number(client: &Client, number: u32) -> anyhow::Result<Option<Self>> {
        let url = format!("https://xkcd.com/{number}/info.0.json");
        let response = client.get(&url).send().await?;

        if response.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }

        let comic = response.error_for_status()?.json::<Self>().await?;
        Ok(Some(comic))
    }

    pub async fn get_latest(client: &Client) -> anyhow::Result<Self> {
        let response = client
            .get("https://xkcd.com/info.0.json")
            .send()
            .await?
            .error_for_status()?;

        let comic = response.json::<Self>().await?;
        Ok(comic)
    }

    pub async fn get_random(client: &Client) -> anyhow::Result<Self> {
        let latest = Self::get_latest(client).await?;

        for _ in 0..5 {
            let number = fastrand::u32(1..=latest.number);
            if let Some(comic) = Self::get_number(client, number).await? {
                return Ok(comic);
            }
        }

        Err(anyhow::anyhow!(
            "failed to get a random comic after 5 retries"
        ))
    }

    pub fn url(&self) -> String {
        format!("https://xkcd.com/{}/", self.number)
    }
}

fn crate_comic_embed(comic: XkcdEntry) -> anyhow::Result<Embed> {
    let image = ImageSource::url(&comic.image_url)?;
    let title = format!(
        "{}: {} ({}-{}-{})",
        comic.number, comic.title, comic.year, comic.month, comic.day
    );

    let embed = EmbedBuilder::new()
        .color(0x2f3136)
        .title(title)
        .url(comic.url())
        .image(image)
        .build();

    Ok(embed)
}
