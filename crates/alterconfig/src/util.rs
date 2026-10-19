use std::{fmt, str};

/// taken from https://github.com/serenity-rs/serenity/blob/current/src/utils/token.rs
pub fn validate_token(token: Option<&str>) -> Result<(), InvalidToken> {
    let Some(token) = token else {
        return Err(InvalidToken);
    };

    let mut parts = token.trim_start_matches("Bot ").split('.');

    let is_valid = parts.next().is_some_and(|p| !p.is_empty())
        && parts.next().is_some_and(|p| !p.is_empty())
        && parts.next().is_some_and(|p| !p.is_empty())
        && parts.next().is_none();

    if is_valid { Ok(()) } else { Err(InvalidToken) }
}

#[derive(Debug)]
pub struct InvalidToken;

impl std::error::Error for InvalidToken {}

impl fmt::Display for InvalidToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("The provided token was invalid")
    }
}
