//! Version-two CLI artifacts; the v1 writer remains unchanged.
use super::{sha, utc, write, PolicyResult, ResultJson, REFERENCE_A};
use anyhow::{ensure, Context, Result};
use research_replay::{
    slot::{self, ClassifiedGap, CommitmentMode},
    v2_engine::{self, ConfigV2},
    SOL, STRATEGY_VERSION, USDC, USDT,
};
use serde::Serialize;
use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::{BufWriter, Write},
    path::Path,
};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RequestV2<'a> {
    canonicalization_version: u8,
    dataset_id: &'a str,
    strategy_version: &'static str,
    window_start_utc: &'a str,
    window_end_utc: &'a str,
    quote_token_id: &'static str,
    token_ids: Vec<&'static str>,
    pool_ids: &'a [String],
    route_shapes: [&'static str; 2],
    price_input_pool_ids: [&'static str; 1],
    min_trade_size_quote_atoms: u64,
    max_trade_size_quote_atoms: u64,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ManifestV2<'a> {
    format_version: u8,
    input_format_version: u32,
    source: &'static str,
    input_hashes: &'a BTreeMap<String, String>,
    config: &'a ConfigV2,
    crate_version: &'static str,
    git_commit: &'a str,
    toolchain: &'a str,
    first_slot: u64,
    last_slot: u64,
    classified_gaps: &'a [ClassifiedGap],
    dropped_fork_writes: &'a BTreeMap<u64, u64>,
    output_hashes: BTreeMap<String, String>,
}

pub(super) fn run(
    capture_root: &Path,
    output: &Path,
    profile: &str,
    git_commit: &str,
    toolchain: &str,
) -> Result<()> {
    let base_profile = profile.strip_prefix("v2-").context("invalid v2 profile")?;
    let base = super::config(base_profile)?;
    let config = ConfigV2 {
        base,
        compute_units_triangle: 600_000,
        reference_pool: REFERENCE_A.into(),
    };
    let capture =
        research_replay::load_capture(capture_root).context("capture validation failed")?;
    ensure!(
        capture.format_version == 2,
        "v2 profiles require format version 2"
    );
    let replay = v2_engine::replay(&capture, &config)?;
    let boundaries = slot::boundaries(&capture.slot_nodes, config.base.commitment_mode);
    let start = utc(boundaries
        .first()
        .context("no boundaries")?
        .event
        .receive_ns)?;
    let end = utc(boundaries.last().context("no boundaries")?.event.receive_ns)?;
    fs::create_dir_all(output)?;
    let mut output_hashes = BTreeMap::new();
    let mut candidate_file = BufWriter::new(File::create(output.join("candidates.jsonl"))?);
    for row in &replay.rows {
        serde_json::to_writer(&mut candidate_file, row)?;
        candidate_file.write_all(b"\n")?;
    }
    candidate_file.flush()?;
    drop(candidate_file);
    output_hashes.insert(
        "candidates.jsonl".into(),
        geyser_capture::format::hash_file(&output.join("candidates.jsonl"))?,
    );
    let summary_bytes = serde_json::to_vec(&serde_json::json!({
        "routeShapes": replay.summary.route_shapes,
        "slotBoundaries": replay.summary.slot_boundaries,
        "probeInputAtoms": replay.summary.probe_input_atoms,
        "policies": replay.summary.policies,
        "shapes": replay.summary.shapes,
        "poolDirections": replay.summary.pool_directions,
        "classifiedGaps": replay.classified_gaps,
        "droppedForkWrites": replay.dropped_fork_writes,
    }))?;
    output_hashes.insert(
        "summary.json".into(),
        write(&output.join("summary.json"), &summary_bytes)?,
    );
    let token_ids = {
        let mut ids = vec![SOL, USDC, USDT];
        ids.sort();
        ids
    };
    let request = RequestV2 {
        canonicalization_version: 2,
        dataset_id: "orca-wp-snapshot-20261030",
        strategy_version: STRATEGY_VERSION,
        window_start_utc: &start,
        window_end_utc: &end,
        quote_token_id: USDC,
        token_ids,
        pool_ids: &replay.pools,
        route_shapes: ["triangle", "two_leg"],
        price_input_pool_ids: [REFERENCE_A],
        min_trade_size_quote_atoms: config.base.sizes_quote_atoms[0],
        max_trade_size_quote_atoms: config.base.sizes_quote_atoms[2],
    };
    let request_bytes = serde_json::to_vec(&request)?;
    let request_hash = sha(&request_bytes);
    output_hashes.insert(
        "request.json".into(),
        write(&output.join("request.json"), &request_bytes)?,
    );
    let result = ResultJson {
        canonicalization_version: 1,
        result_id: format!(
            "replay-v2-{}-usdc-{}-l{}-tip{}",
            capture.last_slot,
            match config.base.commitment_mode {
                CommitmentMode::Confirmed => "confirmed",
                CommitmentMode::Processed => "processed",
            },
            config.base.latency_slots,
            config.base.tip_lamports
        ),
        request_sha256: request_hash,
        request_canonicalization_version: 2,
        dataset_id: "orca-wp-snapshot-20261030",
        dataset_metadata_version: "capture-v2",
        strategy_version: STRATEGY_VERSION,
        engine_version: "research-replay-v2-0_1_0",
        cost_model_version: config.cost_version(),
        created_at_utc: &end,
        window_start_utc: &start,
        window_end_utc: &end,
        evidence_kind: "Simulated",
        status: "Completed",
        error_code: None,
        policies: replay
            .summary
            .policies
            .iter()
            .map(PolicyResult::from)
            .collect(),
    };
    output_hashes.insert(
        "result.json".into(),
        write(&output.join("result.json"), &serde_json::to_vec(&result)?)?,
    );
    let manifest = ManifestV2 {
        format_version: 3,
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
