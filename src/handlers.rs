use askama::Template;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Form,
};
use serde::Deserialize;
use tower_sessions::Session;

use crate::{
    csrf,
    error::AppError,
    htmx::{fragments, render, with_trigger, HxRequest},
    state::AppState,
    templates::{IndexPage, RemainingCount, TaskForm, TaskList, TaskRow},
};

const MAX_TITLE_LEN: usize = 200;

#[derive(Deserialize, Default)]
pub struct SearchParams {
    #[serde(default)]
    pub q: String,
}

/// `GET /?q=` — full page for browsers, list fragment for htmx.
pub async fn index(
    State(state): State<AppState>,
    session: Session,
    HxRequest(is_hx): HxRequest,
    Query(params): Query<SearchParams>,
) -> Result<Response, AppError> {
    let tasks = state.search(&params.q).await?;

    if is_hx {
        return Ok(render(&TaskList {
            query: params.q,
            tasks,
        }));
    }

    Ok(render(&IndexPage {
        csrf_token: csrf::token(&session).await,
        query: params.q,
        tasks,
        count: RemainingCount {
            remaining: state.remaining().await?,
            oob: false,
        },
        form: TaskForm::default(),
    }))
}

#[derive(Deserialize)]
pub struct CreateForm {
    #[serde(default)]
    pub title: String,
}

/// `POST /tasks` — on success return the new row plus an out-of-band
/// counter and a cleared form. On validation error return 422 with the
/// form re-rendered inline.
pub async fn create(
    State(state): State<AppState>,
    Form(form): Form<CreateForm>,
) -> Result<Response, AppError> {
    let title = form.title.trim();
    if let Some(error) = validate_title(title) {
        let body = render(&TaskForm {
            title: form.title.clone(),
            error: Some(error),
            oob: false,
        });
        return Ok((StatusCode::UNPROCESSABLE_ENTITY, body).into_response());
    }

    let id = state.create(title).await?;
    let task = state.get(id).await?.ok_or(sqlx::Error::RowNotFound)?;

    let resp = fragments(vec![
        TaskRow { task }.render(),
        RemainingCount {
            remaining: state.remaining().await?,
            oob: true,
        }
        .render(),
        TaskForm {
            title: String::new(),
            error: None,
            oob: true,
        }
        .render(),
    ]);
    Ok(with_trigger(resp, "task-created"))
}

/// `POST /tasks/{id}/toggle` — replace the row and update the counter.
pub async fn toggle(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Response, AppError> {
    match state.toggle(id).await? {
        Some(task) => Ok(fragments(vec![
            TaskRow { task }.render(),
            RemainingCount {
                remaining: state.remaining().await?,
                oob: true,
            }
            .render(),
        ])),
        None => Ok(StatusCode::NOT_FOUND.into_response()),
    }
}

/// `DELETE /tasks/{id}` — empty main swap removes the row; counter goes out-of-band.
pub async fn delete(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Response, AppError> {
    if !state.delete(id).await? {
        return Ok(StatusCode::NOT_FOUND.into_response());
    }
    Ok(render(&RemainingCount {
        remaining: state.remaining().await?,
        oob: true,
    }))
}

fn validate_title(title: &str) -> Option<String> {
    if title.is_empty() {
        Some("Title must not be empty.".to_string())
    } else if title.chars().count() > MAX_TITLE_LEN {
        Some(format!("Title must be at most {MAX_TITLE_LEN} characters."))
    } else {
        None
    }
}
