use crate::bot::{Data, Error};
use poise::serenity_prelude as serenity;

use super::dad::dad_joke;

pub async fn event_handler(
    ctx: &serenity::Context,
    event: &serenity::FullEvent,
    _framework: poise::FrameworkContext<'_, Data, Error>,
    data: &Data,
) -> Result<(), Error> {
    if let serenity::FullEvent::Message { new_message } = event {
        dad_joke(ctx, new_message, data).await?;
    }
    Ok(())
}
