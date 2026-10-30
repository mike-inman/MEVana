//! Replays one finalized capture to a new local output directory. No network access.
use anyhow::{bail, ensure, Context, Result};
use research_replay::{
    engine::{self, Config, PairSummary, PolicySummary},
    slot::{self, ClassifiedGap, CommitmentMode},
    DATASET_ID, STRATEGY_VERSION,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::{BufWriter, Write},
    path::PathBuf,
};

mod v2_main;

const REFERENCE_A: &str = "83v8iPyZihDEjDdY8RdZddyZNyUtXngz69Lgo9Kt5d6d";
const REFERENCE_B: &str = "FwewVm8u6tFPGewAyHmWAqad9hmF7mvqxK4mJ7iNqqGC";

fn utc(ns: u64) -> Result<String> {
    let instant = chrono::DateTime::<chrono::Utc>::from_timestamp_nanos(i64::try_from(ns)?);
    Ok(format!(
        "{}.{:07}Z",
        instant.format("%Y-%m-%dT%H:%M:%S"),
        instant.timestamp_subsec_nanos() / 100
    ))
}
fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn write(path: &std::path::Path, bytes: &[u8]) -> Result<String> {
    let mut file = File::create(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(sha(bytes))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Request<'a> {
    canonicalization_version: u8,
    dataset_id: &'a str,
    strategy_version: &'static str,
    window_start_utc: &'a str,
    window_end_utc: &'a str,
    token_pair: [&'static str; 2],
    pool_ids: &'a [String],
    min_trade_size_quote_atoms: u64,
    max_trade_size_quote_atoms: u64,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Rejection<'a> {
    reason_code: &'a str,
    count: u64,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PolicyResult<'a> {
    kind: &'a str,
    candidates_evaluated: u64,
    opportunities_detected: u64,
    selected_or_executed: u64,
    rejected_candidates: Vec<Rejection<'a>>,
    missed_candidates: Option<u64>,
    estimated_net_profit_quote_atoms: i64,
    simulated_net_profit_quote_atoms: i64,
    realized_net_profit_quote_atoms: Option<i64>,
}
impl<'a> From<&'a PolicySummary> for PolicyResult<'a> {
    fn from(p: &'a PolicySummary) -> Self {
        Self {
            kind: &p.kind,
            candidates_evaluated: p.candidates_evaluated,
            opportunities_detected: p.opportunities_detected,
            selected_or_executed: p.selected_or_executed,
            rejected_candidates: p
                .rejected_candidates
                .iter()
                .map(|(reason_code, count)| Rejection {
                    reason_code,
                    count: *count,
                })
                .collect(),
            missed_candidates: None,
            estimated_net_profit_quote_atoms: p.estimated_net_profit_quote_atoms,
            simulated_net_profit_quote_atoms: p.simulated_net_profit_quote_atoms,
            realized_net_profit_quote_atoms: None,
        }
    }
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ResultJson<'a> {
    canonicalization_version: u8,
    result_id: String,
    request_sha256: String,
    request_canonicalization_version: u8,
    dataset_id: &'a str,
    dataset_metadata_version: &'a str,
    strategy_version: &'static str,
    engine_version: &'static str,
    cost_model_version: String,
    created_at_utc: &'a str,
    window_start_utc: &'a str,
    window_end_utc: &'a str,
    evidence_kind: &'static str,
    status: &'static str,
    error_code: Option<&'static str>,
    policies: Vec<PolicyResult<'a>>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RunManifest<'a> {
    format_version: u8,
    input_format_version: u32,
    source: &'static str,
    input_hashes: &'a BTreeMap<String, String>,
    config: &'a Config,
    crate_version: &'static str,
    git_commit: &'a str,
    toolchain: &'a str,
    first_slot: u64,
    last_slot: u64,
    classified_gaps: &'a [ClassifiedGap],
    dropped_fork_writes: &'a BTreeMap<u64, u64>,
    output_hashes: BTreeMap<String, String>,
}
struct PairContext<'a> {
    start: &'a str,
    end: &'a str,
    dataset_id: &'a str,
    metadata_version: &'a str,
    last_slot: u64,
}

fn config(profile: &str) -> Result<Config> {
    let (latency_slots, tip_lamports, commitment_mode) = match profile {
        "default" => (1, 10_000, CommitmentMode::Confirmed),
        "latency2" => (2, 10_000, CommitmentMode::Confirmed),
        "cost" => (1, 100_000, CommitmentMode::Confirmed),
        "processed" => (1, 10_000, CommitmentMode::Processed),
        _ => bail!("unknown profile"),
    };
    Ok(Config {
        commitment_mode,
        sizes_quote_atoms: [10_000_000, 100_000_000, 1_000_000_000],
        base_fee_lamports: 5_000,
        priority_micro_lamports_per_cu: 1_000,
        compute_units_two_leg: 400_000,
        tip_lamports,
        latency_slots,
        simple_min_net_quote_atoms: 1_000,
        ranking_top_k: 1,
        seed: 0,
        reference_a: REFERENCE_A.into(),
        reference_b: REFERENCE_B.into(),
        reference_c: REFERENCE_B.into(),
    })
}
fn write_pair(
    out: &std::path::Path,
    summary: &PairSummary,
    config: &Config,
    context: &PairContext<'_>,
    output_hashes: &mut BTreeMap<String, String>,
) -> Result<()> {
    let pair = summary.pair;
    let prefix = pair.label().to_ascii_lowercase();
    let (base, quote) = pair.tokens();
    let request = Request {
        canonicalization_version: 1,
        dataset_id: context.dataset_id,
        strategy_version: STRATEGY_VERSION,
        window_start_utc: context.start,
        window_end_utc: context.end,
        token_pair: [base, quote],
        pool_ids: &summary.pools,
        min_trade_size_quote_atoms: config.sizes_quote_atoms[0],
        max_trade_size_quote_atoms: config.sizes_quote_atoms[2],
    };
    let request_bytes = serde_json::to_vec(&request)?;
    let request_hash = sha(&request_bytes);
    let request_name = format!("{prefix}-request.json");
    output_hashes.insert(
        request_name.clone(),
        write(&out.join(request_name), &request_bytes)?,
    );
    let result = ResultJson {
        canonicalization_version: 1,
        result_id: format!(
            "replay-v2-{}-{}-{}-l{}-tip{}",
            context.last_slot,
            prefix,
            match config.commitment_mode {
                CommitmentMode::Confirmed => "confirmed",
                CommitmentMode::Processed => "processed",
            },
            config.latency_slots,
            config.tip_lamports
        ),
        request_sha256: request_hash,
        request_canonicalization_version: 1,
        dataset_id: context.dataset_id,
        dataset_metadata_version: context.metadata_version,
        strategy_version: STRATEGY_VERSION,
        engine_version: "research-replay-0_2_0",
        cost_model_version: config.cost_version(pair),
        created_at_utc: context.end,
        window_start_utc: context.start,
        window_end_utc: context.end,
        evidence_kind: "Simulated",
        status: "Completed",
        error_code: None,
        policies: summary.policies.iter().map(PolicyResult::from).collect(),
    };
    let result_bytes = serde_json::to_vec(&result)?;
    let result_name = format!("{prefix}-result.json");
    output_hashes.insert(
        result_name.clone(),
        write(&out.join(result_name), &result_bytes)?,
    );
    Ok(())
}
fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    ensure!(args.len() == 7, "usage: research-replay CAPTURE_SESSION OUTPUT_DIR default|latency2|cost|processed GIT_COMMIT TOOLCHAIN RUN_ID");
    let capture_root = PathBuf::from(&args[1]);
    let output = PathBuf::from(&args[2]);
    let profile = &args[3];
    let git_commit = &args[4];
    let toolchain = &args[5];
    let run_id = &args[6];
    ensure!(
        run_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-'),
        "invalid run id"
    );
    ensure!(!output.exists(), "output exists; refusing to overwrite");
    if profile.starts_with("v2-") {
        return v2_main::run(&capture_root, &output, profile, git_commit, toolchain);
    }
    let config = config(profile)?;
    let capture =
        research_replay::load_capture(&capture_root).context("capture validation failed")?;
    let pairs = if profile == "default" {
        vec![
            research_replay::Pair::A,
            research_replay::Pair::B,
            research_replay::Pair::C,
        ]
    } else {
        vec![research_replay::Pair::A]
    };
    let replay = engine::replay(&capture, &config, &pairs)?;
    fs::create_dir_all(&output)?;
    let mut output_hashes = BTreeMap::new();
    let candidates_name = "candidates.jsonl";
    let mut candidate_file = BufWriter::new(File::create(output.join(candidates_name))?);
    for row in &replay.rows {
        serde_json::to_writer(&mut candidate_file, row)?;
        candidate_file.write_all(b"\n")?;
    }
    candidate_file.flush()?;
    drop(candidate_file);
    output_hashes.insert(
        candidates_name.into(),
        geyser_capture::format::hash_file(&output.join(candidates_name))?,
    );
    let summary_bytes = serde_json::to_vec(&serde_json::json!({
        "pairs": replay.summaries,
        "classifiedGaps": replay.classified_gaps,
        "droppedForkWrites": replay.dropped_fork_writes,
    }))?;
    output_hashes.insert(
        "summary.json".into(),
        write(&output.join("summary.json"), &summary_bytes)?,
    );
    let boundaries = slot::boundaries(&capture.slot_nodes, config.commitment_mode);
    let first = boundaries.first().context("no replay boundaries")?;
    let last = boundaries.last().context("no replay boundaries")?;
    let start = utc(first.event.receive_ns)?;
    let end = utc(last.event.receive_ns)?;
    let (dataset_id, metadata_version) = if capture.format_version == 2 {
        ("orca-wp-snapshot-20261030", "capture-v2")
    } else {
        (DATASET_ID, "capture-v1")
    };
    let pair_context = PairContext {
        start: &start,
        end: &end,
        dataset_id,
        metadata_version,
        last_slot: capture.last_slot,
    };
    for summary in &replay.summaries {
        write_pair(&output, summary, &config, &pair_context, &mut output_hashes)?;
    }
    let manifest = RunManifest {
        format_version: 2,
        input_format_version: capture.format_version,
        source: "Simulated public dev-test capture",
        input_hashes: &capture.hashes,
        config: &config,
        crate_version: env!("CARGO_PKG_VERSION"),
        git_commit,
        toolchain,
        first_slot: capture.first_slot,
        last_slot: capture.last_slot,
        classified_gaps: &replay.classified_gaps,
        dropped_fork_writes: &replay.dropped_fork_writes,
        output_hashes,
    };
    write(
        &output.join("run-manifest.json"),
        &serde_json::to_vec(&manifest)?,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn canonical_utc_uses_seven_fractional_digits() {
        assert_eq!(
            utc(1_793_330_491_340_529_000).unwrap(),
            "2026-10-30T03:21:31.3405290Z"
        );
    }
}
