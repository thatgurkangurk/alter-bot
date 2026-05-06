use std::path::Path;

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::util::validate_token;

#[derive(Debug, Clone, Deserialize)]
#[serde(try_from = "String")]
pub struct DiscordToken(String);

impl TryFrom<String> for DiscordToken {
    type Error = String;

    fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
        if let Err(e) = validate_token(Some(&value)) {
            return Err(format!("token error: {e}"));
        }
        Ok(Self(value))
    }
}

impl AsRef<str> for DiscordToken {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct BotConfig {
    pub token: DiscordToken,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub bot: BotConfig,
}

impl Config {
    pub fn load(path: Option<&Path>) -> Result<Self> {
        let file_source = path
            .map_or_else(|| config::File::with_name("config"), config::File::from)
            .required(false);

        config::Config::builder()
            .add_source(file_source)
            .add_source(config::Environment::with_prefix("ALTER_BOT").separator("__"))
            .build()
            .context("failed to load the config")?
            .try_deserialize()
            .context("failed to deserialize the config")
    }
}
