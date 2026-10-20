use crate::CTX;
use anyhow::Context;
use twilight_model::user::CurrentUser;

#[must_use]
pub fn sanitise_pings(message: &str) -> String {
    message.replace('@', "@\u{200B}")
}

#[must_use]
pub fn rgb_to_packed_u32(r: u8, g: u8, b: u8) -> u32 {
    ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
}

pub async fn get_bot_user() -> anyhow::Result<CurrentUser> {
    if let Some(user) = CTX.cache.current_user() {
        return Ok(user);
    }

    let user = CTX
        .http
        .current_user()
        .await
        .context("fetching current user")?
        .model()
        .await?;

    Ok(user)
}
