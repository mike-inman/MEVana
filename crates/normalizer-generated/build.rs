use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Make rebuilds reliable
    println!("cargo:rerun-if-changed=../normalizer-codegen/profiles");
    println!("cargo:rerun-if-changed=../mevana-idl-registry/idls");
    println!("cargo:rerun-if-changed=../../idls");

    // OUT_DIR for include!()
    let out_dir = PathBuf::from(std::env::var("OUT_DIR")?);

    // Build an ABSOLUTE path to profiles/
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
    let profiles_dir = manifest_dir
        .join("../normalizer-codegen/profiles")
        .canonicalize()?; // <- absolute now

    // Generate the registry source
    let src = normalizer_codegen::generate_extractors(&profiles_dir, &out_dir)
        .map_err(|e| format!("codegen failed: {e}"))?;

    // Write it next to OUT_DIR so the crate can include!() it
    std::fs::write(out_dir.join("generated_registry.rs"), src)?;

    Ok(())
}

// // crates/normalizer-generated/build.rs
// fn main() -> Result<(), Box<dyn std::error::Error>> {
//     // Re-run if profiles or idls change
//     println!("cargo:rerun-if-changed=../normalizer-codegen/profiles");
//     // In your repo, IDLs are at repo_root/idl-registry/*
//     println!("cargo:rerun-if-changed=../../idl-registry");

//     // Where code will be written
//     let out_dir = std::env::var("OUT_DIR")?;
//     let out_dir = std::path::PathBuf::from(out_dir);

//     // Profiles describe how to map IDL -> roles -> normalized swap
//     let profiles_dir = std::path::PathBuf::from("../normalizer-codegen/profiles");

//     // Generate the registry source (String)
//     let src = normalizer_codegen::generate_extractors(&profiles_dir, &out_dir)
//         .map_err(|e| format!("codegen failed: {e}"))?;

//     // Write to OUT_DIR so we can include!() it at compile time
//     std::fs::write(out_dir.join("generated_registry.rs"), src)?;

//     // Optional: helpful logs
//     println!("cargo:warning=normalizer-generated wrote generated_registry.rs to {}", out_dir.display());
//     Ok(())
// }
