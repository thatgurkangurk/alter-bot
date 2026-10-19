use poise::serenity_prelude::{ButtonStyle, CreateActionRow, CreateButton};
use reqwest::Client;
use url::Url;

use crate::bot::{Context, Error};
use awty::{check_packwiz_pack, create_ferinth};

#[poise::command(slash_command, rename = "are-we-there-yet")]
/// a command to get update status for a packwiz modpack (MODRINTH ONLY!)
pub async fn are_we_there_yet(
    ctx: Context<'_>,
    #[description = "url to a packwiz pack.toml file"] url: Url,
    #[description = "minecraft version"] version: String,
    #[description = "should i add a percentage"] add_percentage: Option<bool>,
) -> Result<(), Error> {
    ctx.defer().await?;
    let client = Client::new();
    let ferinth = create_ferinth();
    let include_pct = add_percentage.unwrap_or(false);

    let report = check_packwiz_pack(
        &client,
        &ferinth,
        &url,
        &version,
        None,
        None::<fn(&str, usize, usize)>,
    )
    .await?;
    let embed = super::internal::embed::create_awty_embed(&report, include_pct);

    // encode state into custom_id: prefix|author_id|percentage_flag|version|url
    let custom_id = format!(
        "rf_awty|{}|{}|{}|{}",
        ctx.author().id,
        if include_pct { "1" } else { "0" },
        version,
        url
    );

    let components = vec![CreateActionRow::Buttons(vec![
        CreateButton::new(&custom_id)
            .label("🔄 refresh")
            .style(ButtonStyle::Primary),
    ])];

    ctx.send(
        poise::CreateReply::default()
            .embed(embed)
            .components(components),
    )
    .await?;

    Ok(())
}
