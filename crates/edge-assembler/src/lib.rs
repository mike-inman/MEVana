pub mod edge;

// Re-export primary protobuf types for downstream crates.
pub use edge::{EdgeDelta, EdgeSnapshot, ClmmState, CpmmState, DlmmState, ClobState};
