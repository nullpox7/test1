//! CSRF protection for cookie-based sessions.
//!
//! A random token lives in the server-side session. Templates emit it on
//! `<body hx-headers=...>` so every htmx request carries `X-CSRF-Token`.
//! State-changing methods without a matching header are rejected with 403.

use axum::{
    extract::Request,
    http::{Method, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use rand::{distr::Alphanumeric, Rng};
use tower_sessions::Session;

const SESSION_KEY: &str = "csrf_token";
pub const HEADER: &str = "x-csrf-token";

/// Return the session's token, creating one on first use.
pub async fn token(session: &Session) -> String {
    if let Ok(Some(t)) = session.get::<String>(SESSION_KEY).await {
        return t;
    }
    let t: String = rand::rng()
        .sample_iter(&Alphanumeric)
        .take(32)
        .map(char::from)
        .collect();
    // Insert failures only happen if the store is down; surface as a fresh token anyway.
    let _ = session.insert(SESSION_KEY, t.clone()).await;
    t
}

pub async fn require_token(session: Session, req: Request, next: Next) -> Response {
    if matches!(
        *req.method(),
        Method::GET | Method::HEAD | Method::OPTIONS | Method::TRACE
    ) {
        return next.run(req).await;
    }

    let expected = session.get::<String>(SESSION_KEY).await.ok().flatten();
    let provided = req
        .headers()
        .get(HEADER)
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);

    match (expected, provided) {
        (Some(e), Some(p)) if constant_time_eq(e.as_bytes(), p.as_bytes()) => next.run(req).await,
        _ => (StatusCode::FORBIDDEN, "invalid or missing CSRF token").into_response(),
    }
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}
