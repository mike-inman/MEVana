use std::process::Command;

#[test]
fn existing_output_directory_is_never_overwritten() {
    let output = Command::new(env!("CARGO_BIN_EXE_research-replay"))
        .args([
            "/no-such-capture",
            ".",
            "v2-default",
            "commit",
            "toolchain",
            "run",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("output exists; refusing to overwrite")
    );
}
