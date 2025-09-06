// crates/normalizer-generated/src/lib.rs
pub mod registry {
    include!(concat!(env!("OUT_DIR"), "/generated_registry.rs"));
}
pub use registry::{decoder_for, registry};
