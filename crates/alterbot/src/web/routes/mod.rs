pub mod api;
pub mod health;

use axum::Router;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};

pub fn create_router() -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .nest("/health", health::router())
        .nest("/api", api::router())
        .layer(TraceLayer::new_for_http())
        .layer(cors)
}
