use std::net::SocketAddr;

use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://tasks.db".to_string());
    let state = htmx_tasks::AppState::connect(&database_url)
        .await
        .expect("open database and run migrations");
    match state.seed_if_empty().await {
        Ok(0) => {}
        Ok(n) => tracing::info!("seeded {n} sample tasks"),
        Err(err) => tracing::warn!(%err, "seeding failed"),
    }
    let app = htmx_tasks::app(state);

    let addr: SocketAddr = std::env::var("BIND")
        .unwrap_or_else(|_| "127.0.0.1:3000".to_string())
        .parse()
        .expect("BIND must be host:port");
    let listener = tokio::net::TcpListener::bind(addr).await.expect("bind");
    tracing::info!("listening on http://{addr}");
    axum::serve(listener, app).await.expect("server");
}
