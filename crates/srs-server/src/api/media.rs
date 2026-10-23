//! Image uploads for things SMILES can't describe (proteins, antibodies, instrument diagrams).

use axum::Json;
use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode, header};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::AppState;
use crate::error::{ApiError, ApiResult};

pub const MAX_BYTES: usize = 10 * 1024 * 1024;

#[derive(Debug, Serialize, Deserialize)]
pub struct Uploaded {
    /// Path to store in an image field, e.g. `/media/3f2a….png`.
    pub url: String,
}

fn extension(content_type: &str) -> Option<&'static str> {
    match content_type.split(';').next()?.trim() {
        "image/png" => Some("png"),
        "image/jpeg" => Some("jpg"),
        "image/webp" => Some("webp"),
        "image/gif" => Some("gif"),
        "image/svg+xml" => Some("svg"),
        _ => None,
    }
}

/// Store the request body as an image, named by its SHA-256 so re-uploads are free.
pub async fn upload(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> ApiResult<(StatusCode, Json<Uploaded>)> {
    let content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let ext = extension(content_type).ok_or_else(|| {
        ApiError::invalid(format!(
            "unsupported image type `{content_type}`; use PNG, JPEG, WebP, GIF, or SVG"
        ))
    })?;
    if body.is_empty() {
        return Err(ApiError::invalid("empty upload"));
    }
    let hash: String = Sha256::digest(&body).iter().map(|b| format!("{b:02x}")).collect();
    let name = format!("{hash}.{ext}");
    let path = state.media_dir.join(&name);
    if !tokio::fs::try_exists(&path).await.map_err(anyhow::Error::from)? {
        tokio::fs::create_dir_all(&state.media_dir)
            .await
            .map_err(anyhow::Error::from)?;
        tokio::fs::write(&path, &body)
            .await
            .map_err(anyhow::Error::from)?;
    }
    Ok((
        StatusCode::CREATED,
        Json(Uploaded {
            url: format!("/media/{name}"),
        }),
    ))
}
