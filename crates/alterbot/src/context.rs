use crate::{cache::Cache, module::poll::PollManager};
use alterconfig::ConfigManager;
use reqwest::Client as ReqwestClient;
use sea_orm::DatabaseConnection;
use std::{
    ops::Deref,
    sync::{Arc, OnceLock},
};
use twilight_cache_inmemory::ResourceType;
use twilight_http::{Client, client::InteractionClient};
use twilight_model::id::{Id, marker::ApplicationMarker};

pub static CTX: Ref = Ref(OnceLock::new());

#[derive(Debug)]
pub struct Context {
    pub application_id: Id<ApplicationMarker>,
    pub http: Client,
    pub cache: Cache,
    pub config_manager: ConfigManager,
    pub database: DatabaseConnection,
    pub poll_manager: Arc<PollManager>,
    pub http_client: ReqwestClient,
}

impl Context {
    pub fn interaction(&self) -> InteractionClient<'_> {
        self.http.interaction(self.application_id)
    }
}

pub fn init(
    application_id: Id<ApplicationMarker>,
    http: Client,
    config_manager: ConfigManager,
    database: &DatabaseConnection,
    poll_manager: &Arc<PollManager>,
    http_client: ReqwestClient,
) {
    let context = Context {
        http_client,
        application_id,
        http,
        poll_manager: Arc::clone(poll_manager),
        cache: crate::cache::Cache::builder()
            .resource_types(
                ResourceType::GUILD
                    | ResourceType::MEMBER
                    | ResourceType::USER
                    | ResourceType::CHANNEL
                    | ResourceType::ROLE
                    | ResourceType::USER_CURRENT,
            )
            .build(),
        config_manager,
        database: database.clone(),
    };
    assert!(CTX.0.set(context).is_ok());
}

pub struct Ref(OnceLock<Context>);

impl Deref for Ref {
    type Target = Context;

    fn deref(&self) -> &Self::Target {
        self.0.get().unwrap()
    }
}
