pub mod config;
// pub mod engine; // Temporarily disabled due to compilation errors
pub mod engine_fast;
// pub mod executor; // Temporarily disabled due to compilation errors
pub mod hotpath;
pub mod models;
pub mod edge_integration;
// pub mod whirlpool_adapter; // Temporarily disabled due to compilation errors

// Compile-time feature validation
#[cfg(all(feature = "stubs", feature = "production"))]
compile_error!("Cannot enable both 'stubs' and 'production' features - choose one or the other");

// Note: Removed the development requirement check to allow flexible usage

#[cfg(feature = "stubs")]
#[cfg(not(feature = "production"))]
const _: () = {
    // Development mode warning (only shown at compile time)
    #[cfg(not(target_env = "msvc"))] // Hide on Windows to avoid spam
    const WARNING: &str = "
🟡 DEVELOPMENT MODE WARNING 🟡
===============================
The 'stubs' feature is enabled. This means:
- DEX instruction builders return empty/placeholder instructions
- Bundler submits to nowhere (no real transactions)
- Flash loan provider does nothing
- Transaction builder uses dummy blockhashes

This is intended for DEVELOPMENT and TESTING only.
DO NOT use in PRODUCTION environments!

To enable production features, build with: --features production
To disable this warning, build with: --release
===============================
";
};

// Re-export key types for convenience
pub use config::{get_knobs, set_knobs, Knobs};
// pub use engine::Engine; // Temporarily disabled due to compilation errors
pub use engine_fast::FastEngine;
// pub use executor::{ // Temporarily disabled due to compilation errors
//     build_leg_buy, build_leg_sell, build_payer_tx, get_feature_info, BundleSubmitter, Executor,
//     FastExecutor, FlashLoanProvider,
// };
pub use hotpath::SignalQueue;
pub use models::{ArbSignal, Pair, Quote};
// pub use plugin::DexPlugin; // Temporarily disabled due to compilation errors
// pub use whirlpool_adapter::WhirlpoolAdapter; // Temporarily disabled due to compilation errors
