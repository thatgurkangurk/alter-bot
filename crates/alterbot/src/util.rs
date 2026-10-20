use crate::CTX;
use anyhow::Context;
use twilight_model::{
    id::{Id, marker::UserMarker},
    user::CurrentUser,
    util::ImageHash,
};

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

#[must_use]
/// returns a webp url
pub fn get_avatar_url(image_hash: Option<ImageHash>, user_id: Id<UserMarker>) -> String {
    match image_hash {
        Some(hash) => format!(
            "https://cdn.discordapp.com/avatars/{}/{}.webp",
            user_id, hash
        ),
        None => {
            let default_index = (user_id.get() >> 22) % 5;
            format!("https://cdn.discordapp.com/embed/avatars/{default_index}.webp")
        }
    }
}
