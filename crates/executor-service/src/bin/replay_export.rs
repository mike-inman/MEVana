use anyhow::Result;
use chrono::{DateTime, Utc};
use clap::Parser;
use futures_util::TryStreamExt;
use sqlx::{PgPool, Row};

#[derive(Parser, Debug)]
#[command(
    name = "replay-export",
    about = "Export opportunities + executions for a time range as JSON lines"
)]
struct Args {
    #[arg(long)]
    from: String,
    #[arg(long)]
    to: String,
    #[arg(
        long,
        default_value = "postgresql://mev:<POSTGRES_PASSWORD>@localhost:5433/mevdb"
    )]
    database_url: String,
    #[arg(long)]
    include_deltas: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let args = Args::parse();
    let from_dt: DateTime<Utc> = args.from.parse()?;
    let to_dt: DateTime<Utc> = args.to.parse()?;
    let pool = PgPool::connect(&args.database_url).await?;

    let mut rows = sqlx::query("SELECT o.id as opp_id, o.token_in,o.token_out,o.profit_bps,o.max_size_in,o.detected_at,o.max_edge_age_ms, e.id as exec_id, e.status, e.estimated_profit_lamports,e.actual_profit_lamports,e.execution_time_ms,e.error_message,e.flash_loan_provider,e.executed_at,e.simulated_token_deltas FROM opportunities o LEFT JOIN execution_results e ON e.opportunity_id=o.id WHERE o.detected_at BETWEEN $1 AND $2 ORDER BY o.detected_at")
		.bind(from_dt)
		.bind(to_dt)
		.fetch(&pool);
    while let Some(r) = rows.try_next().await? {
        let mut obj = serde_json::json!({
            "opportunity_id": r.try_get::<uuid::Uuid,_>("opp_id")?,
            "token_in": r.try_get::<String,_>("token_in")?,
            "token_out": r.try_get::<String,_>("token_out")?,
            "profit_bps": r.try_get::<i32,_>("profit_bps")?,
            "max_size_in": r.try_get::<i64,_>("max_size_in")?,
            "detected_at": r.try_get::<chrono::DateTime<Utc>,_>("detected_at")?,
            "max_edge_age_ms": r.try_get::<i32,_>("max_edge_age_ms")?,
            "execution_id": r.try_get::<Option<uuid::Uuid>,_>("exec_id")?,
            "status": r.try_get::<Option<String>,_>("status")?,
            "estimated_profit_lamports": r.try_get::<Option<i64>,_>("estimated_profit_lamports")?,
            "actual_profit_lamports": r.try_get::<Option<i64>,_>("actual_profit_lamports")?,
            "execution_time_ms": r.try_get::<Option<i32>,_>("execution_time_ms")?,
            "error_message": r.try_get::<Option<String>,_>("error_message")?,
            "flash_loan_provider": r.try_get::<Option<String>,_>("flash_loan_provider")?,
            "executed_at": r.try_get::<Option<chrono::DateTime<Utc>>,_>("executed_at")?,
        });
        if args.include_deltas {
            let deltas: Option<serde_json::Value> = r.try_get("simulated_token_deltas")?;
            obj["simulated_token_deltas"] = deltas.unwrap_or(serde_json::Value::Null);
        }
        println!("{}", serde_json::to_string(&obj)?);
    }
    Ok(())
}
