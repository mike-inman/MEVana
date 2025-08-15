mod generated {
    include!(concat!(env!("OUT_DIR"), "/registry.rs"));
}

// Example usage:
fn load_extractors() -> Vec<Box<dyn normalizer_runtime::SwapExtractor>> {
    generated::registry()
}
