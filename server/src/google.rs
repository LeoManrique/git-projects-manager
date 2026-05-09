use anyhow::{Context, Result, anyhow};
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header, jwk::JwkSet};
use parking_lot::RwLock;
use serde::Deserialize;
use std::sync::Arc;
use std::time::{Duration, Instant};

const JWKS_URL: &str = "https://www.googleapis.com/oauth2/v3/certs";
const ISSUERS: &[&str] = &["https://accounts.google.com", "accounts.google.com"];
const JWKS_TTL: Duration = Duration::from_secs(3600);

#[derive(Debug, Deserialize)]
pub struct GoogleClaims {
    pub sub: String,
    pub email: Option<String>,
    pub name: Option<String>,
    pub email_verified: Option<bool>,
}

#[derive(Clone)]
pub struct GoogleVerifier {
    client_id: String,
    http: reqwest::Client,
    cache: Arc<RwLock<Option<JwksCache>>>,
}

struct JwksCache {
    jwks: JwkSet,
    fetched_at: Instant,
}

impl GoogleVerifier {
    pub fn new(client_id: String) -> Self {
        Self {
            client_id,
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(8))
                .build()
                .expect("reqwest client"),
            cache: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn verify(&self, id_token: &str) -> Result<GoogleClaims> {
        let header = decode_header(id_token).context("invalid id_token header")?;
        let kid = header.kid.ok_or_else(|| anyhow!("id_token missing kid"))?;

        let jwks = self.jwks().await?;
        let jwk = jwks
            .find(&kid)
            .ok_or_else(|| anyhow!("kid {kid} not found in Google JWKS"))?;
        let key = DecodingKey::from_jwk(jwk)?;

        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_audience(&[&self.client_id]);
        validation.set_issuer(ISSUERS);

        let data = decode::<GoogleClaims>(id_token, &key, &validation)
            .context("id_token validation failed")?;

        if data.claims.email_verified == Some(false) {
            return Err(anyhow!("email not verified"));
        }
        Ok(data.claims)
    }

    async fn jwks(&self) -> Result<JwkSet> {
        if let Some(cached) = self.cache.read().as_ref() {
            if cached.fetched_at.elapsed() < JWKS_TTL {
                return Ok(cached.jwks.clone());
            }
        }
        let fresh: JwkSet = self
            .http
            .get(JWKS_URL)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        *self.cache.write() = Some(JwksCache {
            jwks: fresh.clone(),
            fetched_at: Instant::now(),
        });
        Ok(fresh)
    }
}
