use ::serenity::model::id::{ChannelId, MessageId};
use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use poise::serenity_prelude as serenity;
use serde::{Deserialize, Serialize};

use super::AppState;

pub(super) mod polls;

#[derive(Serialize)]
pub struct StatusResponse {
    pub status: &'static str,
    pub active_shards: usize,
    pub version: &'static str,
}

pub async fn status_handler(
    State(state): State<AppState>,
) -> Result<Json<StatusResponse>, StatusCode> {
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

#[derive(Deserialize)]
pub struct MessageRequest {
    pub channel_id: String,
    pub message: String,
    pub reply_to_id: Option<String>,
}

pub async fn send_message_handler(
    State(state): State<AppState>,
    Json(body): Json<MessageRequest>,
) -> impl IntoResponse {
    let channel_id: ChannelId = match body.channel_id.parse::<u64>() {
        Ok(id) => ChannelId::new(id),
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                "Invalid channel_id format".to_string(),
            );
        }
    };

    let http = &state.http;

    let result = if let Some(reply_id_str) = body.reply_to_id {
        if let Ok(reply_id) = reply_id_str.parse::<u64>() {
            channel_id
                .send_message(
                    http,
                    serenity::builder::CreateMessage::new()
                        .content(body.message)
                        .reference_message((channel_id, MessageId::new(reply_id))),
                )
                .await
        } else {
            return (
                StatusCode::BAD_REQUEST,
                "Invalid reply_to_id format".to_string(),
            );
        }
    } else {
        channel_id.say(http, body.message).await
    };

    match result {
        Ok(_) => (StatusCode::OK, "Success".to_string()),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Discord Error: {e}"),
        ),
    }
}
