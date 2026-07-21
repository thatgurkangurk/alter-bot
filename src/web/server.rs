use anyhow::{Context, Result};
use axum::{
    Router,
    routing::{get, post},
};
use poise::serenity_prelude as serenity;
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::{error, info};

use super::routes::{send_message_handler, status_handler};
use crate::config::Config;

#[derive(Clone)]
pub struct AppState {
    pub http: Arc<serenity::Http>,
    pub shard_manager: Arc<serenity::ShardManager>,
    pub config: Config,
}

pub struct WebServerBuilder {
    http: Option<Arc<serenity::Http>>,
    shard_manager: Option<Arc<serenity::ShardManager>>,
    config: Option<Config>,
    addr: Option<SocketAddr>,
}

impl WebServerBuilder {
    pub const fn new() -> Self {
        Self {
            http: None,
            shard_manager: None,
            config: None,
            addr: None,
        }
    }

    pub fn http(mut self, http: Arc<serenity::Http>) -> Self {
        self.http = Some(http);
        self
    }

    pub fn shard_manager(mut self, shard_manager: Arc<serenity::ShardManager>) -> Self {
        self.shard_manager = Some(shard_manager);
        self
    }

    pub fn config(mut self, config: Config) -> Self {
        self.config = Some(config);
        self
    }

    pub const fn bind(mut self, addr: SocketAddr) -> Self {
        self.addr = Some(addr);
        self
    }

    pub fn build(self) -> Result<WebServer> {
        let http = self
            .http
            .context("http must be provided to WebServerBuilder")?;
        let shard_manager = self
            .shard_manager
            .context("shard_manager must be provided to WebServerBuilder")?;
        let config = self
            .config
            .context("config must be provided to WebServerBuilder")?;
        let addr = self
            .addr
            .context("bind address must be provided to WebServerBuilder")?;

        Ok(WebServer {
            state: AppState {
                http,
                shard_manager,
                config,
            },
            addr,
        })
    }
}

impl Default for WebServerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

pub struct WebServer {
    state: AppState,
    addr: SocketAddr,
}

impl WebServer {
    pub const fn builder() -> WebServerBuilder {
        WebServerBuilder::new()
    }

    /// spawns the axum web server in a background task
    pub fn run(self) {
        let addr = self.addr;
        let app = Router::new()
            .route("/status", get(status_handler))
            .route("/api/send-message", post(send_message_handler))
            .with_state(self.state);

        tokio::spawn(async move {
            info!("web server listening on http://{addr}");

            let listener = match tokio::net::TcpListener::bind(addr).await {
                Ok(l) => l,
                Err(e) => {
                    error!("failed to bind axum server to {addr}: {e}");
                    return;
                }
            };

            if let Err(err) = axum::serve(listener, app).await {
                error!("axum server error: {err}");
            }
        });
    }
}
