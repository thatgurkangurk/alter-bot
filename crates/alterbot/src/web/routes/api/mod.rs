pub mod message;

use crate::web::middleware;
use axum::{Router, middleware as axum_middleware};

pub fn router() -> Router {
    Router::new()
        .nest("/messages", message::router())
        .layer(axum_middleware::from_fn(middleware::require_bearer_auth))
}
