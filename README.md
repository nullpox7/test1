# htmx-tasks

A small task manager built with **htmx + Rust (axum) + Askama**. It exists to
show the patterns that make a server-rendered app easy for humans and AI
agents to maintain:

- full page vs. fragment from the same templates, switched on `HX-Request`
- out-of-band swaps for counters and form reset
- server-side sessions, `HttpOnly` cookie, CSRF token on every htmx request
- SQLite persistence via sqlx (WAL mode), migrations applied at startup, in-memory DB for tests
- compile-time checked templates (Askama) and black-box HTTP tests

## Run

```
cargo run
# open http://127.0.0.1:3000
```

Set `BIND=0.0.0.0:8080` to change the address. The database defaults to
`sqlite://tasks.db` in the working directory; override with `DATABASE_URL`.
Migrations in `migrations/` run automatically at startup, and three sample
tasks are inserted the first time the table is empty.

## Test

```
cargo test
```

## Layout

| Path | Role |
|---|---|
| `src/state.rs` | SQLite store (sqlx); the only module that knows SQL |
| `src/error.rs` | `sqlx::Error` → HTTP 500 |
| `migrations/` | schema, applied by `sqlx::migrate!` at startup |
| `src/csrf.rs` | session-bound CSRF token + middleware |
| `src/htmx.rs` | `HxRequest` extractor, fragment rendering, `HX-Trigger` |
| `src/templates.rs` | one Askama struct per template |
| `src/handlers.rs` | route handlers |
| `templates/` | `layout.html`, `index.html`, `tasks/*.html` fragments |
| `static/` | vendored `htmx.min.js` (2.0.4), `app.css`, `app.js` |
| `tests/http.rs` | behaviour tests via `tower::ServiceExt::oneshot`, each on a fresh in-memory DB |

## Routes

| Method | Path | Returns |
|---|---|---|
| GET | `/?q=` | page, or `#task-list` fragment for htmx |
| POST | `/tasks` | new `<li>` + oob counter + oob cleared form; 422 with inline error |
| POST | `/tasks/{id}/toggle` | replaced `<li>` + oob counter |
| DELETE | `/tasks/{id}` | oob counter only (empty main swap removes the row) |
