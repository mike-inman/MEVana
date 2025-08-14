// crates/normalizer-service/build.rs
use std::{env, fs, path::PathBuf};

fn write_empty_registry(out_dir: &PathBuf) {
    let stub = r#"
        pub fn registry() -> Vec<Box<dyn normalizer_runtime::SwapExtractor>> {
            Vec::new()
        }
    "#;
    fs::write(out_dir.join("registry.rs"), stub).expect("write stub registry");
}

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());

    let idl_dir = manifest_dir.join("../../idls");
    let profiles_dir = manifest_dir.join("../../profiles");

    println!("cargo:rerun-if-changed={}", idl_dir.display());
    println!("cargo:rerun-if-changed={}", profiles_dir.display());

    match normalizer_codegen::generate(&idl_dir, &profiles_dir, &out_dir) {
        Ok(()) => {}
        Err(e) => {
            eprintln!("codegen failed, falling back to empty registry: {e}");
            write_empty_registry(&out_dir);
        }
    }
}
