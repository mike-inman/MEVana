//! 2026-11-01: Real public pool/config fixtures with Synthetic mint metadata.
use base64::Engine;
use serde_json::json;
#[test]
fn adapter_distinguishes_config_and_decodes_verified_pool() {
    let dir = tempfile::tempdir().unwrap();
    let encode = |bytes: &[u8]| base64::engine::general_purpose::STANDARD.encode(bytes);
    let mut sol = vec![0; 82];
    sol[44] = 9;
    let mut usdc = vec![0; 82];
    usdc[44] = 6;
    let account = |data: String, owner: &str| json!({"slot":100,"account":{"owner":owner,"data":[data,"base64"]}});
    let input = json!({
        "So11111111111111111111111111111111111111112":account(encode(&sol),"Synthetic"),
        "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v":account(encode(&usdc),"Synthetic"),
        "3h2e43PunVA5K34vwKCLHWhZF4aZpyaC9RmxvshGAQpL":account(encode(include_bytes!("fixtures/raydium_config.bin")),gap_survey::decode::RAYDIUM),
        "pool":account(encode(include_bytes!("fixtures/raydium_sol_usdc.bin")),gap_survey::decode::RAYDIUM)
    });
    let source = dir.path().join("input.json");
    let target = dir.path().join("output.json");
    std::fs::write(&source, serde_json::to_vec(&input).unwrap()).unwrap();
    assert!(
        std::process::Command::new(env!("CARGO_BIN_EXE_pool_prices"))
            .arg(source)
            .arg(&target)
            .status()
            .unwrap()
            .success()
    );
    let result: serde_json::Value =
        serde_json::from_slice(&std::fs::read(target).unwrap()).unwrap();
    assert_eq!(result.as_object().unwrap().len(), 1);
    assert_eq!(result["pool"]["fee_micro_bps"], 4_000_000);
    assert_eq!(result["pool"]["slot"], 100);
}
