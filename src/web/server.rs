use anyhow::{Context, Result};
use axum::{
    Router,
    routing::{get, post},
};
use poise::serenity_prelude as serenity;
use sea_orm::DatabaseConnection;
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::{error, info};

use super::routes::{polls::create_poll_handler, send_message_handler, status_handler};
use crate::{bot::PollCache, config::ConfigManager};

#[derive(Clone)]
pub struct AppState {
    pub http: Arc<serenity::Http>,
    pub shard_manager: Arc<serenity::ShardManager>,
    pub config_manager: ConfigManager,
    pub poll_cache: PollCache,
    pub db: DatabaseConnection,
}

pub struct WebServerBuilder {
    http: Option<Arc<serenity::Http>>,
    shard_manager: Option<Arc<serenity::ShardManager>>,
    config_manager: Option<ConfigManager>,
    addr: Option<SocketAddr>,
    poll_cache: Option<PollCache>,
    db: Option<DatabaseConnection>,
}

impl WebServerBuilder {
    pub const fn new() -> Self {
        Self {
            http: None,
            shard_manager: None,
            config_manager: None,
            addr: None,
            poll_cache: None,
            db: None,
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

    pub fn config_manager(mut self, config_manager: ConfigManager) -> Self {
        self.config_manager = Some(config_manager);
        self
    }

    pub const fn bind(mut self, addr: SocketAddr) -> Self {
        self.addr = Some(addr);
        self
    }

    pub fn poll_cache(mut self, poll_cache: PollCache) -> Self {
        self.poll_cache = Some(poll_cache);
        self
    }

    pub fn db(mut self, db: DatabaseConnection) -> Self {
        self.db = Some(db);
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
            .config_manager
            .context("config must be provided to WebServerBuilder")?;
        let addr = self
            .addr
            .context("bind address must be provided to WebServerBuilder")?;
        let poll_cache = self
            .poll_cache
            .context("poll cache must be provided to WebServerBuilder")?;
        let db = self.db.context("db must be provided to WebServerBuilder")?;

        Ok(WebServer {
            state: AppState {
                http,
                shard_manager,
                config_manager: config,
                poll_cache,
                db,
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
            .route("/api/polls", post(create_poll_handler))
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
