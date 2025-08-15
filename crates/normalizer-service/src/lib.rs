// crates/normalizer-service/src/lib.rs
// Re-export the generated registry/decoder so other crates can use them.

pub mod generated {
    // build.rs writes {program_*.rs, registry.rs} into OUT_DIR
    include!(concat!(env!("OUT_DIR"), "/registry.rs"));
}

// Public surface: let other crates grab the decoders/extractors
pub use generated::decoder_for;
pub use generated::registry;

// Example usage:
fn load_extractors() -> Vec<Box<dyn normalizer_runtime::SwapExtractor>> {
    generated::registry()
}
