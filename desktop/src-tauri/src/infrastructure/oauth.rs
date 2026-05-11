use anyhow::{Context, Result, anyhow};
use oauth2::{CsrfToken, PkceCodeChallenge};
use serde::Deserialize;
use std::time::{Duration, Instant};

const AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Deserialize)]
struct TokenResponse {
    id_token: String,
}

/// Run a Google OAuth loopback flow with PKCE in the user's system browser.
/// Returns the Google ID token on success.
///
/// `open_url` is a callback the caller provides to open the system browser
/// (typically `tauri_plugin_shell` opener).
pub async fn run_loopback_pkce(
    client_id: &str,
    client_secret: &str,
    open_url: impl FnOnce(&str) -> Result<()>,
) -> Result<String> {
    let listener = tiny_http::Server::http("127.0.0.1:0")
        .map_err(|e| anyhow!("failed to bind loopback listener: {e}"))?;
    let addr = listener
        .server_addr()
        .to_ip()
        .context("loopback listener missing IP addr")?;
    let port = addr.port();
    let redirect_uri = format!("http://127.0.0.1:{port}/callback");

    let (challenge, verifier) = PkceCodeChallenge::new_random_sha256();
    let state = CsrfToken::new_random();
    let expected_state = state.secret().clone();

    let mut auth_url = url::Url::parse(AUTH_URL)?;
    auth_url
        .query_pairs_mut()
        .append_pair("client_id", client_id)
        .append_pair("redirect_uri", &redirect_uri)
        .append_pair("response_type", "code")
        .append_pair("scope", "openid email profile")
        .append_pair("state", state.secret())
        .append_pair("code_challenge", challenge.as_str())
        .append_pair("code_challenge_method", "S256")
        .append_pair("access_type", "online")
        .append_pair("prompt", "select_account");

    open_url(auth_url.as_str()).context("failed to open system browser")?;

    let task = tokio::task::spawn_blocking(move || -> Result<(String, String)> {
        let deadline = Instant::now() + TIMEOUT;
        loop {
            let now = Instant::now();
            if now >= deadline {
                return Err(anyhow!("timed out waiting for OAuth callback"));
            }
            let request = match listener
                .recv_timeout(deadline - now)
                .context("loopback listener error")?
            {
                Some(r) => r,
                None => continue,
            };

            let url_str = format!("http://localhost{}", request.url());
            let parsed = match url::Url::parse(&url_str) {
                Ok(u) => u,
                Err(_) => {
                    let _ = request.respond(
                        tiny_http::Response::from_string("bad request").with_status_code(400),
                    );
                    continue;
                }
            };

            if parsed.path() != "/callback" {
                let _ = request.respond(
                    tiny_http::Response::from_string("not found").with_status_code(404),
                );
                continue;
            }

            let mut code = None;
            let mut got_state = None;
            let mut error = None;
            for (k, v) in parsed.query_pairs() {
                match k.as_ref() {
                    "code" => code = Some(v.into_owned()),
                    "state" => got_state = Some(v.into_owned()),
                    "error" => error = Some(v.into_owned()),
                    _ => {}
                }
            }

            if let Some(e) = error {
                respond_html(
                    request,
                    400,
                    &format!("<h2>Sign-in failed</h2><p>{}</p>", html_escape(&e)),
                );
                return Err(anyhow!("authorization error: {e}"));
            }

            let (Some(code), Some(got_state)) = (code, got_state) else {
                respond_html(request, 400, "<h2>Sign-in failed</h2><p>Missing code or state.</p>");
                return Err(anyhow!("callback missing code or state"));
            };

            respond_html(
                request,
                200,
                "<h2>Signed in</h2><p>You can close this window and return to Git Projects Manager.</p>",
            );
            return Ok((code, got_state));
        }
    });

    let (code, got_state) = task.await.context("loopback task panicked")??;
    if got_state != expected_state {
        return Err(anyhow!("OAuth state mismatch (possible CSRF)"));
    }

    let http = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()?;
    let resp = http
        .post(TOKEN_URL)
        .form(&[
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("redirect_uri", redirect_uri.as_str()),
            ("grant_type", "authorization_code"),
            ("code", code.as_str()),
            ("code_verifier", verifier.secret().as_str()),
        ])
        .send()
        .await
        .context("token exchange request failed")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(anyhow!("token exchange failed ({status}): {body}"));
    }

    let token_resp: TokenResponse = resp
        .json()
        .await
        .context("token endpoint returned unexpected body")?;
    Ok(token_resp.id_token)
}

fn respond_html(request: tiny_http::Request, status: u16, body_html: &str) {
    let html = format!(
        r#"<!doctype html><html><head><meta charset="utf-8"><title>Git Projects Manager</title><style>body{{font-family:-apple-system,BlinkMacSystemFont,sans-serif;max-width:480px;margin:80px auto;padding:0 24px;text-align:center;color:#222}}h2{{margin-top:0}}</style></head><body>{body_html}</body></html>"#
    );
    let header: tiny_http::Header = "Content-Type: text/html; charset=utf-8"
        .parse()
        .expect("static header parses");
    let response = tiny_http::Response::from_string(html)
        .with_status_code(status)
        .with_header(header);
    let _ = request.respond(response);
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
