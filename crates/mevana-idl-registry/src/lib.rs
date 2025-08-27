use serde::Deserialize;
use std::{
    env, fs, io,
    path::{Path, PathBuf},
    process::Command,
};

/// Configure one DEX program to fetch/build an IDL for.
#[derive(Debug, Deserialize, Clone)]
pub struct Dex {
    /// Human-friendly name (used for logs only)
    pub name: String,
    /// Solana mainnet program ID (Base58)
    pub program_id: String,
    /// Optional Anchor repo to build if on-chain IDL is missing
    pub anchor_repo: Option<String>,
    /// Expected IDL file names inside `<repo>/target/idl/`
    #[serde(default)]
    pub expected_idl_names: Vec<String>,
    /// Optional fallback of (git repo url, path within repo, output file name)
    pub fallback_idl_git: Option<(String, String, String)>,
}

/// Default set: Saber pre-filled. Lifinity/Invariant are present but disabled until you
/// add confirmed program IDs (or provide them via config file / env).
fn default_dexes() -> Vec<Dex> {
    vec![
        // === Successfully Integrated DEXs ===
        Dex {
            name: "jupiter_idl".into(),
            program_id: "JUP4Fb2cqiRUcaTHdrPC8h2gNsA2ETXiPDD33WcGuJB".into(),
            anchor_repo: None, // Jupiter aggregator v4
            expected_idl_names: vec![
                "jupiter_aggregator_v4.json".into(),
                "jupiter_idl.json".into(),
                "jupiter.json".into(),
            ],
            fallback_idl_git: None,
        },
        // Jupiter Aggregator v6 (Swap Router)
        Dex {
            name: "jupiter_aggregator_v6".into(),
            program_id: "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4".into(),
            anchor_repo: None,
            expected_idl_names: vec!["jupiter_aggregator_v6.json".into()],
            fallback_idl_git: None,
        },
        // Jupiter Limit Order Program v1 (jupo...)
        Dex {
            name: "jupiter_limit_order".into(),
            program_id: "jupoNjAxXgZ4rjzxzPMP4oxduvQsQtZzyknqvzYNrNu".into(),
            anchor_repo: None,
            expected_idl_names: vec!["jupiter_limit_order.json".into()],
            fallback_idl_git: None,
        },
        // Jupiter Limit Order Program v2 (j1o2...)
        Dex {
            name: "jupiter_limit_order_v2".into(),
            program_id: "j1o2qRpjcyUwEvwtcfhEQefh773ZgjxcVRry7LDqg5X".into(),
            anchor_repo: None,
            expected_idl_names: vec!["jupiter_limit_order_v2.json".into()],
            fallback_idl_git: None,
        },
        Dex {
            name: "phoenix".into(),
            program_id: "PhoeNiXZ8ByJGLkxNfZRnkUfjvmuYqLR89jjFHGqdXY".into(),
            anchor_repo: Some("https://github.com/Ellipsis-Labs/phoenix-v1".into()),
            expected_idl_names: vec!["phoenix.json".into(), "phoenix_v1.json".into()],
            fallback_idl_git: None,
        },
        Dex {
            name: "whirlpool_idl".into(),
            program_id: "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc".into(),
            anchor_repo: Some("https://github.com/orca-so/whirlpools".into()),
            expected_idl_names: vec!["whirlpool.json".into(), "whirlpool_idl.json".into()],
            fallback_idl_git: None,
        },
        Dex {
            name: "meteora_damm".into(),
            program_id: "cpamdpZCGKUy5JxQXB4dcpGPiikHawvSWAd6mEn1sGG".into(),
            anchor_repo: Some("https://github.com/MeteoraAg/dlmm-sdk".into()),
            expected_idl_names: vec![
                "meteora.json".into(),
                "meteora_damm.json".into(),
                "cp_amm.json".into(),
            ],
            fallback_idl_git: Some((
                "https://github.com/bitquery/solana-idl-lib".into(),
                "meteora/cp_amm_011.json".into(),
                "meteora_damm.json".into(),
            )),
        },
        Dex {
            name: "raydium_amm".into(),
            program_id: "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8".into(),
            anchor_repo: Some("https://github.com/raydium-io/raydium-amm".into()),
            expected_idl_names: vec!["raydium_amm.json".into(), "amm.json".into()],
            fallback_idl_git: Some((
                "https://github.com/bitquery/solana-idl-lib".into(),
                "raydium/raydium_amm.json".into(),
                "raydium_amm.json".into(),
            )),
        },
        Dex {
            name: "raydium_clmm".into(),
            program_id: "5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1".into(),
            anchor_repo: Some("https://github.com/raydium-io/raydium-clmm".into()),
            expected_idl_names: vec!["raydium_clmm.json".into(), "amm_v3.json".into()],
            fallback_idl_git: Some((
                "https://github.com/bitquery/solana-idl-lib".into(),
                "raydium/amm_v3.json".into(),
                "raydium_clmm.json".into(),
            )),
        },
        Dex {
            name: "pump".into(),
            program_id: "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P".into(),
            anchor_repo: None, // Pump.fun bonding curve
            expected_idl_names: vec!["pump.json".into(), "pump_0.1.0.json".into()],
            fallback_idl_git: Some((
                "https://github.com/bitquery/solana-idl-lib".into(),
                "pumpfun/pump.json".into(),
                "pump.json".into(),
            )),
        },
        Dex {
            name: "pump_amm".into(),
            program_id: "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA".into(),
            anchor_repo: None, // PumpSwap AMM
            expected_idl_names: vec!["pump_amm.json".into(), "pump_amm_0.1.0.json".into()],
            fallback_idl_git: None,
        },
        Dex {
            name: "openbook".into(),
            program_id: "opnb2LAfJYbRMAHHvqjCwQxanZn7ReEHp1k81EohpZb".into(),
            anchor_repo: Some("https://github.com/openbook-dex/openbook-v2".into()),
            expected_idl_names: vec!["openbook.json".into(), "openbook_v2.json".into()],
            fallback_idl_git: None,
        },
        Dex {
            name: "goosefx".into(),
            program_id: "GFXsSL5sSaDfNFQUYsHekbWBW1TsFdjDYzACh62tEHxn".into(),
            anchor_repo: Some("https://github.com/GooseFX1/gfx-ssl-v2-sdk".into()),
            expected_idl_names: vec!["goosefx.json".into(), "gfx_ssl_v2.json".into()],
            fallback_idl_git: None,
        },
        // === Legacy/Additional DEXs ===
        Dex {
            name: "saber_stable_swap".into(),
            // Canonical Saber StableSwap program ID:
            program_id: "SSwpkEEcbUqx4vtoEByFjSkhKdCT862DNVb52nZg1UZ".into(),
            anchor_repo: Some("https://github.com/saber-hq/stable-swap".into()),
            expected_idl_names: vec!["stable_swap.json".into(), "saber_stable_swap.json".into()],
            fallback_idl_git: None,
        },
        // Lifinity v2 — verified mainnet program ID from Medium post
        Dex {
            name: "lifinity_v2".into(),
            program_id: "2wT8Yq49kHgDzXuPxZSaeLaH1qbmGXtEyPy64bL7aD3c".into(), // Lifinity v2 mainnet
            anchor_repo: None, // no public Anchor workspace known
            expected_idl_names: vec!["lifinity.json".into(), "lifinity_v2.json".into()],
            fallback_idl_git: Some((
                "https://github.com/bitquery/solana-idl-lib".into(),
                "lifinity/idl-0.1.1.json".into(), // correct repo path
                "lifinity_v2.json".into(),        // output filename
            )),
        },
        // Invariant — verified mainnet program ID from docs.invariant.app
        Dex {
            name: "invariant".into(),
            program_id: "48XDC18nH5FLq8kKfE6MJK2hcPFD7xsJQc4dSAgQWNAi".into(), // Invariant mainnet
            anchor_repo: Some("https://github.com/invariant-labs/protocol".into()),
            expected_idl_names: vec![
                "invariant.json".into(),
                "invariant_protocol.json".into(),
                "protocol.json".into(),
            ],
            fallback_idl_git: None, // Invariant not available in Bitquery repo
        },
    ]
}

