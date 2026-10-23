//! Minimal client for the Claude Messages API, used to draft notes. There is no official Rust
//! SDK, so this speaks the HTTP API directly.

use std::time::Duration;

use anyhow::{Context, anyhow};
use serde_json::{Value, json};

pub const DEFAULT_BASE_URL: &str = "https://api.anthropic.com";
pub const DEFAULT_MODEL: &str = "claude-opus-5-5";
const API_VERSION: &str = "2026-10-21";
/// Lets the API retry a refused request on a fallback model instead of returning the refusal.
const FALLBACK_BETA: &str = "server-side-fallback-2026-07-01";
const MAX_TOKENS: u32 = 16_000;

#[derive(Debug, Clone)]
pub struct Claude {
    http: reqwest::Client,
    api_key: String,
    base_url: String,
    pub model: String,
}

/// Why a request produced no usable output. Shown to the user as-is.
#[derive(Debug, thiserror::Error)]
pub enum ClaudeError {
    #[error("Claude declined to draft this. Try rephrasing the topic.")]
    Refused,
    #[error("The draft was too long and got cut off. Ask for fewer notes.")]
    Truncated,
    #[error("The Claude API returned an error ({status}): {message}")]
    Api { status: u16, message: String },
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl Claude {
    pub fn new(api_key: String, base_url: Option<String>, model: Option<String>) -> Self {
        Self {
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(600))
                .build()
                .expect("HTTP client"),
            api_key,
            base_url: base_url.unwrap_or_else(|| DEFAULT_BASE_URL.to_string()),
            model: model.unwrap_or_else(|| DEFAULT_MODEL.to_string()),
        }
    }

    /// `ANTHROPIC_API_KEY` enables drafting. `SRS_ANTHROPIC_BASE_URL` and `SRS_CLAUDE_MODEL`
    /// override the defaults.
    pub fn from_env() -> Option<Self> {
        let key = std::env::var("ANTHROPIC_API_KEY")
            .ok()
            .filter(|k| !k.trim().is_empty())?;
        Some(Self::new(
            key,
            std::env::var("SRS_ANTHROPIC_BASE_URL").ok(),
            std::env::var("SRS_CLAUDE_MODEL").ok(),
        ))
    }

    /// Send one message and return the reply parsed as JSON matching `schema`.
    pub async fn structured(&self, system: &str, user: &str, schema: Value) -> Result<Value, ClaudeError> {
        let body = json!({
            "model": self.model,
            "max_tokens": MAX_TOKENS,
            "system": system,
            "messages": [{ "role": "user", "content": user }],
            "output_config": {
                "effort": "high",
                "format": { "type": "json_schema", "schema": schema },
            },
            "fallbacks": "default",
        });
        let response = self
            .http
            .post(format!("{}/v1/messages", self.base_url.trim_end_matches('/')))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", API_VERSION)
            .header("anthropic-beta", FALLBACK_BETA)
            .json(&body)
            .send()
            .await
            .context("could not reach the Claude API")?;
        let status = response.status();
        let reply: Value = response
            .json()
            .await
            .context("unreadable reply from the Claude API")?;
        if !status.is_success() {
            let message = reply["error"]["message"]
                .as_str()
                .unwrap_or("unknown error")
                .to_string();
            return Err(ClaudeError::Api {
                status: status.as_u16(),
                message,
            });
        }
        match reply["stop_reason"].as_str() {
            Some("refusal") => return Err(ClaudeError::Refused),
            Some("max_tokens") => return Err(ClaudeError::Truncated),
            _ => {}
        }
        let text: String = reply["content"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|b| b["type"] == "text")
            .filter_map(|b| b["text"].as_str())
            .collect();
        serde_json::from_str(&text).map_err(|e| anyhow!("reply was not valid JSON: {e}").into())
    }
}
