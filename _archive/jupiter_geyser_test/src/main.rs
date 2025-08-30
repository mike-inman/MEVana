use anyhow::Result;
use futures_util::StreamExt;
use std::collections::HashMap;
use tonic::transport::ClientTlsConfig;
use yellowstone_grpc_client::GeyserGrpcClient;
use yellowstone_grpc_proto::{
    geyser::SubscribeRequestFilterTransactions,
    prelude::{
        subscribe_update::UpdateOneof, CommitmentLevel, SubscribeRequest,
    },
};

// Jupiter V6 Program ID - ONLY this program
const JUPITER_PROGRAM_ID: &str = "JUP4Fb2cqiRUcaTHdrPC8h2gNsA2ETXiPDD33WcGuJB";

#[tokio::main]
async fn main() -> Result<()> {
    println!("🎯 JUPITER-ONLY GEYSER STREAM");
    println!("=================================");
    println!("Target: {}", JUPITER_PROGRAM_ID);
    println!("Filtering at Geyser source level");
    println!("=================================\n");

    // Connect to Shyft Geyser
    let endpoint = "https://grpc.ny.shyft.to";
    let x_token = "efff2d23-97ae-4c5f-8b8b-9a6265001189";
    
    println!("🔌 Connecting to Geyser: {}", endpoint);
    
    let mut client = GeyserGrpcClient::build_from_shared(endpoint.to_string())?
        .x_token(Some(x_token.to_string()))?
        .connect_timeout(std::time::Duration::from_secs(10))
        .timeout(std::time::Duration::from_secs(10))
        .tls_config(ClientTlsConfig::new())?
        .connect()
        .await?;

    println!("✅ Connected to Geyser");

    // Build Jupiter-filtered subscription request
    let mut transactions: HashMap<String, SubscribeRequestFilterTransactions> = HashMap::new();
    transactions.insert(
        "jupiter".to_owned(),
        SubscribeRequestFilterTransactions {
            vote: Some(false),
            failed: Some(false),
            account_include: vec![JUPITER_PROGRAM_ID.to_string()], // ONLY Jupiter
            account_exclude: vec![],
            account_required: vec![],
            signature: None,
        },
    );

    let request = SubscribeRequest {
        accounts: Default::default(),
        slots: Default::default(),
        transactions,
        entry: Default::default(),
        blocks: Default::default(),
        blocks_meta: Default::default(),
        account_data_slice: Default::default(),
        ping: None,
        commitment: Some(CommitmentLevel::Confirmed as i32),
    };

    println!("🔍 Geyser filter configured:");
    println!("   account_include: [{}]", JUPITER_PROGRAM_ID);
    println!("   vote: false, failed: false");
    println!("   commitment: Confirmed");
    println!("\n🚀 Starting Jupiter-only stream...\n");

    // Subscribe and process Jupiter transactions
    let (mut _subscribe_tx, mut stream) = client.subscribe_with_request(Some(request)).await?;

    let mut jupiter_count = 0;
    let start_time = std::time::Instant::now();

    while let Some(message) = stream.next().await {
        match message {
            Ok(msg) => {
                if let Some(UpdateOneof::Transaction(transaction_update)) = msg.update_oneof {
                    jupiter_count += 1;
                    
                    let signature = if let Some(transaction) = &transaction_update.transaction {
                        if let Some(info) = &transaction.transaction {
                            format!("{}...{}", 
                                &bs58::encode(&info.signature).into_string()[..8],
                                &bs58::encode(&info.signature).into_string()[56..])
                        } else { "unknown".to_string() }
                    } else { "unknown".to_string() };

                    println!("🎯 JUPITER TRANSACTION #{}: {} (slot: {})", 
                        jupiter_count, 
                        signature,
                        transaction_update.slot
                    );

                    // Show progress every 10 transactions
                    if jupiter_count % 10 == 0 {
                        let elapsed = start_time.elapsed().as_secs();
                        let rate = if elapsed > 0 { jupiter_count as f64 / elapsed as f64 } else { 0.0 };
                        println!("📊 {} Jupiter transactions in {}s ({:.2}/s)", 
                            jupiter_count, elapsed, rate);
                    }
                }
            }
            Err(e) => {
                println!("❌ Stream error: {}", e);
                break;
            }
        }
    }

    Ok(())
}
