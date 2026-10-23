use std::path::PathBuf;

use anyhow::Context;
use srs_server::{AppState, app, open_db, seed};
use tracing_subscriber::EnvFilter;

/// Configuration comes from environment variables so `cargo run` works with no arguments.
struct Config {
    bind: String,
    data_dir: PathBuf,
    seed_dir: PathBuf,
    web_dir: PathBuf,
    day_rollover_hour: u32,
}

impl Config {
    fn from_env() -> anyhow::Result<Self> {
        let var = |k: &str, default: &str| std::env::var(k).unwrap_or_else(|_| default.to_string());
        Ok(Self {
            // 0.0.0.0 so a phone on the same network can reach it.
            bind: var("SRS_BIND", "0.0.0.0:8080"),
            data_dir: var("SRS_DATA_DIR", "data").into(),
            seed_dir: var("SRS_SEED_DIR", "seed").into(),
            web_dir: var("SRS_WEB_DIR", "web/.output/public").into(),
            day_rollover_hour: var("SRS_DAY_ROLLOVER_HOUR", "4")
                .parse()
                .ok()
                .filter(|h| *h < 24)
                .context("SRS_DAY_ROLLOVER_HOUR must be 0-23")?,
        })
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info,tower_http=info".into()))
        .init();
    let config = Config::from_env()?;

    std::fs::create_dir_all(&config.data_dir)?;
    let db_path = config.data_dir.join("srs.sqlite");
    let pool = open_db(&format!("sqlite://{}", db_path.display())).await?;
    tracing::info!(db = %db_path.display(), "database ready");

    let imported = seed::import_dir(&pool, &config.seed_dir).await?;
    tracing::info!(sets = imported, dir = %config.seed_dir.display(), "seed sets imported");

    let mut state = AppState::new(pool);
    state.day_rollover_hour = config.day_rollover_hour;

    let listener = tokio::net::TcpListener::bind(&config.bind).await?;
    tracing::info!("listening on http://{}", config.bind);
    axum::serve(listener, app(state, Some(config.web_dir))).await?;
    Ok(())
}
