use anyhow::{ensure, Result};
use clap::{Parser, Subcommand};
use gap_survey::capture::{record, CaptureArgs};
use gap_survey::{
    analysis::{write_analysis, Analysis},
    gate::evaluate,
};
use geyser_capture::config::Provider;
use std::path::PathBuf;

#[derive(Parser)]
#[command(about = "Bounded, read-only Indicative pool-price survey")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Record {
        #[arg(long)]
        run_id: String,
        #[arg(long, value_parser = ["pilot", "main"])]
        phase: String,
        #[arg(long, value_enum, default_value = "sodae")]
        provider: Provider,
        #[arg(long)]
        duration: u64,
        #[arg(long, required = true)]
        max_bytes: u64,
        #[arg(long, default_value = "crates/gap-survey/allowlist.json")]
        allowlist: PathBuf,
        #[arg(
            long,
            default_value = "/Users/michaelinman/MEVanaData/captures/handoff18-20261031"
        )]
        output_root: PathBuf,
        #[arg(long, default_value = "/Users/michaelinman/Repos/MEVana/.env")]
        credentials: PathBuf,
    },
    Analyze {
        #[arg(long)]
        run_dir: PathBuf,
        #[arg(long)]
        output: PathBuf,
        #[arg(long, default_value = "crates/gap-survey/allowlist.json")]
        allowlist: PathBuf,
    },
    Gate {
        #[arg(long,required=true,num_args=1..)]
        analyses: Vec<PathBuf>,
        #[arg(long)]
        output: PathBuf,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Record {
            run_id,
            phase,
            provider,
            duration,
            max_bytes,
            allowlist,
            output_root,
            credentials,
        } => {
            ensure!(duration > 0, "duration must be positive");
            let m = record(CaptureArgs {
                allowlist: &allowlist,
                output_root: &output_root,
                credentials: &credentials,
                provider,
                duration_seconds: duration,
                max_bytes,
                run_id: &run_id,
                phase: &phase,
            })
            .await?;
            println!(
                "run={} updates={} payload_bytes={} gaps={} stalls={} stop={}",
                m.run_id, m.updates, m.payload_bytes, m.gaps, m.stalls, m.stop_reason
            );
        }
        Command::Analyze {
            run_dir,
            output,
            allowlist,
        } => {
            write_analysis(&run_dir, &allowlist, &output)?;
            println!("Wrote deterministic Indicative analysis");
        }
        Command::Gate { analyses, output } => {
            let runs: Vec<Analysis> = analyses
                .iter()
                .map(|p| serde_json::from_slice(&std::fs::read(p)?).map_err(anyhow::Error::from))
                .collect::<Result<_>>()?;
            let results = evaluate(&runs)?;
            std::fs::write(output, serde_json::to_vec_pretty(&results)?)?;
            println!("Wrote {} fixed-gate segment results", results.len());
        }
    }
    Ok(())
}
