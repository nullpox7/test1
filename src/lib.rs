//! htmx + axum + Askama task manager.
//!
//! Layout:
//! - `state`     : SQLite-backed state via sqlx (the single source of truth)
//! - `error`     : storage error → HTTP 500 mapping
//! - `csrf`      : session-bound CSRF token + middleware
//! - `htmx`      : request/response helpers for htmx headers
//! - `templates` : Askama template structs, one per fragment
//! - `handlers`  : HTTP handlers returning full pages or fragments

mod csrf;
mod error;
mod handlers;
mod htmx;
mod state;
mod templates;

pub use state::{AppState, Task};

use axum::{
    middleware,
    routing::{get, post},
    Router,
};
use tower_http::services::ServeDir;
use tower_sessions::{MemoryStore, SessionManagerLayer};

/// Build the application router. Tests call this directly.
pub fn app(state: AppState) -> Router {
    let session_layer = SessionManagerLayer::new(MemoryStore::default())
        .with_secure(false) // set true behind TLS
        .with_http_only(true)
        .with_same_site(tower_sessions::cookie::SameSite::Lax);

    Router::new()
        .route("/", get(handlers::index))
        .route("/tasks", post(handlers::create))
        .route("/tasks/{id}/toggle", post(handlers::toggle))
        .route("/tasks/{id}", axum::routing::delete(handlers::delete))
        .nest_service("/static", ServeDir::new("static"))
        .layer(middleware::from_fn(csrf::require_token))
        .layer(session_layer)
        .with_state(state)
}
