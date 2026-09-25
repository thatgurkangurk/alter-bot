use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use thiserror::Error;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

#[derive(Debug, Error)]
pub enum AppError {
    #[error("discord api error: {0}")]
    Serenity(#[from] serenity::Error),

    #[error("database error: {0}")]
    Database(#[from] sea_orm::DbErr),

    #[error("not found: {0}")]
    #[allow(dead_code)] // its FINE.
    NotFound(String),

    #[error("internal server error")]
    Internal(#[from] anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AppError::Serenity(err) => {
                tracing::error!("discord api error: {:?}", err);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "failed to request from discord",
                )
            }
            AppError::Database(err) => {
                tracing::error!("database Error: {:?}", err);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "database operation failed",
                )
            }
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.as_str()),
            AppError::Internal(err) => {
                tracing::error!("internal error: {:?}", err);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "an unexpected internal error occurred",
                )
            }
        };

        (
            status,
            Json(ErrorResponse {
                error: message.to_string(),
            }),
        )
            .into_response()
    }
}
