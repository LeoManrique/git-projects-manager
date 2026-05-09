use crate::config;
use crate::domain::auth::{SyncSession, SyncUser};
use crate::domain::kanban::KanbanCard;
use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub struct SyncClient {
    http: reqwest::Client,
    base_url: String,
}

#[derive(Debug)]
pub enum SyncOutcome {
    Ok(Vec<KanbanCard>),
    Unauthorized,
    Network(String),
}

#[derive(Serialize)]
struct AuthRequest<'a> {
    id_token: &'a str,
}

#[derive(Deserialize)]
struct WireUser {
    sub: String,
    #[serde(default)]
    email: Option<String>,
    #[serde(default)]
    name: Option<String>,
}

#[derive(Deserialize)]
struct AuthResponse {
    session_token: String,
    expires_at: i64,
    user: WireUser,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SyncRequest<'a> {
    cards: &'a [KanbanCard],
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SyncResponse {
    cards: Vec<KanbanCard>,
}

impl SyncClient {
    pub fn new() -> Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .context("failed to build HTTP client")?;
        Ok(Self {
            http,
            base_url: config::sync_server_url(),
        })
    }

    pub async fn sign_in(&self, id_token: &str) -> Result<SyncSession> {
        let url = format!("{}/v1/auth/google", self.base_url);
        let resp = self
            .http
            .post(&url)
            .json(&AuthRequest { id_token })
            .send()
            .await
            .context("sign-in request failed")?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("server rejected sign-in ({status}): {body}"));
        }
        let body: AuthResponse = resp.json().await.context("invalid sign-in response")?;
        Ok(SyncSession {
            token: body.session_token,
            expires_at: body.expires_at,
            user: SyncUser {
                sub: body.user.sub,
                email: body.user.email,
                name: body.user.name,
            },
        })
    }

    pub async fn sign_out(&self, token: &str) -> Result<()> {
        let url = format!("{}/v1/auth/sign-out", self.base_url);
        let resp = self
            .http
            .post(&url)
            .bearer_auth(token)
            .send()
            .await
            .context("sign-out request failed")?;
        // 401 here just means the server already considers us signed-out — treat as success.
        if !resp.status().is_success() && resp.status() != reqwest::StatusCode::UNAUTHORIZED {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("server rejected sign-out ({status}): {body}"));
        }
        Ok(())
    }

    pub async fn sync(&self, token: &str, cards: &[KanbanCard]) -> SyncOutcome {
        let url = format!("{}/v1/sync", self.base_url);
        let resp = match self
            .http
            .post(&url)
            .bearer_auth(token)
            .json(&SyncRequest { cards })
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => return SyncOutcome::Network(e.to_string()),
        };

        if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
            return SyncOutcome::Unauthorized;
        }
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return SyncOutcome::Network(format!("server returned {status}: {body}"));
        }

        match resp.json::<SyncResponse>().await {
            Ok(body) => SyncOutcome::Ok(body.cards),
            Err(e) => SyncOutcome::Network(format!("invalid sync response: {e}")),
        }
    }
}
