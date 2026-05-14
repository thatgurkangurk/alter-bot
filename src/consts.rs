use poise::serenity_prelude as serenity;

pub const VERSION: &str = match option_env!("APP_VERSION") {
    Some(v) => v,
    None => "local-dev",
};

pub const REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");

/// split this on ':'
pub const AUTHORS_RAW: &str = env!("CARGO_PKG_AUTHORS");

pub const DATA_DIR: &str = match option_env!("DATA_DIR") {
    Some(dir) => dir,
    None => "./data",
};

#[allow(clippy::unreadable_literal)]
pub const ELIGIBLE_TO_VOTE_ROLE_ID: serenity::RoleId = serenity::RoleId::new(1504436168257831034);