/// Public entrypoint you’ll call from other crates’ build.rs
pub fn sync_idls() {
    let registry_dir = resolve_registry_dir();
    ensure_dir(&registry_dir);

    // You can also support external config: set MEVANA_IDL_CONFIG to a JSON file with Dex[]
    // If not provided, we use the default_dexes().
    let dexes = match env::var("MEVANA_IDL_CONFIG") {
        Ok(cfg) => load_config(Path::new(&cfg)).unwrap_or_else(|e| {
            println!("cargo:warning=Failed to read MEVANA_IDL_CONFIG={cfg}: {e}. Falling back to defaults.");
            default_dexes()
        }),
        Err(_) => default_dexes(),
    };

    ensure_anchor_env_default();

    for d in dexes {
        if d.program_id.starts_with("PUT_") {
            println!(
                "cargo:warning=Skipping {} (program ID placeholder). Fill it in to enable.",
                d.name
            );
            continue;
        }
        let out_file = registry_dir.join(format!("{}.json", d.name));
        match fetch_on_chain(&d.program_id, &out_file) {
            Ok(true) => {
                println!(
                    "cargo:warning=IDL (on-chain) → {} → {}",
                    d.name,
                    out_file.display()
                );
                continue;
            }
            Ok(false) | Err(_) => {
                println!(
                    "cargo:warning=No on-chain IDL for {}. Trying fallbacks…",
                    d.name
                );
            }
        }

        // Anchor workspace fallback
        if let Some(repo) = &d.anchor_repo {
            match build_from_anchor_repo(repo, &out_file, &d.expected_idl_names) {
                Ok(()) => {
                    println!(
                        "cargo:warning=IDL (built) → {} → {}",
                        d.name,
                        out_file.display()
                    );
                    continue;
                }
                Err(e) => println!("cargo:warning=Anchor fallback failed for {}: {e}", d.name),
            }
        }

        // Git repo with prebuilt IDL fallback
        if let Some((repo_url, idl_rel_path, out_name)) = &d.fallback_idl_git {
            if let Err(e) = copy_from_idl_repo(repo_url, idl_rel_path, &registry_dir.join(out_name))
            {
                println!("cargo:warning=IDL fallback repo failed for {}: {e}", d.name);
            } else {
                println!(
                    "cargo:warning=IDL (fallback repo) → {} → {}",
                    d.name, out_name
                );
                continue;
            }
        }

        println!("cargo:warning=❌ Could not obtain IDL for {}", d.name);
    }
}

