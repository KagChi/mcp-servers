use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};

/// API key authentication middleware
///
/// Validates Bearer token in Authorization header against configured API key.
/// If API key is empty, authentication is disabled and all requests pass through.
/// Returns 401 Unauthorized if missing or invalid when authentication is enabled.
pub async fn auth_middleware(
    State(api_key): State<String>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    // If API key is empty, authentication is disabled
    if api_key.is_empty() {
        return Ok(next.run(request).await);
    }

    // Extract Authorization header
    let auth_header = request
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok());

    // Validate Bearer token format and match against API key
    match auth_header {
        Some(header) if header.starts_with("Bearer ") => {
            let token = &header[7..]; // Skip "Bearer " prefix
            if token == api_key {
                Ok(next.run(request).await)
            } else {
                Err(StatusCode::UNAUTHORIZED)
            }
        }
        _ => Err(StatusCode::UNAUTHORIZED),
    }
}
