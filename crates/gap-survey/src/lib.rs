//! Indicative public pool-price survey. No execution path.
use anyhow::{ensure, Result};
pub mod analysis;
pub mod capture;
pub mod decode;
pub mod gate;

pub const MAX_RUN_SECONDS: u64 = 3_600;
pub const MAX_COMBINED_BYTES: u64 = 1_500_000_000;

pub fn validate_budget(duration_seconds: u64, max_bytes: u64) -> Result<()> {
    ensure!(
        (1..=MAX_RUN_SECONDS).contains(&duration_seconds),
        "duration out of bounds"
    );
    ensure!(
        (1..=MAX_COMBINED_BYTES).contains(&max_bytes),
        "byte budget out of bounds"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn budgets_reject_zero_and_oversized_runs() {
        assert!(validate_budget(1, 1).is_ok());
        assert!(validate_budget(0, 1).is_err());
        assert!(validate_budget(3_601, 1).is_err());
        assert!(validate_budget(1, 0).is_err());
        assert!(validate_budget(1, MAX_COMBINED_BYTES + 1).is_err());
    }
    #[test]
    fn no_execution_dependencies_or_source_paths() {
        let manifest = include_str!("../Cargo.toml");
        for forbidden in ["wallet-vault", "executor-service", "jito", "solana-client"] {
            assert!(
                !manifest.contains(forbidden),
                "forbidden dependency: {forbidden}"
            );
        }
        for source in [
            include_str!("capture.rs"),
            include_str!("analysis.rs"),
            include_str!("decode.rs"),
            include_str!("main.rs"),
        ] {
            assert!(!source.contains("send_transaction"));
            assert!(!source.contains("sign_transaction"));
            assert!(!source.contains("submit_bundle"));
        }
    }
}
