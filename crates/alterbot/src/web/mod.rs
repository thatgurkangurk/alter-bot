pub mod middleware;
mod module;
pub mod routes;

use anyhow::Context;
use std::net::SocketAddr;
use tokio::net::TcpListener;

pub mod error {
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
    #[allow(dead_code)]
    pub enum AppError {
        #[error("twilight http error: {0}")]
        TwilightHttp(#[from] twilight_http::Error),

        #[error("twilight deserialization error: {0}")]
        TwilightDeserialize(#[from] twilight_http::response::DeserializeBodyError),

        #[error("database error: {0}")]
        Database(#[from] sea_orm::DbErr),

        #[error("bad request: {0}")]
        BadRequest(String),

        #[error("not found: {0}")]
        NotFound(String),

        #[error("unauthorized: {0}")]
        Unauthorized(String),

        #[error("internal server error: {0}")]
        Internal(#[from] anyhow::Error),
    }

    impl IntoResponse for AppError {
        fn into_response(self) -> Response {
            let (status, message) = match &self {
                AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.to_string()),
                AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.to_string()),
                AppError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg.to_string()),
                AppError::TwilightHttp(err) => {
                    tracing::error!("discord api error: {:?}", err);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "failed to request from discord api".to_string(),
                    )
                }
                AppError::TwilightDeserialize(err) => {
                    tracing::error!("discord response deserialization error: {:?}", err);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "failed to parse discord api response".to_string(),
                    )
                }
                AppError::Database(err) => {
                    tracing::error!("database error: {:?}", err);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "database operation failed".to_string(),
                    )
                }
                AppError::Internal(err) => {
                    tracing::error!("internal error: {:?}", err);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "an unexpected internal error occurred".to_string(),
                    )
                }
            };

            (status, Json(ErrorResponse { error: message })).into_response()
        }
    }
}

pub async fn run(addr: SocketAddr) -> anyhow::Result<()> {
    let app = routes::create_router();

    let listener = TcpListener::bind(addr)
        .await
        .with_context(|| format!("failed to bind web server to {addr}"))?;

    tracing::info!("web server listening on http://{addr}");

    axum::serve(listener, app)
        .await
        .context("axum server error")?;

    Ok(())
}
