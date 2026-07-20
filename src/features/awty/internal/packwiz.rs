use anyhow::{Context, Result};
use futures::StreamExt;
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashSet;
use poise::serenity_prelude::{CreateEmbed, Colour, Timestamp};
use url::Url;

use crate::bot::Error;

#[derive(Deserialize, Debug, Clone)]
pub struct PackToml {
    pub index: PackIndex,
}

#[derive(Deserialize, Debug, Clone)]
pub struct PackIndex {
    pub file: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct IndexToml {
    pub files: Vec<IndexFile>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct IndexFile {
    pub file: String,
    #[serde(default)]
    pub metafile: bool,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ModToml {
    pub update: Option<ModUpdate>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ModUpdate {
    pub modrinth: Option<ModrinthUpdate>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ModrinthUpdate {
    #[serde(rename = "mod-id")]
    pub mod_id: String,
}

/// fetches and parses the pack.toml
pub async fn fetch_pack(client: &Client, url: &Url) -> Result<PackToml> {
    let text = client.get(url.clone()).send().await?.text().await?;
    toml::from_str(&text).context("Failed to parse pack.toml")
}

/// fetches and parses the index.toml using the base pack URL and the index file path
pub async fn fetch_index(
    client: &Client,
    base_url: &Url,
    index_path: &str,
) -> Result<(Url, IndexToml)> {
    let index_url = base_url
        .join(index_path)
        .context("Failed to resolve index URL")?;
    let text = client.get(index_url.clone()).send().await?.text().await?;
    let index = toml::from_str(&text).context("Failed to parse index.toml")?;

    Ok((index_url, index))
}

/// extracts all valid urls for files marked as `metafile = true` in the index
pub fn get_metafile_urls(index_url: &Url, index: &IndexToml) -> Vec<Url> {
    index
        .files
        .iter()
        .filter(|f| f.metafile)
        .filter_map(|f| index_url.join(&f.file).ok())
        .collect()
}

/// fetches a mod.toml metafile and attempts to extract the modrinth mod id.
/// returns `Ok(None)` if the file is valid but doesn't contain a modrinth mod id (github/file url/curseforge).
pub async fn fetch_modrinth_id(client: &Client, url: Url) -> Result<Option<String>> {
    let response = client.get(url).send().await?.text().await?;
    let mod_toml: ModToml = toml::from_str(&response).context("Failed to parse mod metafile")?;

    let mod_id = mod_toml.update.and_then(|u| u.modrinth).map(|m| m.mod_id);

    Ok(mod_id)
}

pub async fn check_packwiz_status(
    client: &Client,
    url: &Url,
    version: &str,
    add_percentage: bool,
) -> Result<CreateEmbed, Error> {
    let pack = super::packwiz::fetch_pack(client, url).await?;
    let (index_url, index) = super::packwiz::fetch_index(client, url, &pack.index.file).await?;
    let metafile_urls = super::packwiz::get_metafile_urls(&index_url, &index);

    let mut stream = futures::stream::iter(metafile_urls)
        .map(|u| {
            let client_clone = client.clone();
            let filename = u.path_segments().and_then(|mut s| s.next_back()).unwrap_or("unknown.toml").to_string();
            tokio::spawn(async move {
                let res = fetch_modrinth_id(&client_clone, u).await;
                (filename, res)
            })
        })
        .buffer_unordered(4);

    let mut modrinth_ids = HashSet::new();
    while let Some(result) = stream.next().await {
        if let Ok((_, fetch_result)) = result
            && let Ok(Some(id)) = fetch_result {
                modrinth_ids.insert(id);
            }
    }

    let ferinth = super::create_ferinth();
    let results = super::version::are_on_version(&ferinth, modrinth_ids.into_iter().collect(), version).await?;
    let formatted_output = super::format_mod_statuses(&results, add_percentage);

    Ok(CreateEmbed::new()
        .title(format!("update status for {version}"))
        .description(format!("```text\n{formatted_output}\n```"))
        .colour(Colour::BLURPLE)
        .timestamp(Timestamp::now()))
}