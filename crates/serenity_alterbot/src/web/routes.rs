use crate::web::{AppResult, AppState};
use ::serenity::model::id::{ChannelId, MessageId};
use axum::{Json, extract::State};
use poise::serenity_prelude as serenity;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

pub(super) mod guilds;
pub(super) mod polls;

#[derive(Debug, Serialize, ToSchema)]
pub struct StatusResponse {
    pub status: &'static str,
    pub active_shards: usize,
    pub version: &'static str,
}

#[utoipa::path(
    get,
    path = "/status",
    responses(
        (status = 200, description = "Bot operational status", body = StatusResponse)
    )
)]
pub async fn status_handler(State(state): State<AppState>) -> AppResult<Json<StatusResponse>> {
    let active_shards = {
        let runners = state.shard_manager.runners.lock().await;
        runners.len()
    };

    if active_shards == 0 {
        return Ok(Json(StatusResponse {
            status: "disconnected",
            active_shards: 0,
            version: crate::consts::VERSION,
        }));
    }

    Ok(Json(StatusResponse {
        status: "online",
        active_shards,
        version: crate::consts::VERSION,
    }))
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct MessageRequest {
    pub channel_id: String,
    pub message: String,
    pub reply_to_id: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MessageResponse {
    pub success: bool,
    pub message: String,
}

#[utoipa::path(
    post,
    path = "/api/messages",
    request_body = MessageRequest,
    responses(
        (status = 200, description = "message sent successfully", body = MessageResponse),
        (status = 400, description = "invalid channel or reply id format"),
        (status = 500, description = "discord api error")
    )
)]
pub async fn send_message_handler(
    State(state): State<AppState>,
    Json(body): Json<MessageRequest>,
) -> AppResult<Json<MessageResponse>> {
    let channel_id = body
        .channel_id
        .parse::<u64>()
        .map(ChannelId::new)
        .map_err(|_| crate::web::error::AppError::NotFound("Invalid channel_id format".into()))?;

    let http = &state.http;

    if let Some(reply_id_str) = body.reply_to_id {
        let reply_id = reply_id_str
            .parse::<u64>()
            .map(MessageId::new)
            .map_err(|_| {
                crate::web::error::AppError::NotFound("Invalid reply_to_id format".into())
            })?;

        channel_id
            .send_message(
                http,
                serenity::builder::CreateMessage::new()
                    .content(body.message)
                    .reference_message((channel_id, reply_id)),
            )
            .await?;
    } else {
        channel_id.say(http, body.message).await?;
    }

    Ok(Json(MessageResponse {
        success: true,
        message: "Message dispatched successfully".to_string(),
    }))
}
