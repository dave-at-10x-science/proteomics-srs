//! Local server for the proteomics SRS: serves the web app and a JSON API over SQLite.

pub mod api;
pub mod error;
pub mod seed;
pub mod store;

use std::path::PathBuf;
use std::sync::Arc;

use axum::Router;
use chrono::{DateTime, Duration, Local, NaiveTime, TimeZone, Utc};
use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;

/// The single built-in user until accounts exist.
pub const LOCAL_USER: i64 = 1;

pub type Clock = Arc<dyn Fn() -> DateTime<Utc> + Send + Sync>;

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub clock: Clock,
    /// Local hour at which a new study day starts (e.g. 4 = 4am).
    pub day_rollover_hour: u32,
}

impl AppState {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool,
            clock: Arc::new(Utc::now),
            day_rollover_hour: 4,
        }
    }

    pub fn now(&self) -> DateTime<Utc> {
        (self.clock)()
    }

    /// Start and end of the study day containing `now`, in the machine's local time zone.
    pub fn study_day(&self, now: DateTime<Utc>) -> (DateTime<Utc>, DateTime<Utc>) {
        let local = now.with_timezone(&Local);
        let rollover = NaiveTime::from_hms_opt(self.day_rollover_hour, 0, 0).expect("valid hour");
        let mut start_date = local.date_naive();
        if local.time() < rollover {
            start_date = start_date.pred_opt().expect("date in range");
        }
        let start = Local
            .from_local_datetime(&start_date.and_time(rollover))
            .earliest()
            .unwrap_or(local)
            .with_timezone(&Utc);
        (start, start + Duration::days(1))
    }
}

/// Open (creating if needed) the SQLite database and apply migrations.
pub async fn open_db(url: &str) -> anyhow::Result<SqlitePool> {
    let options = url
        .parse::<SqliteConnectOptions>()?
        .create_if_missing(true)
        .foreign_keys(true);
    // In-memory databases are per-connection, so keep a single one for tests.
    let max = if url.contains(":memory:") { 1 } else { 5 };
    let pool = SqlitePoolOptions::new()
        .max_connections(max)
        .connect_with(options)
        .await?;
    sqlx::migrate!("./migrations").run(&pool).await?;
    Ok(pool)
}

/// The full app: JSON API under `/api`, and the built web app (if present) for everything else.
pub fn app(state: AppState, web_dir: Option<PathBuf>) -> Router {
    let router = Router::new().nest("/api", api::router()).with_state(state);
    let router = match web_dir {
        Some(dir) if dir.join("index.html").exists() => {
            // SPA: unknown paths fall back to index.html so client-side routes work on reload.
            let index = ServeFile::new(dir.join("index.html"));
            router.fallback_service(ServeDir::new(dir).not_found_service(index))
        }
        _ => router
            .fallback(|| async { "proteomics-srs API is running. Build the web app (web/) to get the UI." }),
    };
    router.layer(TraceLayer::new_for_http())
}
