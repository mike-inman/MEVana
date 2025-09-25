use std::collections::{HashMap, HashSet};
use std::env;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Context, Result};
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use parking_lot::RwLock;
use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;

#[derive(Clone, Debug)]
pub struct ClerkAuthenticator {
    client: Client,
    jwks_url: String,
    issuer: Option<String>,
    audiences: Vec<String>,
    allowed_roles: Option<HashSet<String>>,
    authorized_party: Option<String>,
    cache_ttl: Duration,
    cache: Arc<RwLock<Option<CachedKeys>>>,
}

#[derive(Clone, Debug)]
pub struct ClerkIdentity {
    pub subject: String,
    pub email: Option<String>,
    pub session_id: Option<String>,
    pub token_id: Option<String>,
    pub roles: Vec<String>,
    pub claims: Value,
}

#[derive(Clone)]
struct CachedKeys {
    fetched_at: Instant,
    keys: HashMap<String, Arc<DecodingKey>>,
}

impl std::fmt::Debug for CachedKeys {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CachedKeys")
            .field("fetched_at", &self.fetched_at)
            .field("key_count", &self.keys.len())
            .finish()
    }
}

#[derive(Debug, Deserialize)]
struct JsonWebKeySet {
    keys: Vec<JsonWebKey>,
}

#[derive(Debug, Deserialize)]
struct JsonWebKey {
    kid: Option<String>,
    kty: Option<String>,
    alg: Option<String>,
    n: Option<String>,
    e: Option<String>,
}

impl ClerkAuthenticator {
    pub fn from_env() -> Result<Option<Self>> {
        let jwks_url = match env::var("CLERK_JWKS_URL") {
            Ok(url) if !url.trim().is_empty() => url,
            _ => return Ok(None),
        };

        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .context("failed to build Clerk auth HTTP client")?;

        let issuer = env::var("CLERK_ISSUER")
            .ok()
            .filter(|v| !v.trim().is_empty());
        let audiences = env::var("CLERK_ALLOWED_AUDIENCES")
            .ok()
            .map(|v| split_env_list(&v))
            .unwrap_or_default();
        let allowed_roles = env::var("CLERK_ALLOWED_ROLES")
            .ok()
            .map(|v| split_env_list(&v))
            .filter(|roles| !roles.is_empty())
            .map(|roles| roles.into_iter().map(|r| r.to_ascii_lowercase()).collect());
        let authorized_party = env::var("CLERK_AUTHORIZED_PARTY")
            .ok()
            .filter(|v| !v.trim().is_empty());
        let cache_ttl = env::var("CLERK_JWKS_CACHE_SECS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .filter(|secs| *secs > 0)
            .map(Duration::from_secs)
            .unwrap_or_else(|| Duration::from_secs(300));

        Ok(Some(Self {
            client,
            jwks_url,
            issuer,
            audiences,
            allowed_roles,
            authorized_party,
            cache_ttl,
            cache: Arc::new(RwLock::new(None)),
        }))
    }

    pub async fn validate(&self, token: &str) -> Result<ClerkIdentity> {
        let header = decode_header(token).context("failed to decode JWT header")?;
        let kid = header
            .kid
            .ok_or_else(|| anyhow!("missing kid in Clerk JWT header"))?;

        let decoding_key = self.get_decoding_key(&kid).await?;

        let mut validation = Validation::new(Algorithm::RS256);
        validation.validate_exp = true;

        if let Some(issuer) = &self.issuer {
            validation.set_issuer(&[issuer.clone()]);
        }
        if !self.audiences.is_empty() {
            validation.set_audience(&self.audiences);
        }

        let token_data = decode::<Value>(token, &decoding_key, &validation)
            .context("failed to decode Clerk JWT claims")?;
        let claims = token_data.claims;

        if let Some(expected_azp) = &self.authorized_party {
            let azp = claims
                .get("azp")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            if !azp.eq_ignore_ascii_case(expected_azp) {
                anyhow::bail!("clerk token rejected: azp mismatch");
            }
        }

        let subject = claims
            .get("sub")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("missing sub in Clerk JWT claims"))?
            .to_string();
        let email = claims
            .get("email")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let session_id = claims
            .get("sid")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let token_id = claims
            .get("jti")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let roles = extract_roles(&claims);

        if let Some(allowed_roles) = &self.allowed_roles {
            let has_allowed_role = roles
                .iter()
                .map(|role| role.to_ascii_lowercase())
                .any(|role| allowed_roles.contains(&role));
            if !has_allowed_role {
                anyhow::bail!("clerk token rejected: required role missing");
            }
        }

        Ok(ClerkIdentity {
            subject,
            email,
            session_id,
            token_id,
            roles,
            claims,
        })
    }

