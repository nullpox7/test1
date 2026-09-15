//! Small helpers around htmx request/response headers.

use axum::{
    extract::FromRequestParts,
    http::{request::Parts, HeaderValue},
    response::{Html, IntoResponse, Response},
};
use std::convert::Infallible;

/// `true` when the request was issued by htmx (carries `HX-Request: true`).
/// Handlers use this to return a fragment instead of a full page.
#[derive(Clone, Copy, Debug)]
pub struct HxRequest(pub bool);

impl<S: Send + Sync> FromRequestParts<S> for HxRequest {
    type Rejection = Infallible;

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        let is_hx = parts
            .headers
            .get("hx-request")
            .map(|v| v == "true")
            .unwrap_or(false);
        Ok(HxRequest(is_hx))
    }
}

/// Render an Askama template into an HTML response.
/// A render failure is a programming error, so it maps to 500.
pub fn render<T: askama::Template>(t: &T) -> Response {
    match t.render() {
        Ok(body) => Html(body).into_response(),
        Err(err) => {
            tracing::error!(%err, "template render failed");
            axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

/// Concatenate several rendered fragments into one response
/// (main swap + out-of-band swaps). Pass `template.render()` results.
pub fn fragments(parts: Vec<askama::Result<String>>) -> Response {
    let mut body = String::new();
    for p in parts {
        match p {
            Ok(s) => body.push_str(&s),
            Err(err) => {
                tracing::error!(%err, "template render failed");
                return axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
        }
    }
    Html(body).into_response()
}

/// Attach `HX-Trigger` so client-side listeners can react (used for toasts).
pub fn with_trigger(mut resp: Response, event: &str) -> Response {
    if let Ok(v) = HeaderValue::from_str(event) {
        resp.headers_mut().insert("hx-trigger", v);
    }
    resp
}
