# htmx-tasks — conventions for humans and agents

This is an htmx + axum + Askama app. The server is the single source of truth.

## Architecture rules
- Every handler returns HTML. Full page when the request comes from a browser,
  a fragment when `HX-Request: true` (see `src/htmx.rs::HxRequest`).
- One Askama struct per fragment in `src/templates.rs`; template field names
  must match struct fields (compile-time checked).
- Mutations return the changed fragment plus out-of-band fragments
  (`hx-swap-oob`) for anything else that changed (counter, form).
- Client-side JS is limited to `static/app.js` and only reacts to `HX-Trigger`
  events. Do not add client-side state stores. Never put tokens or personal
  data in localStorage/sessionStorage.

## Data rules
- SQL lives only in `src/state.rs`. Handlers call `AppState` methods and
  never touch the pool directly.
- Schema changes are new files in `migrations/` (never edit an applied one);
  `sqlx::migrate!` applies them at startup. Queries use `query`/`query_as`
  with `bind`, never string formatting.
- Tests get a fresh database from `AppState::in_memory()`; never share a
  file DB between tests.

## Security rules
- Auth/session state stays in the server session (HttpOnly cookie).
- All state-changing routes go through `csrf::require_token`. New mutating
  routes must be `POST`/`DELETE`/`PUT`/`PATCH`, never `GET`.
- Never disable Askama autoescaping (`|safe`) on user-supplied strings.

## Before pushing
```
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
```
Add or update a test in `tests/http.rs` for every behaviour change.
