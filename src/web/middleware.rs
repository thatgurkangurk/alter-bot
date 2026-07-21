use super::AppState;
use axum::{
    extract::{Request, State},
    http::{StatusCode, header},
    middleware::Next,
    response::Response,
};

pub async fn require_bearer_auth(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Result<Response, (StatusCode, &'static str)> {
    let auth_header = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok());

    let token = match auth_header {
        Some(header) if header.starts_with("Bearer ") => &header["Bearer ".len()..],
        _ => {
            return Err((
                StatusCode::UNAUTHORIZED,
                "Missing or invalid Authorization header format",
            ));
        }
    };

    let config = state.config_manager.get().await;
    let is_authorized = config.web.tokens.iter().any(|t| t == token);

    if !is_authorized {
        return Err((StatusCode::UNAUTHORIZED, "Invalid bearer token"));
    }

    Ok(next.run(req).await)
}
