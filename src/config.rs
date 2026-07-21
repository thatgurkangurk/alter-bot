use anyhow::{Context, Result};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

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
pub struct WebConfig {
    pub tokens: Vec<String>,
    pub port: Option<u16>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub uri: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub bot: BotConfig,
    pub db: DatabaseConfig,
    pub web: WebConfig,
}

#[derive(Clone)]
pub struct ConfigManager {
    inner: Arc<RwLock<Config>>,
    path: Option<PathBuf>,
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

impl ConfigManager {
    pub fn new(path: Option<&Path>) -> Result<Self> {
        let config = Config::load(path)?;

        Ok(Self {
            inner: Arc::new(RwLock::new(config)),
            path: path.map(Path::to_path_buf),
        })
    }

    pub async fn get(&self) -> Config {
        self.inner.read().await.clone()
    }

    pub async fn reload(&self) -> Result<()> {
        let new_config = Config::load(self.path.as_deref())?;

        {
            let mut writer = self.inner.write().await;
            *writer = new_config;
        }

        Ok(())
    }

    pub fn watch(&self) -> anyhow::Result<()> {
        let Some(path) = &self.path else {
            warn!("no file path associated with the config manager. watcher skipped.");
            return Ok(());
        };

        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

        let mut watcher = RecommendedWatcher::new(
            move |res: Result<Event, notify::Error>| {
                if let Ok(event) = res {
                    let _ = tx.send(event);
                }
            },
            notify::Config::default(),
        )?;

        watcher.watch(path, RecursiveMode::NonRecursive)?;

        let manager = self.clone();
        tokio::spawn(async move {
            let _watcher = watcher;

            while let Some(event) = rx.recv().await {
                if matches!(event.kind, EventKind::Modify(_) | EventKind::Create(_)) {
                    info!("config file update detected. reloading...");

                    if let Err(err) = manager.reload().await {
                        error!("auto-reload failed: {:#}", err);
                    } else {
                        info!("config reloaded successfully.");
                    }
                }
            }
        });

        Ok(())
    }
}
