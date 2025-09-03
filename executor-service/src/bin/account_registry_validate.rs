use anyhow::{anyhow, Result};
use executor_service::account_registry::{AccountRegistry, AccountRegistryConfig};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::path::PathBuf;

// Simple CLI to validate one or more account registry JSON files before promotion.
// Usage:
//   cargo run --bin account-registry-validate -- path/to/registry1.json path/to/registry2.json --strict
// Exit code non-zero if any file fails strict validation.

fn main() -> Result<()> {
    env_logger::init();
    let mut args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() || args.iter().any(|a| a == "-h" || a == "--help") {
        eprintln!("Usage: account-registry-validate [--strict] <file1.json> <file2.json> ...");
        std::process::exit(1);
    }
    let diff_flag = if let Some(pos) = args.iter().position(|a| a == "--diff") {
        args.remove(pos);
        true
    } else {
        false
    };
    // Optional: --patch-json <file> (emit structured diff JSON for sequential comparisons)
    let mut patch_json_path: Option<String> = None;
    if let Some(pos) = args.iter().position(|a| a == "--patch-json") {
        args.remove(pos);
        if pos < args.len() {
            // next arg should be path
            if pos < args.len() {
                patch_json_path = Some(args.remove(pos).to_string());
            }
        }
    } else if let Some(idx) = args.iter().position(|a| a.starts_with("--patch-json=")) {
        if let Some(rest) = args[idx].strip_prefix("--patch-json=") {
            patch_json_path = Some(rest.to_string());
        }
        args.remove(idx);
    }
    let strict_flag = if let Some(pos) = args.iter().position(|a| a == "--strict") {
        args.remove(pos);
        true
    } else {
        false
    };
    // Optional: --allowlist-file <path> for decimal decrease allowlist (one mint per line)
    let mut allowlist_file: Option<String> = None;
    if let Some(pos) = args.iter().position(|a| a == "--allowlist-file") {
        args.remove(pos);
        if pos < args.len() {
            allowlist_file = Some(args.remove(pos));
        }
    } else if let Some(idx) = args.iter().position(|a| a.starts_with("--allowlist-file=")) {
        if let Some(rest) = args[idx].strip_prefix("--allowlist-file=") {
            allowlist_file = Some(rest.to_string());
        }
        args.remove(idx);
    }
    if args.is_empty() {
        return Err(anyhow!("No registry files provided"));
    }
    let mut any_fail = false;
    let mut loaded: Vec<(String, AccountRegistry)> = Vec::new();
    for file in &args {
        let path = PathBuf::from(file);
        let cfg = AccountRegistryConfig {
            strict: strict_flag,
            path: Some(path.to_string_lossy().into()),
        };
        match AccountRegistry::load(&cfg) {
            Ok(reg) => {
                // We'll print summary
                println!(
                    "VALID {} missing={} validation_errors={} mint_decimals={}",
                    file,
                    reg.missing_keys.len(),
                    reg.validation_errors.len(),
                    reg.mint_decimals.len()
                );
                if strict_flag
                    && (!reg.missing_keys.is_empty() || !reg.validation_errors.is_empty())
                {
                    any_fail = true;
                }
                loaded.push((file.clone(), reg));
            }
            Err(e) => {
                eprintln!("INVALID {} err={}", file, e);
                any_fail = true;
            }
        }
    }
    #[derive(Serialize)]
    struct DecChange {
        mint: String,
        from: u8,
        to: u8,
    }
    #[derive(Serialize)]
    struct DiffEntry {
        from: String,
        to: String,
        added_keys: Vec<String>,
        removed_keys: Vec<String>,
        decimal_changes: Vec<DecChange>,
    }
    let mut diff_entries: Vec<DiffEntry> = Vec::new();
    if diff_flag && loaded.len() > 1 {
        println!("--- DIFF REPORT (sequential comparisons) ---");
        for w in loaded.windows(2) {
            let (prev_name, prev) = &w[0];
            let (next_name, next) = &w[1];
            println!("Comparing {} -> {}", prev_name, next_name);
            let mut prev_keys: BTreeSet<String> = BTreeSet::new();
            let mut next_keys: BTreeSet<String> = BTreeSet::new();
            let mut prev_decimals: BTreeMap<String, u8> = BTreeMap::new();
            let mut next_decimals: BTreeMap<String, u8> = BTreeMap::new();
            for (prov, mints) in &prev.data.flash_loans {
                for (mint, r) in mints {
                    prev_keys.insert(format!("flash.{prov}.{mint}"));
                    prev_decimals.insert(r.mint.clone(), r.decimals);
                }
            }
            for (prov, mints) in &next.data.flash_loans {
                for (mint, r) in mints {
                    next_keys.insert(format!("flash.{prov}.{mint}"));
                    next_decimals.insert(r.mint.clone(), r.decimals);
                }
            }
            for k in prev.data.clmm_pools.keys() {
                prev_keys.insert(format!("clmm.{k}"));
            }
            for k in next.data.clmm_pools.keys() {
                next_keys.insert(format!("clmm.{k}"));
            }
            for k in prev.data.orderbooks.keys() {
                prev_keys.insert(format!("orderbook.{k}"));
            }
            for k in next.data.orderbooks.keys() {
                next_keys.insert(format!("orderbook.{k}"));
            }
            for k in prev.data.oracles.keys() {
                prev_keys.insert(format!("oracle.{k}"));
            }
            for k in next.data.oracles.keys() {
                next_keys.insert(format!("oracle.{k}"));
            }
            if prev.data.jito.tip_account.is_some() {
                prev_keys.insert("jito.tip_account".into());
            }
            if next.data.jito.tip_account.is_some() {
                next_keys.insert("jito.tip_account".into());
            }
            let added: Vec<String> = next_keys.difference(&prev_keys).cloned().collect();
            let removed: Vec<String> = prev_keys.difference(&next_keys).cloned().collect();
            if added.is_empty() && removed.is_empty() {
                println!("  Keys: no additions or removals");
            } else {
                if !added.is_empty() {
                    println!("  Added ({}): {:?}", added.len(), added);
                }
                if !removed.is_empty() {
                    println!("  Removed ({}): {:?}", removed.len(), removed);
                }
            }
            let mut dec_changes: Vec<DecChange> = Vec::new();
            for (mint, prev_dec) in prev_decimals.iter() {
                if let Some(next_dec) = next_decimals.get(mint) {
                    if next_dec != prev_dec {
                        dec_changes.push(DecChange {
                            mint: mint.clone(),
                            from: *prev_dec,
                            to: *next_dec,
                        });
                    }
                }
            }
            if dec_changes.is_empty() {
                println!("  Decimals: no changes");
            } else {
                println!("  Decimals changed ({}):", dec_changes.len());
                for ch in &dec_changes {
                    println!("    {}: {} -> {}", ch.mint, ch.from, ch.to);
                }
            }
            diff_entries.push(DiffEntry {
                from: prev_name.clone(),
                to: next_name.clone(),
                added_keys: added,
                removed_keys: removed,
                decimal_changes: dec_changes,
            });
        }
    }
    // Gating: fail if any decimal decreases unless allowlisted.
    // Allowlist sources:
    //  1. File passed via --allowlist-file (one mint per line, # comments allowed)
    //  2. Env var ACCOUNT_REGISTRY_DECIMAL_DECREASE_ALLOW="MINT1,MINT2"
    if !diff_entries.is_empty() {
        let mut allow: BTreeSet<String> = BTreeSet::new();
        if let Some(path) = allowlist_file.as_ref() {
            match std::fs::read_to_string(path) {
                Ok(contents) => {
                    for line in contents.lines() {
                        let t = line.trim();
                        if t.is_empty() || t.starts_with('#') {
                            continue;
                        }
                        allow.insert(t.to_string());
                    }
                }
                Err(e) => eprintln!("WARN: could not read allowlist file {} err={}", path, e),
            }
        }
        let allow_env =
            std::env::var("ACCOUNT_REGISTRY_DECIMAL_DECREASE_ALLOW").unwrap_or_default();
        for mint in allow_env.split(',').filter(|s| !s.trim().is_empty()) {
            allow.insert(mint.trim().to_string());
        }
        let mut violations: Vec<String> = Vec::new();
        for entry in &diff_entries {
            for ch in &entry.decimal_changes {
                if ch.to < ch.from && !allow.contains(&ch.mint) {
                    violations.push(format!("{}:{} {}->{}", ch.mint, entry.to, ch.from, ch.to));
                }
            }
        }
        if !violations.is_empty() {
            eprintln!("ERROR: Decimal decreases detected (not allowlisted):");
            for v in &violations {
                eprintln!("  {}", v);
            }
            any_fail = true; // propagate non-zero exit
        }
    }
    if let Some(path) = patch_json_path {
        if diff_entries.is_empty() {
            println!(
                "No diff entries to export (need >=2 files and --diff). Skipping patch write."
            );
        } else {
            let json = serde_json::to_string_pretty(&diff_entries)?;
            if path == "-" {
                println!("{}", json);
            } else {
                std::fs::write(&path, json)?;
                println!("Wrote diff JSON patch to {}", path);
            }
        }
    }
    if any_fail {
        std::process::exit(2);
    }
    Ok(())
}
