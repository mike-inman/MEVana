#![allow(unused_imports)]

use anyhow::Result;
use axum::{
    extract::{
        ws::{Message, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
    routing::get,
    Router,
};
use futures_util::{SinkExt, StreamExt};
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::{ClientConfig, Message as _};
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;

#[derive(Clone)]
struct AppState {
    tx: broadcast::Sender<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let broker = std::env::var("KAFKA_BROKER").unwrap_or_else(|_| "localhost:9092".to_string());
    let topic = std::env::var("OUT_TOPIC").unwrap_or_else(|_| "enriched.swaps.all".to_string());
    let addr = std::env::var("FEED_ADDR").unwrap_or_else(|_| "0.0.0.0:4000".to_string());

    let (tx, _rx) = broadcast::channel::<String>(1024);
    let st = AppState { tx: tx.clone() };

    tokio::spawn(run_consumer(broker.clone(), topic.clone(), tx));

    let app = Router::new().route("/ws", get(ws_handler)).with_state(st);

    println!("🔌 swap-feed listening on ws://{addr}/ws (topic={topic})");
    let listener = TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn run_consumer(broker: String, topic: String, tx: broadcast::Sender<String>) -> Result<()> {
    let consumer: StreamConsumer = ClientConfig::new()
        .set("bootstrap.servers", &broker)
        .set("group.id", "swap-feed")
        .set("enable.partition.eof", "false")
        .set("auto.offset.reset", "latest")
        .create()?;
    consumer.subscribe(&[&topic])?;

    loop {
        match consumer.recv().await {
            Err(e) => {
                eprintln!("kafka error: {e}");
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
            Ok(m) => {
                if let Some(payload) = m.payload_view::<str>().transpose().ok().flatten() {
                    let _ = tx.send(payload.to_string());
                }
            }
        }
    }
}

async fn ws_handler(State(st): State<AppState>, ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(move |mut socket| async move {
        let mut stream = BroadcastStream::new(st.tx.subscribe());
        while let Some(Ok(msg)) = stream.next().await {
            if socket.send(Message::Text(msg)).await.is_err() {
                break;
            }
        }
    })
}
