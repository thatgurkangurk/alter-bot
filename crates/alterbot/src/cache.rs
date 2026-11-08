use twilight_cache_inmemory::{CacheableModels, InMemoryCache, model};
use twilight_model::{
    channel::{Channel, StageInstance},
    guild::{GuildIntegration, Role, scheduled_event::GuildScheduledEvent},
    user::{CurrentUser, User},
};

pub type Cache = InMemoryCache<Models>;

#[derive(Clone, Debug)]
pub struct Models;

impl CacheableModels for Models {
    type Channel = Channel;
    type CurrentUser = CurrentUser;
    type Emoji = model::CachedEmoji;
    type Guild = model::CachedGuild;
    type GuildIntegration = GuildIntegration;
    type Member = model::CachedMember;
    type Message = model::CachedMessage;
    type Presence = model::CachedPresence;
    type Role = Role;
    type StageInstance = StageInstance;
    type Sticker = model::CachedSticker;
    type User = User;
    type VoiceState = model::CachedVoiceState;
    type GuildScheduledEvent = GuildScheduledEvent;
}
