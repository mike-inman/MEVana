use clap::Parser;
#[derive(Parser)]
struct Args {
    directory: std::path::PathBuf,
}
fn main() -> anyhow::Result<()> {
    let a = Args::parse();
    for entry in std::fs::read_dir(&a.directory)? {
        let entry = entry?;
        if entry.path().extension().is_some_and(|e| e == "part") {
            eprintln!(
                "incomplete (never catalog): {}",
                entry.file_name().to_string_lossy()
            );
        }
    }
    let report = geyser_capture::inspect::inspect(&a.directory)?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    anyhow::ensure!(
        report.incomplete.is_empty(),
        "incomplete .part files present; never catalog this session"
    );
    Ok(())
}