    async fn get_decoding_key(&self, kid: &str) -> Result<Arc<DecodingKey>> {
        {
            let cache_guard = self.cache.read();
            if let Some(cache) = cache_guard.as_ref() {
                if !cache.is_expired(self.cache_ttl) {
                    if let Some(key) = cache.keys.get(kid) {
                        return Ok(Arc::clone(key));
                    }
                }
            }
        }

        let fetched = self.fetch_jwks().await?;
        let maybe_key = fetched.keys.get(kid).cloned();

        self.cache.write().replace(fetched);

        maybe_key.ok_or_else(|| anyhow!("no matching JWK found for kid={}", kid))
    }

    async fn fetch_jwks(&self) -> Result<CachedKeys> {
        let response = self
            .client
            .get(&self.jwks_url)
            .send()
            .await
            .context("failed to download Clerk JWKS")?;

        if !response.status().is_success() {
            anyhow::bail!(
                "failed to download Clerk JWKS: status {}",
                response.status()
            );
        }

        let jwks: JsonWebKeySet = response
            .json()
            .await
            .context("failed to parse Clerk JWKS response")?;

        let mut keys = HashMap::new();
        for jwk in jwks.keys {
            if jwk.kty.as_deref() != Some("RSA") {
                continue;
            }
            let kid = match jwk.kid {
                Some(k) => k,
                None => continue,
            };
            let n = match jwk.n {
                Some(v) => v,
                None => continue,
            };
            let e = match jwk.e {
                Some(v) => v,
                None => continue,
            };

            match DecodingKey::from_rsa_components(&n, &e) {
                Ok(key) => {
                    keys.insert(kid, Arc::new(key));
                }
                Err(err) => {
                    log::warn!("skipping Clerk JWK kid={} decode_error={}", kid, err);
                }
            }
        }

        if keys.is_empty() {
            anyhow::bail!("Clerk JWKS download produced no RSA keys");
        }

        Ok(CachedKeys {
            fetched_at: Instant::now(),
            keys,
        })
    }
}

impl CachedKeys {
    fn is_expired(&self, ttl: Duration) -> bool {
        self.fetched_at.elapsed() > ttl
    }
}

fn split_env_list(input: &str) -> Vec<String> {
    input
        .split(',')
        .map(|item| item.trim())
        .filter(|item| !item.is_empty())
        .map(|item| item.to_string())
        .collect()
}

fn extract_roles(claims: &Value) -> Vec<String> {
    let mut roles = Vec::new();

    if let Some(value) = claims.get("role") {
        if let Some(role) = value.as_str() {
            roles.push(role.to_string());
        }
    }

    if let Some(value) = claims.get("roles") {
        collect_strings(value, &mut roles);
    }

    if let Some(value) = claims.get("org_roles") {
        collect_strings(value, &mut roles);
    }

    if let Some(value) = claims.get("permissions") {
        collect_strings(value, &mut roles);
    }

    for path in [
        "/public_metadata/roles",
        "/publicMetadata/roles",
        "/private_metadata/roles",
        "/privateMetadata/roles",
        "/metadata/roles",
    ] {
        if let Some(value) = claims.pointer(path) {
            collect_strings(value, &mut roles);
        }
    }

    roles.sort();
    roles.dedup();
    roles
}

fn collect_strings(value: &Value, target: &mut Vec<String>) {
    match value {
        Value::String(s) => target.push(s.to_string()),
        Value::Array(items) => {
            for item in items {
                if let Some(s) = item.as_str() {
                    target.push(s.to_string());
                }
            }
        }
        _ => {}
    }
}
