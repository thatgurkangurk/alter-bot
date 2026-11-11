use crate::{cache::Cache, module::poll::PollManager};
use alterconfig::ConfigManager;
use reqwest::Client as ReqwestClient;
use sea_orm::DatabaseConnection;
use std::sync::{Arc, OnceLock};
use twilight_cache_inmemory::ResourceType;
use twilight_http::{Client, client::InteractionClient};
use twilight_model::id::{Id, marker::ApplicationMarker};

pub static CTX: Context = Context::new();

pub struct Context {
    pub application_id: OnceLock<Id<ApplicationMarker>>,
    pub http: OnceLock<Client>,
    pub cache: OnceLock<Cache>,
    pub config_manager: OnceLock<ConfigManager>,
    pub database: OnceLock<DatabaseConnection>,
    pub poll_manager: OnceLock<Arc<PollManager>>,
    pub http_client: OnceLock<ReqwestClient>,
}

impl Context {
    pub const fn new() -> Self {
        Self {
            application_id: OnceLock::new(),
            http: OnceLock::new(),
            cache: OnceLock::new(),
            config_manager: OnceLock::new(),
            database: OnceLock::new(),
            poll_manager: OnceLock::new(),
            http_client: OnceLock::new(),
        }
    }

    pub fn interaction(&self) -> InteractionClient<'_> {
        let app_id = *self.application_id.get().expect("application id not set");
        let http = self.http.get().expect("http client not set");
        http.interaction(app_id)
    }
}

pub fn config() -> alterconfig::Config {
    CTX.config_manager
        .get()
        .expect("config not initialised")
        .get_sync()
}

pub fn http() -> &'static Client {
    CTX.http.get().expect("http client not initialised")
}

pub fn db() -> &'static DatabaseConnection {
    CTX.database.get().expect("database not initialised")
}

pub fn set_db(db: DatabaseConnection) {
    assert!(CTX.database.set(db).is_ok(), "database already initialised");
}

pub fn http_client() -> &'static ReqwestClient {
    CTX.http_client
        .get()
        .expect("reqwest http client not initialised")
}

pub fn poll_manager() -> &'static Arc<PollManager> {
    CTX.poll_manager
        .get()
        .expect("poll manager not initialised")
}

pub fn set_poll_manager(pm: Arc<PollManager>) {
    assert!(
        CTX.poll_manager.set(pm).is_ok(),
        "poll manager already initialised"
    );
}

pub fn cache() -> &'static Cache {
    CTX.cache.get().expect("cache not initialised")
}

pub fn init_early(
    application_id: Id<ApplicationMarker>,
    http: Client,
    config_manager: ConfigManager,
    http_client: ReqwestClient,
) {
    let cache = Cache::builder()
        .resource_types(
            ResourceType::GUILD
                | ResourceType::MEMBER
                | ResourceType::USER
                | ResourceType::CHANNEL
                | ResourceType::ROLE
                | ResourceType::USER_CURRENT,
        )
        .build();

    assert!(CTX.application_id.set(application_id).is_ok());
    assert!(CTX.http.set(http).is_ok());
    assert!(CTX.config_manager.set(config_manager).is_ok());
    assert!(CTX.http_client.set(http_client).is_ok());
    assert!(CTX.cache.set(cache).is_ok());
}
