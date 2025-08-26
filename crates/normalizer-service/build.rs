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
    // Ensure IDLs are present before compiling this crate.
    mevana_idl_registry::sync_idls();
    
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());

    let idl_dir = manifest_dir.join("../../idls");
    let profiles_dir = manifest_dir.join("../../profiles");

    println!("cargo:rerun-if-changed={}", idl_dir.display());
    println!("cargo:rerun-if-changed={}", profiles_dir.display());

    // 1) Generate into OUT_DIR (used by the build)
    let gen = normalizer_codegen::generate(&idl_dir, &profiles_dir, &out_dir);

    // 2) Optionally mirror generated sources to a human-visible folder
    //    Enable with: NORMALIZER_WRITE_GENERATED=1
    if std::env::var("NORMALIZER_WRITE_GENERATED").ok().as_deref() == Some("1") {
        let mirror_dir = manifest_dir.join("../normalizer-generated/src");
        let _ = fs::create_dir_all(&mirror_dir);
        // re-run codegen directly to the mirror dir
        if let Err(e) = normalizer_codegen::generate(&idl_dir, &profiles_dir, &mirror_dir) {
            eprintln!("mirror codegen failed: {e}");
        } else {
            println!(
                "cargo:warning=generated sources mirrored to {}",
                mirror_dir.display()
            );
        }
    }

    if let Err(e) = gen {
        eprintln!("codegen failed, falling back to empty registry: {e}");
        write_empty_registry(&out_dir);
    }
}
