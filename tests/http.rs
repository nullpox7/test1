//! Black-box HTTP tests. These lock in the observable behaviour so an agent
//! refactoring handlers or templates gets a red build on regressions.

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
    Router,
};
use http_body_util::BodyExt;
use tower::ServiceExt;

use htmx_tasks::{app, AppState};

struct Client {
    app: Router,
    cookie: String,
    csrf: String,
}

impl Client {
    /// GET / once to obtain a session cookie and the CSRF token embedded in the page.
    async fn new(state: AppState) -> Self {
        let app = app(state);
        let resp = app
            .clone()
            .oneshot(Request::get("/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let cookie = resp
            .headers()
            .get(header::SET_COOKIE)
            .expect("session cookie")
            .to_str()
            .unwrap()
            .split(';')
            .next()
            .unwrap()
            .to_string();
        let body = body_string(resp).await;
        let csrf = extract_between(&body, r#""X-CSRF-Token": ""#, "\"").to_string();
        assert_eq!(csrf.len(), 32);
        Self { app, cookie, csrf }
    }

    async fn send(&self, req: Request<Body>) -> (StatusCode, String) {
        let resp = self.app.clone().oneshot(req).await.unwrap();
        let status = resp.status();
        (status, body_string(resp).await)
    }

    fn req(
        &self,
        method: Method,
        uri: &str,
        hx: bool,
        with_csrf: bool,
    ) -> axum::http::request::Builder {
        let mut b = Request::builder()
            .method(method)
            .uri(uri)
            .header(header::COOKIE, &self.cookie);
        if hx {
            b = b.header("HX-Request", "true");
        }
        if with_csrf {
            b = b.header("X-CSRF-Token", &self.csrf);
        }
        b
    }

    async fn form(
        &self,
        method: Method,
        uri: &str,
        form: &str,
        with_csrf: bool,
    ) -> (StatusCode, String) {
        let req = self
            .req(method, uri, true, with_csrf)
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(Body::from(form.to_string()))
            .unwrap();
        self.send(req).await
    }
}

async fn body_string(resp: axum::response::Response) -> String {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    String::from_utf8(bytes.to_vec()).unwrap()
}

fn extract_between<'a>(s: &'a str, start: &str, end: &str) -> &'a str {
    let i = s.find(start).expect("start marker") + start.len();
    let j = s[i..].find(end).expect("end marker") + i;
    &s[i..j]
}

#[tokio::test]
async fn full_page_for_browsers_fragment_for_htmx() {
    let c = Client::new(AppState::with_sample_data()).await;

    let (status, body) = c
        .send(
            c.req(Method::GET, "/", false, false)
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("<html"));
    assert!(body.contains(r#"id="task-list""#));
    assert!(body.contains("2 remaining"));

    let (status, body) = c
        .send(
            c.req(Method::GET, "/", true, false)
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(!body.contains("<html"), "htmx request must get a fragment");
    assert!(body.trim_start().starts_with("<ul id=\"task-list\""));
}

#[tokio::test]
async fn search_filters_by_title() {
    let c = Client::new(AppState::with_sample_data()).await;
    let (_, body) = c
        .send(
            c.req(Method::GET, "/?q=askama", true, false)
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert!(body.contains("Write an Askama template"));
    assert!(!body.contains("Ship it"));

    let (_, body) = c
        .send(
            c.req(Method::GET, "/?q=nothing-here", true, false)
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert!(body.contains("No tasks matching"));
}

#[tokio::test]
async fn mutations_without_csrf_token_are_rejected() {
    let c = Client::new(AppState::new()).await;
    let (status, _) = c.form(Method::POST, "/tasks", "title=x", false).await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let (status, _) = c
        .send(
            c.req(Method::DELETE, "/tasks/1", true, false)
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn create_returns_row_and_oob_counter_and_cleared_form() {
    let state = AppState::new();
    let c = Client::new(state.clone()).await;

    let (status, body) = c.form(Method::POST, "/tasks", "title=Buy+milk", true).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains(r#"<li id="task-1""#));
    assert!(body.contains("Buy milk"));
    assert!(body.contains(r#"id="remaining" class="remaining" hx-swap-oob="true""#));
    assert!(body.contains("1 remaining"));
    assert!(body.contains(r#"id="task-form" class="task-form" hx-swap-oob="true""#));
    assert!(body.contains(r#"value="""#), "form must be cleared");
    assert_eq!(state.remaining(), 1);
}

#[tokio::test]
async fn create_escapes_html_in_titles() {
    let c = Client::new(AppState::new()).await;
    let (status, body) = c
        .form(
            Method::POST,
            "/tasks",
            "title=%3Cscript%3Ealert(1)%3C%2Fscript%3E",
            true,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(!body.contains("<script>"));
    // Askama escapes `<` as `&#60;`; accept the named form too.
    assert!(body.contains("&#60;script") || body.contains("&lt;script"));
}

#[tokio::test]
async fn create_with_empty_title_returns_422_with_inline_error() {
    let state = AppState::new();
    let c = Client::new(state.clone()).await;
    let (status, body) = c.form(Method::POST, "/tasks", "title=+++", true).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(body.contains(r#"role="alert""#));
    assert!(body.contains("must not be empty"));
    assert_eq!(state.remaining(), 0);
}

#[tokio::test]
async fn toggle_and_delete_update_row_and_counter() {
    let state = AppState::new();
    let id = state.create("one");
    state.create("two");
    let c = Client::new(state.clone()).await;

    let (status, body) = c
        .form(Method::POST, &format!("/tasks/{id}/toggle"), "", true)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains(r#"class="task done""#));
    assert!(body.contains("1 remaining"));

    let (status, body) = c
        .send(
            c.req(Method::DELETE, &format!("/tasks/{id}"), true, true)
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(!body.contains("<li"), "delete must not return a row");
    assert!(body.contains(r#"hx-swap-oob="true""#));
    assert!(state.get(id).is_none());

    let (status, _) = c
        .send(
            c.req(Method::DELETE, &format!("/tasks/{id}"), true, true)
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
