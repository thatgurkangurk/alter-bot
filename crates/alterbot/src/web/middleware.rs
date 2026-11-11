use axum::{
    extract::Request,
    http::{StatusCode, header},
    middleware::Next,
    response::Response,
};

pub async fn require_bearer_auth(
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
                "missing or invalid Authorization header format",
            ));
        }
    };

    let config = crate::context::config();
    let is_authorized = config.web.tokens.iter().any(|t| t == token);

    if !is_authorized {
        return Err((StatusCode::UNAUTHORIZED, "invalid bearer token"));
    }

    Ok(next.run(req).await)
}
