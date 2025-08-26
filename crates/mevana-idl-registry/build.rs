fn main() {
    // Re-run if config changes
    println!("cargo:rerun-if-env-changed=MEVANA_IDL_CONFIG");
    println!("cargo:rerun-if-env-changed=IDL_REGISTRY_DIR");
    // Note: we can't call sync_idls() here since this is the build script 
    // for the crate itself. The sync happens when other crates build.
}
