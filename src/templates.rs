//! One struct per template. Keep field names identical to what the
//! template reads so a mismatch fails at compile time.

use askama::Template;

use crate::state::Task;

/// Full page: layout + search box + form + list.
#[derive(Template)]
#[template(path = "index.html")]
pub struct IndexPage {
    pub csrf_token: String,
    pub query: String,
    pub tasks: Vec<Task>,
    pub count: RemainingCount,
    pub form: TaskForm,
}

/// Fragment: the list only (search results).
#[derive(Template)]
#[template(path = "tasks/list.html")]
pub struct TaskList {
    pub query: String,
    pub tasks: Vec<Task>,
}

/// Fragment: one `<li>`.
#[derive(Template)]
#[template(path = "tasks/row.html")]
pub struct TaskRow {
    pub task: Task,
}

/// Fragment: remaining counter. `oob = true` makes htmx swap it out-of-band.
#[derive(Template)]
#[template(path = "tasks/count.html")]
pub struct RemainingCount {
    pub remaining: usize,
    pub oob: bool,
}

/// Fragment: the create form. `error` is shown inline on validation failure.
#[derive(Template, Clone, Default)]
#[template(path = "tasks/form.html")]
pub struct TaskForm {
    pub title: String,
    pub error: Option<String>,
    pub oob: bool,
}
