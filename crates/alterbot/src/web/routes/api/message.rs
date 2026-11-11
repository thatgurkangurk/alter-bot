use axum::{Json, Router, routing::post};
use serde::{Deserialize, Serialize};
use twilight_model::id::{
    Id,
    marker::{ChannelMarker, MessageMarker},
};

use crate::web::error::{AppError, AppResult};

#[derive(Debug, Deserialize)]
pub struct MessageRequest {
    /// Discord Channel Snowflake ID
    pub channel_id: String,
    /// Content of the message to send
    pub message: String,
    /// Optional Discord Message Snowflake ID to reply to
    pub reply_to_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct MessageResponse {
    pub success: bool,
    pub message: String,
}

pub fn router() -> Router {
    Router::new().route("/", post(send_message_handler))
}

pub async fn send_message_handler(
    Json(body): Json<MessageRequest>,
) -> AppResult<Json<MessageResponse>> {
    let channel_id: Id<ChannelMarker> = body
        .channel_id
        .parse::<u64>()
        .map(Id::new)
        .map_err(|_| AppError::BadRequest("invalid channel_id format".into()))?;

    let mut create_msg = crate::context::http()
        .create_message(channel_id)
        .content(&body.message);

    if let Some(reply_id_str) = body.reply_to_id {
        let reply_id: Id<MessageMarker> = reply_id_str
            .parse::<u64>()
            .map(Id::new)
            .map_err(|_| AppError::BadRequest("invalid reply_to_id format".into()))?;

        create_msg = create_msg.reply(reply_id);
    }

    create_msg.await?;

    Ok(Json(MessageResponse {
        success: true,
        message: "message dispatched successfully".to_string(),
    }))
}
