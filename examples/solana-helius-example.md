
To get Solana data into Prometheus and Grafana using Rust with Helius, you can build a custom application that acts as a metrics exporter. The Rust application will use the official Helius SDK to fetch data from the Solana blockchain, format it into Prometheus metrics, and serve it on an HTTP endpoint for Prometheus to scrape. 
This process involves four main components:
Helius: An API platform for fetching enriched Solana blockchain data.
Rust Exporter: A custom Rust application that uses the Helius Rust SDK to retrieve data and exposes it in a Prometheus-compatible format.
Prometheus: A monitoring system that scrapes and stores the time-series data from your Rust exporter.
Grafana: A visualization tool that queries data from Prometheus to create dashboards. 
Step 1: Set up your Rust project
First, create a new Rust project and add the necessary dependencies.
Create a new project:
sh
cargo new solana_helius_exporter
cd solana_helius_exporter
Add dependencies: In your Cargo.toml file, add the helius, prometheus, tokio, and warp crates.
toml
[dependencies]
helius = "0.2.1"
prometheus = "0.13.3"
tokio = { version = "1", features = ["full"] }
warp = "0.3.6"
Step 2: Write the Rust exporter code
In this example, the Rust application will expose a metric for the current Solana block height.
Define metrics: Create a file src/main.rs. Use lazy_static to initialize your Prometheus metric once.
Use Helius to fetch data: The exporter will use the Helius client to make an RPC call and get the current slot.
Create an HTTP server: Use warp to create a simple HTTP server that serves the Prometheus metrics on a /metrics endpoint. 
src/main.rs
rust
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;

use helius::{
    error::Result,
    types::{Cluster, RpcBlockNumber},
    Helius,
};
use lazy_static::lazy_static;
use prometheus::{opts, register_gauge, Encoder, Gauge, TextEncoder};
use tokio::{self, time::interval};
use warp::Filter;

lazy_static! {
    static ref BLOCK_HEIGHT: Gauge = register_gauge!(opts!(
        "solana_block_height",
        "The current block height on the Solana network."
    ))
    .expect("metric can be created");
}

async fn get_metrics(
    helius: Arc<Helius>,
) -> std::result::Result<impl warp::Reply, Infallible> {
    // Fetch the current block height from Helius.
    let rpc_result: Result<RpcBlockNumber> = helius.rpc().get_block_height(None).await;

    // Update the Prometheus gauge with the new value.
    if let Ok(block_height) = rpc_result {
        BLOCK_HEIGHT.set(block_height.into());
    } else {
        // Log the error but don't fail the request.
        eprintln!("Failed to fetch block height from Helius.");
    }

    // Encode and serve the metrics for Prometheus to scrape.
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = vec![];
    encoder.encode(&metric_families, &mut buffer).unwrap();

    let output = String::from_utf8(buffer).unwrap();
    Ok(output)
}

#[tokio::main]
async fn main() -> Result<()> {
    // Replace with your actual Helius API key and cluster.
    let helius_api_key = std::env::var("HELIUS_API_KEY")
        .expect("HELIUS_API_KEY must be set");
    let cluster = Cluster::MainnetBeta;
    let helius = Arc::new(Helius::new(&helius_api_key, cluster)?);

    // Endpoint for Prometheus to scrape.
    let metrics_route = warp::path("metrics")
        .and(with_helius(helius.clone()))
        .and_then(get_metrics);

    println!("Serving metrics on http://127.0.0.1:9184/metrics");
    warp::serve(metrics_route)
        .run(([127, 0, 0, 1], 9184))
        .await;

    Ok(())
}

fn with_helius(
    helius: Arc<Helius>,
) -> impl Filter<Extract = (Arc<Helius>,), Error = Infallible> + Clone {
    warp::any().map(move || helius.clone())
}
Step 3: Configure Prometheus
After building your Rust exporter, you need to configure Prometheus to scrape its /metrics endpoint.
Install Prometheus: Follow the official Prometheus documentation to install it.
Update prometheus.yml: Add a new scrape job to your Prometheus configuration file to collect metrics from your Rust exporter.
yaml
# my_prometheus.yml
global:
  scrape_interval: 15s

scrape_configs:
  - job_name: 'solana_exporter_rust'
    static_configs:
      - targets: ['localhost:9184'] # Address where your Rust exporter is running
Run Prometheus: Start Prometheus with your new configuration file.
sh
prometheus --config.file=my_prometheus.yml
 
Step 4: Set up Grafana dashboard
Finally, you will connect Grafana to your Prometheus data source to visualize the Solana metrics. 
Install Grafana: Follow the official Grafana installation guide.
Add Prometheus data source:
Log in to Grafana and navigate to Connections > Add new connection.
Search for "Prometheus" and click Add new data source.
Set the Prometheus server URL to http://localhost:9090 (the default Prometheus address).
Click Save & test.
Create a dashboard:
Navigate to Dashboards > New Dashboard.
Add a new panel and select your Prometheus data source.
In the query editor, enter solana_block_height.
Configure the visualization settings (e.g., set the panel title to "Solana Block Height").
Click Apply to see your metric. You should see the block height updating automatically as Prometheus scrapes the data from your Rust application. 1