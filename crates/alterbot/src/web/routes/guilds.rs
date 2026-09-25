use crate::web::{AppResult, AppState};
use axum::{
    Json,
    extract::{Path, State},
};
use serde::Serialize;
use serenity::model::channel::GuildChannel;
use serenity::model::guild::GuildInfo;
use serenity::model::id::GuildId;

#[derive(Debug, Serialize)]
pub struct GuildResponse {
    pub id: String,
    pub name: String,
    pub icon: Option<String>,
}

impl From<GuildInfo> for GuildResponse {
    fn from(guild: GuildInfo) -> Self {
        Self {
            id: guild.id.to_string(),
            name: guild.name,
            icon: guild.icon.map(|i| i.to_string()),
        }
    }
}

pub async fn list_guilds(State(state): State<AppState>) -> AppResult<Json<Vec<GuildResponse>>> {
    let guilds = state.http.get_guilds(None, None).await?;

    let response = guilds.into_iter().map(GuildResponse::from).collect();

    Ok(Json(response))
}

#[derive(Debug, Serialize)]
pub struct ChannelResponse {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub position: u16,
    pub parent_id: Option<String>,
}

impl From<GuildChannel> for ChannelResponse {
    fn from(channel: GuildChannel) -> Self {
        Self {
            id: channel.id.to_string(),
            name: channel.name,
            kind: format!("{:?}", channel.kind),
            position: channel.position,
            parent_id: channel.parent_id.map(|id| id.to_string()),
        }
    }
}

pub async fn list_guild_channels(
    State(state): State<AppState>,
    Path(guild_id): Path<GuildId>,
) -> AppResult<Json<Vec<ChannelResponse>>> {
    let channels = state.http.get_channels(guild_id).await?;

    let response = channels.into_iter().map(ChannelResponse::from).collect();

    Ok(Json(response))
}
