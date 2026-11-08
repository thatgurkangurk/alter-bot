use crate::cache::Cache;
use alterconfig::ConfigManager;
use sea_orm::DatabaseConnection;
use std::{ops::Deref, sync::OnceLock};
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
    database: DatabaseConnection,
) {
    let context = Context {
        application_id,
        http,
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
        database,
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
