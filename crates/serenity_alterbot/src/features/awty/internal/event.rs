use awty::{check_packwiz_pack, create_ferinth};
use poise::serenity_prelude::{
    self as serenity, CreateInteractionResponse, CreateInteractionResponseMessage,
    EditInteractionResponse,
};
use reqwest::Client;
use url::Url;

use crate::bot::Error;

pub async fn handle_persistent_buttons(
    ctx: &serenity::Context,
    event: &serenity::FullEvent,
) -> Result<(), Error> {
    if let serenity::FullEvent::InteractionCreate { interaction } = event
        && let Some(press) = interaction.as_message_component()
        && press.data.custom_id.starts_with("rf_awty|")
    {
        let parts: Vec<&str> = press.data.custom_id.split('|').collect();
        if parts.len() < 5 {
            return Ok(());
        }

        let target_author_id = parts[1];
        let add_percentage = parts[2] == "1";
        let version = parts[3];
        let url_str = parts[4];

        if press.user.id.to_string() != target_author_id {
            press
                .create_response(
                    ctx,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new()
                            .content(
                                "only the person who originally requested this list can refresh it",
                            )
                            .ephemeral(true),
                    ),
                )
                .await?;
            return Ok(());
        }

        press.defer(ctx).await?;

        if let Ok(url) = Url::parse(url_str) {
            let client = Client::new();
            let ferinth = create_ferinth();

            if let Ok(report) = check_packwiz_pack(
                &client,
                &ferinth,
                &url,
                version,
                None,
                None::<fn(&str, usize, usize)>,
            )
            .await
            {
                let updated_embed = super::embed::create_awty_embed(&report, add_percentage);

                press
                    .edit_response(ctx, EditInteractionResponse::new().embed(updated_embed))
                    .await?;
            }
        }
    }
    Ok(())
}
