use crate::ShardHandle;
use alterconfig::ConfigManager;
use dashmap::DashMap;
use std::{ops::Deref, sync::OnceLock};
use twilight_cache_inmemory::{InMemoryCache, InMemoryCacheBuilder};
use twilight_http::{Client, client::InteractionClient};
use twilight_model::id::{Id, marker::ApplicationMarker};

pub static CTX: Ref = Ref(OnceLock::new());

#[derive(Debug)]
pub struct Context {
    pub application_id: Id<ApplicationMarker>,
    pub http: Client,
    pub shards: DashMap<u32, ShardHandle>,
    pub cache: InMemoryCache,
    pub config_manager: ConfigManager,
}

impl Context {
    pub fn interaction(&self) -> InteractionClient<'_> {
        self.http.interaction(self.application_id)
    }
}

pub fn init(
    application_id: Id<ApplicationMarker>,
    http: Client,
    shards: u32,
    config_manager: ConfigManager,
) {
    let context = Context {
        application_id,
        http,
        shards: DashMap::with_capacity(shards as usize),
        cache: InMemoryCacheBuilder::new().build(),
        config_manager,
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
