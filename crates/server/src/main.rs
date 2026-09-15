use std::path::PathBuf;
use std::time::Duration;

use ebru_server::{AppState, Config, Workshop, http, store::Store};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().with_target(false).init();

    let address = std::env::var("EBRU_ADDR").unwrap_or_else(|_| "127.0.0.1:8740".into());
    let data = PathBuf::from(std::env::var("EBRU_DATA").unwrap_or_else(|_| "data".into()));
    let web = std::env::var("EBRU_WEB").ok().map(PathBuf::from).filter(|p| p.is_dir());
    // Seconds of quiet before the tray starts marbling by itself; "off" to never.
    let idle_after = match std::env::var("EBRU_IDLE").as_deref() {
        Ok("off") => None,
        Ok(seconds) => Some(Duration::from_secs(seconds.parse()?)),
        Err(_) => Some(Duration::from_secs(20)),
    };

    std::fs::create_dir_all(&data)?;
    let store = Store::open(&data)?;
    let workshop = Workshop::start(store.clone(), idle_after);
    let state = AppState { workshop, store, config: Config { data, web: web.clone(), idle_after } };

    let listener = tokio::net::TcpListener::bind(&address).await?;
    tracing::info!(
        "the tray is ready at http://{}{}",
        listener.local_addr()?,
        if web.is_some() { "" } else { " (API only; run the web app separately)" }
    );
    axum::serve(listener, http::router(state)).with_graceful_shutdown(async {
        let _ = tokio::signal::ctrl_c().await;
    })
    .await?;
    Ok(())
}