fn resolve_registry_dir() -> PathBuf {
    if let Ok(custom) = env::var("IDL_REGISTRY_DIR") {
        return PathBuf::from(custom);
    }
    // default to <workspace-root>/idls (shared IDLs directory)
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    find_workspace_root(&crate_dir)
        .unwrap_or(crate_dir)
        .join("idls")
}

fn find_workspace_root(mut dir: &Path) -> Option<PathBuf> {
    // Walk up until we find a Cargo.toml that contains a [workspace] section.
    loop {
        let candidate = dir.join("Cargo.toml");
        if candidate.exists() {
            if let Ok(content) = fs::read_to_string(&candidate) {
                if content.contains("[workspace]") {
                    return Some(dir.to_path_buf());
                }
            }
        }
        match dir.parent() {
            Some(p) => dir = p,
            None => return None,
        }
    }
}

fn ensure_dir(path: &Path) {
    if let Err(e) = fs::create_dir_all(path) {
        panic!("cannot create {}: {e}", path.display());
    }
}

fn ensure_anchor_env_default() {
    if env::var("ANCHOR_PROVIDER_URL").is_err() {
        println!("cargo:warning=ANCHOR_PROVIDER_URL not set; defaulting to https://api.mainnet-beta.solana.com");
        env::set_var("ANCHOR_PROVIDER_URL", "https://api.mainnet-beta.solana.com");
    }
}

fn fetch_on_chain(program_id: &str, out_file: &Path) -> io::Result<bool> {
    if which::which("anchor").is_err() {
        println!("cargo:warning=anchor CLI not found on PATH; skipping on-chain fetch");
        return Ok(false);
    }
    let status = Command::new("anchor")
        .args(["idl", "fetch", program_id, "-o"])
        .arg(out_file)
        .status()?;
    Ok(status.success())
}

fn build_from_anchor_repo(
    repo_url: &str,
    out_file: &Path,
    expected_names: &[String],
) -> io::Result<()> {
    if which::which("git").is_err() || which::which("anchor").is_err() {
        return Err(io::Error::new(io::ErrorKind::Other, "git/anchor missing"));
    }
    let repo_name = repo_url.split('/').last().unwrap_or("repo");
    let repo_dir = Path::new("target").join(format!("idl-src-{}", repo_name));

    if !repo_dir.exists() {
        let st = Command::new("git")
            .args(["clone", "--depth=1", repo_url, repo_dir.to_str().unwrap()])
            .status()?;
        if !st.success() {
            return Err(io::Error::new(io::ErrorKind::Other, "git clone failed"));
        }
    }

    let st = Command::new("anchor")
        .current_dir(&repo_dir)
        .arg("build")
        .status()?;
    if !st.success() {
        return Err(io::Error::new(io::ErrorKind::Other, "anchor build failed"));
    }

    // Try known names first
    for name in expected_names {
        let candidate = repo_dir.join("target/idl").join(name);
        if candidate.exists() {
            fs::copy(&candidate, out_file)?;
            return Ok(());
        }
    }

    // Or pick any single JSON in target/idl
    let idl_dir = repo_dir.join("target/idl");
    if idl_dir.exists() {
        for entry in fs::read_dir(idl_dir)? {
            let p = entry?.path();
            if p.extension().and_then(|s| s.to_str()) == Some("json") {
                fs::copy(&p, out_file)?;
                return Ok(());
            }
        }
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "no IDL found after build",
    ))
}

fn copy_from_idl_repo(repo_url: &str, rel_path: &str, dst: &Path) -> io::Result<()> {
    if which::which("git").is_err() {
        return Err(io::Error::new(io::ErrorKind::Other, "git missing"));
    }
    let repo_name = repo_url.split('/').last().unwrap_or("repo");
    let repo_dir = Path::new("target").join(format!("idl-fallback-{}", repo_name));
    if !repo_dir.exists() {
        let st = Command::new("git")
            .args(["clone", "--depth=1", repo_url, repo_dir.to_str().unwrap()])
            .status()?;
        if !st.success() {
            return Err(io::Error::new(io::ErrorKind::Other, "git clone failed"));
        }
    }
    let src = repo_dir.join(rel_path);
    fs::copy(&src, dst)?;
    Ok(())
}

fn load_config(config_path: &Path) -> io::Result<Vec<Dex>> {
    let content = fs::read_to_string(config_path)?;
    let dexes: Vec<Dex> = serde_json::from_str(&content)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    Ok(dexes)
}
