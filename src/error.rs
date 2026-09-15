//! Map storage failures to a 500 without leaking details to the client.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

#[derive(Debug)]
pub struct AppError(pub sqlx::Error);

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        Self(e)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        tracing::error!(error = %self.0, "database error");
        (StatusCode::INTERNAL_SERVER_ERROR, "internal error").into_response()
    }
}
