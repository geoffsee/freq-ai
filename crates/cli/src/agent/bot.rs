use crate::agent::cmd::log;
use crate::agent::config_store::{load_bot_private_key_pem, load_bot_token};
use crate::agent::types::{BotCredentials, BotSettings};
use std::env;
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::time::Instant;

static BOT_TOKEN_CACHE: Mutex<Option<(String, Instant)>> = Mutex::new(None);
const TOKEN_CACHE_SECS: u64 = 50 * 60;

fn bot_token_cache() -> &'static Mutex<Option<(String, Instant)>> {
    &BOT_TOKEN_CACHE
}

/// Load bot credentials from environment variables.
///
/// Resolution order:
/// 1. `DEV_BOT_TOKEN` — direct token (PAT or pre-minted installation token)
/// 2. `DEV_BOT_TOKEN_PATH` — path to a file containing the token
/// 3. `DEV_BOT_APP_ID` + `DEV_BOT_INSTALLATION_ID` + `DEV_BOT_PRIVATE_KEY` — GitHub App
pub fn load_bot_credentials_from_env() -> Option<BotCredentials> {
    // Direct token from env
    if let Ok(token) = env::var("DEV_BOT_TOKEN") {
        let token = token.trim().to_string();
        if !token.is_empty() {
            return Some(BotCredentials::Token(token));
        }
    }

    // Token from file
    if let Ok(path) = env::var("DEV_BOT_TOKEN_PATH")
        && let Ok(token) = std::fs::read_to_string(&path)
    {
        let token = token.trim().to_string();
        if !token.is_empty() {
            return Some(BotCredentials::Token(token));
        }
    }

    // GitHub App credentials
    let app_id = env::var("DEV_BOT_APP_ID").ok().filter(|s| !s.is_empty())?;
    let installation_id = env::var("DEV_BOT_INSTALLATION_ID")
        .ok()
        .filter(|s| !s.is_empty())?;
    let private_key_path = env::var("DEV_BOT_PRIVATE_KEY").unwrap_or_else(|_| {
        env::var("HOME")
            .map(|h| format!("{h}/.config/freq-ai/dev-ui-bot.pem"))
            .unwrap_or_else(|_| ".config/freq-ai/dev-ui-bot.pem".to_string())
    });
    let private_key_pem = std::fs::read_to_string(&private_key_path)
        .map_err(|e| {
            log(&format!(
                "Failed to read bot private key at {private_key_path}: {e}"
            ))
        })
        .ok()?;

    Some(BotCredentials::GitHubApp {
        app_id,
        installation_id,
        private_key_pem,
    })
}

pub fn load_bot_settings(root: &str, dev_cfg: &crate::agent::types::DevConfig) -> BotSettings {
    if let Some(creds) = load_bot_credentials_from_env() {
        return BotSettings::from_credentials(&creds);
    }

    let mut settings = dev_cfg.bot.clone().into_bot_settings();
    if let Some(token) = load_bot_token(root) {
        settings.token = token;
    }
    if let Some(private_key_pem) = load_bot_private_key_pem(root) {
        settings.private_key_pem = private_key_pem;
    }
    settings
}

/// Resolve bot credentials to a usable `GH_TOKEN` value.
pub fn resolve_bot_token(creds: &BotCredentials) -> Option<String> {
    match creds {
        BotCredentials::Token(t) => Some(t.clone()),
        BotCredentials::GitHubApp {
            app_id,
            installation_id,
            private_key_pem,
        } => {
            // Check cache
            if let Ok(cache) = bot_token_cache().lock()
                && let Some((ref token, ref created_at)) = *cache
                && created_at.elapsed() < std::time::Duration::from_secs(TOKEN_CACHE_SECS)
            {
                return Some(token.clone());
            }

            let token = mint_installation_token(app_id, installation_id, private_key_pem)?;

            if let Ok(mut cache) = bot_token_cache().lock() {
                *cache = Some((token.clone(), Instant::now()));
            }

            Some(token)
        }
    }
}

/// Mint a GitHub App installation token via JWT + REST API.
fn mint_installation_token(
    app_id: &str,
    installation_id: &str,
    private_key_pem: &str,
) -> Option<String> {
    use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};

    let key = EncodingKey::from_rsa_pem(private_key_pem.as_bytes())
        .map_err(|e| log(&format!("Invalid RSA PEM key: {e}")))
        .ok()?;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs();

    let claims = serde_json::json!({
        "iss": app_id,
        "iat": now.saturating_sub(60),
        "exp": now + 600,
    });

    let jwt = encode(&Header::new(Algorithm::RS256), &claims, &key)
        .map_err(|e| log(&format!("JWT signing failed: {e}")))
        .ok()?;

    let url = format!("https://api.github.com/app/installations/{installation_id}/access_tokens");

    // Pass the JWT-bearing Authorization header to curl via stdin (`--config -`)
    // instead of as a `-H` argv value, so the short-lived JWT is never visible
    // to other local users via `ps aux` / `/proc/<pid>/cmdline`.
    let mut child = Command::new("curl")
        .args([
            "--config",
            "-",
            "-s",
            "-X",
            "POST",
            "-H",
            "Accept: application/vnd.github+json",
            "-H",
            "X-GitHub-Api-Version: 2022-11-28",
            &url,
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| log(&format!("curl spawn failed: {e}")))
        .ok()?;

    {
        use std::io::Write;
        let mut stdin = child.stdin.take()?;
        let auth_config = format!("header = \"Authorization: Bearer {jwt}\"\n");
        stdin
            .write_all(auth_config.as_bytes())
            .map_err(|e| log(&format!("curl stdin write failed: {e}")))
            .ok()?;
    }

    let output = child
        .wait_with_output()
        .map_err(|e| log(&format!("curl failed: {e}")))
        .ok()?;

    if !output.status.success() {
        log("Failed to mint bot installation token (curl error)");
        return None;
    }

    let body: serde_json::Value = serde_json::from_slice(&output.stdout)
        .map_err(|e| log(&format!("Failed to parse installation token response: {e}")))
        .ok()?;

    let token = body.get("token").and_then(|t| t.as_str()).map(String::from);
    if token.is_none() {
        let msg = body
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("unknown error");
        log(&format!(
            "GitHub API error minting installation token: {msg}"
        ));
    }
    token
}
