use crate::{
    features::polls::{CreatePollParams, create_and_post_poll},
    models::poll,
    web::AppState,
};
use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use poise::serenity_prelude::{ChannelId, GuildId, RoleId};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct OptionInput {
    pub name: String,
    pub weight: Option<f64>,
}

#[derive(Deserialize)]
pub struct CreatePollRequest {
    pub title: String,
    pub guild_id: String,
    pub channel_id: String,
    pub duration_minutes: i64,
    pub required_role_id: Option<String>,
    pub options: Vec<OptionInput>,
}

use serde::Serialize;
use uuid::Uuid;

#[derive(Serialize)]
pub struct PollResponse {
    pub id: Uuid,
    pub title: String,
    pub channel_id: u64,
    pub guild_id: u64,
    pub ends_at: String,
    pub is_active: bool,
}

impl From<poll::Model> for PollResponse {
    fn from(m: poll::Model) -> Self {
        Self {
            id: m.id,
            title: m.title,
            #[allow(clippy::cast_sign_loss)]
            channel_id: m.channel_id as u64,
            #[allow(clippy::cast_sign_loss)]
            guild_id: m.guild_id as u64,
            ends_at: m.ends_at.to_string(),
            is_active: m.is_active,
        }
    }
}

pub async fn create_poll_handler(
    State(state): State<AppState>,
    Json(payload): Json<CreatePollRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    if payload.options.len() < 2 {
        return Err((
            StatusCode::BAD_REQUEST,
            "at least two poll options are required".to_string(),
        ));
    }

    let guild_id_u64 = payload.guild_id.parse::<u64>().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            "invalid guild_id string format".to_string(),
        )
    })?;

    let channel_id_u64 = payload.channel_id.parse::<u64>().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            "invalid channel_id string format".to_string(),
        )
    })?;

    let required_role_id = match payload.required_role_id {
        Some(ref role_str) if !role_str.trim().is_empty() => {
            let parsed = role_str.parse::<u64>().map_err(|_| {
                (
                    StatusCode::BAD_REQUEST,
                    "invalid required_role_id string format".to_string(),
                )
            })?;
            Some(RoleId::new(parsed))
        }
        _ => None,
    };

    let formatted_options: Vec<Option<String>> = payload
        .options
        .into_iter()
        .map(|opt| {
            let formatted = match opt.weight {
                Some(weight) => format!("{}:{}", opt.name, weight),
                None => opt.name,
            };
            Some(formatted)
        })
        .collect();

    let params = CreatePollParams {
        title: payload.title,
        guild_id: GuildId::new(guild_id_u64),
        target_channel_id: ChannelId::new(channel_id_u64),
        duration_minutes: payload.duration_minutes,
        required_role_id,
        raw_inputs: formatted_options,
    };

    let poll = create_and_post_poll(&state.db, &state.http, params)
        .await
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

    let ends_at = poll.ends_at.to_utc();
    state.poll_cache.write().await.insert(poll.id, ends_at);

    // todo: add an endpoint to get a poll (maybe)
    // let location_header = (header::LOCATION, format!("/api/polls/{}", poll.id));
    let body = Json(PollResponse::from(poll));

    Ok((StatusCode::CREATED, body))
}
