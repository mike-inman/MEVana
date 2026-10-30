use anyhow::{bail, ensure, Context, Result};
use clap::{Parser, ValueEnum};
use std::{
    fmt,
    net::{IpAddr, SocketAddr},
    path::{Path, PathBuf},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    Sodae,
    Tatum,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum SnapshotRpc {
    Tatum,
}
impl Provider {
    pub fn host(self) -> &'static str {
        match self {
            Self::Sodae => "ams.tls.sodae.io",
            Self::Tatum => "solana-mainnet-grpc.gateway.tatum.io",
        }
    }
    pub fn key(self) -> &'static str {
        match self {
            Self::Sodae => "SODAE.IO_API_KEY",
            Self::Tatum => "TATUM_MAINNET",
        }
    }
    pub fn header(self) -> &'static str {
        match self {
            Self::Sodae => "x-token",
            Self::Tatum => "x-api-key",
        }
    }
    pub fn endpoint(self) -> String {
        format!("https://{}:443", self.host())
    }
}
pub struct Secret {
    provider: Provider,
    value: String,
}
impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[REDACTED]")
    }
}
impl Secret {
    pub fn expose(&self) -> &str {
        &self.value
    }
}
// Read only the selected approved key; never load other .env values.
pub fn credential(
    provider: Provider,
    path: &Path,
    env: impl Fn(&str) -> Option<String>,
) -> Result<Secret> {
    if let Some(v) = env(provider.key()) {
        ensure!(!v.is_empty(), "empty provider credential");
        return Ok(Secret { provider, value: v });
    }
    let text =
        std::fs::read_to_string(path).context("approved root credential file unavailable")?;
    for line in text.lines() {
        if let Some((key, value)) = line
            .trim()
            .strip_prefix("export ")
            .unwrap_or(line.trim())
            .split_once('=')
        {
            if key.trim() == provider.key() {
                let v = value.trim().trim_matches(['\'', '"']).to_owned();
                ensure!(!v.is_empty(), "empty provider credential");
                return Ok(Secret { provider, value: v });
            }
        }
    }
    bail!("selected provider credential absent")
}
pub fn validate_pair(
    provider: Provider,
    endpoint: &str,
    secret: &Secret,
    synthetic: bool,
) -> Result<url::Url> {
    ensure!(
        secret.provider == provider,
        "credential belongs to another provider"
    );
    validate_endpoint(provider, endpoint, synthetic)
}
pub fn validate_endpoint(provider: Provider, endpoint: &str, synthetic: bool) -> Result<url::Url> {
    let u = url::Url::parse(endpoint).map_err(|_| anyhow::anyhow!("invalid endpoint"))?;
    ensure!(
        u.username().is_empty()
            && u.password().is_none()
            && u.query().is_none()
            && u.fragment().is_none()
            && u.path() == "/",
        "endpoint must have no credentials, path or query"
    );
    ensure!(u.scheme() == "https", "TLS is required");
    let host = u.host_str().context("missing host")?;
    // Synthetic mode never authorizes forwarding real provider credentials to a mock.
    ensure!(
        host == provider.host() || (synthetic && is_local_host(host)),
        "provider/token endpoint mismatch"
    );
    Ok(u)
}
fn is_local_host(host: &str) -> bool {
    host == "localhost" || host.parse::<IpAddr>().is_ok_and(private_ip)
}
pub fn private_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v) => {
            v.is_private()
                || v.is_loopback()
                || v.is_link_local()
                || v.is_unspecified()
                || v.is_broadcast()
                || v.is_multicast()
                || v.octets()[0] == 0
        }
        IpAddr::V6(v) => {
            v.is_loopback()
                || v.is_unspecified()
                || v.is_unique_local()
                || v.is_unicast_link_local()
                || v.is_multicast()
                || v.to_ipv4_mapped()
                    .is_some_and(|v| private_ip(IpAddr::V4(v)))
        }
    }
}
#[derive(Parser, Debug, Clone)]
#[command(about = "Bounded, read-only public market-data capture")]
pub struct Args {
    #[arg(long, value_enum)]
    pub snapshot_rpc: Option<SnapshotRpc>,
    #[arg(long, value_enum, default_value = "sodae")]
    pub provider: Provider,
    #[arg(long)]
    pub endpoint: Option<String>,
    #[arg(long)]
    pub synthetic_source: bool,
    #[arg(long, default_value = "/Users/michaelinman/Repos/MEVana/.env")]
    pub credentials: PathBuf,
    #[arg(long)]
    pub output: Option<PathBuf>,
    #[arg(long)]
    pub allowlist: Option<PathBuf>,
    #[arg(long, default_value_t = 600)]
    pub duration: u64,
    #[arg(long, default_value_t = 3600)]
    pub rotation_seconds: u64,
    #[arg(long, default_value_t = 67108864)]
    pub rotation_bytes: u64,
    #[arg(long, default_value_t = 450000000)]
    pub max_bytes: u64,
    #[arg(long, default_value_t = 5)]
    pub max_reconnect_failures: u32,
    #[arg(long, default_value_t = 30)]
    pub stall_seconds: u64,
    #[arg(long, default_value = "127.0.0.1:8121")]
    pub metrics: SocketAddr,
    #[arg(long, default_value = "processed")]
    pub commitment: String,
    #[arg(long, default_value = "dev-test")]
    pub connection_tier: String,
    #[arg(long, default_value_t=true, action=clap::ArgAction::Set)]
    pub transactions: bool,
    #[arg(long)]
    pub filter_check: bool,
}
impl Args {
    pub fn allowlist(&self) -> Result<Vec<String>> {
        let keys: Vec<String> = match &self.allowlist {
            Some(p) => serde_json::from_slice(&std::fs::read(p)?)?,
            None => vec![],
        };
        ensure!(keys.len() <= 100, "allowlist exceeds bounded filter count");
        for k in &keys {
            ensure!(
                bs58::decode(k).into_vec().is_ok_and(|v| v.len() == 32),
                "invalid allowlist public key"
            );
        }
        Ok(keys)
    }
    pub fn commitment(&self) -> Result<i32> {
        match self.commitment.as_str() {
            "processed" => Ok(0),
            "confirmed" => Ok(1),
            "finalized" => Ok(2),
            _ => bail!("invalid commitment"),
        }
    }
}
