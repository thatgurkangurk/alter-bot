use poise::serenity_prelude::{CreateActionRow, CreateButton, ButtonStyle};
use reqwest::Client;
use url::Url;

use crate::{
    awty,
    bot::{Context, Error},
};

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
    let include_pct = add_percentage.unwrap_or(false);

    let embed = awty::packwiz::check_packwiz_status(&client, &url, &version, include_pct).await?;

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
            .components(components)
    ).await?;

    Ok(())
}