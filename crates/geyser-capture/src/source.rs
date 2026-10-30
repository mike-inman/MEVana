//! Raw tonic codec avoids Prost re-encoding and preserves unknown fields byte for byte.
use crate::{
    config::{Provider, Secret},
    format::{now_ns, Gap},
};
use async_trait::async_trait;
use bytes::{Buf, BufMut};
use prost::Message;
use std::{sync::Arc, time::Duration};
use tokio::sync::{mpsc, watch};
use tonic::{
    codec::{Codec, DecodeBuf, Decoder, EncodeBuf, Encoder},
    transport::{Channel, ClientTlsConfig, Endpoint},
    Status,
};
use yellowstone_grpc_proto::prelude::*;

pub struct RawCodec;
pub struct RequestEncoder;
pub struct RawDecoder;
impl Codec for RawCodec {
    type Encode = SubscribeRequest;
    type Decode = Vec<u8>;
    type Encoder = RequestEncoder;
    type Decoder = RawDecoder;
    fn encoder(&mut self) -> RequestEncoder {
        RequestEncoder
    }
    fn decoder(&mut self) -> RawDecoder {
        RawDecoder
    }
}
impl Encoder for RequestEncoder {
    type Item = SubscribeRequest;
    type Error = Status;
    fn encode(
        &mut self,
        item: Self::Item,
        dst: &mut EncodeBuf<'_>,
    ) -> std::result::Result<(), Status> {
        let b = item.encode_to_vec();
        dst.put_slice(&b);
        Ok(())
    }
}
impl Decoder for RawDecoder {
    type Item = Vec<u8>;
    type Error = Status;
    fn decode(&mut self, src: &mut DecodeBuf<'_>) -> std::result::Result<Option<Vec<u8>>, Status> {
        Ok(Some(src.copy_to_bytes(src.remaining()).to_vec()))
    }
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Failure {
    Rejected,
    Quota,
    Authentication,
    Transport,
    ByteBudget,
}
impl Failure {
    pub fn classify(s: &Status) -> Self {
        match s.code() {
            tonic::Code::InvalidArgument
            | tonic::Code::Unimplemented
            | tonic::Code::PermissionDenied => Self::Rejected,
            tonic::Code::ResourceExhausted => Self::Quota,
            tonic::Code::Unauthenticated => Self::Authentication,
            _ => Self::Transport,
        }
    }
    pub fn fatal(self) -> bool {
        self != Self::Transport
    }
}
#[async_trait]
pub trait Source: Send {
    async fn connect(&mut self) -> std::result::Result<(), Failure>;
    async fn next(&mut self) -> std::result::Result<Option<Vec<u8>>, Failure>;
    async fn ping(&mut self) -> std::result::Result<(), Failure>;
}
pub struct GrpcSource {
    endpoint: String,
    provider: Provider,
    secret: Arc<Secret>,
    synthetic: bool,
    request: SubscribeRequest,
    stream: Option<tonic::Streaming<Vec<u8>>>,
    sender: Option<mpsc::Sender<SubscribeRequest>>,
}
impl GrpcSource {
    pub fn new(
        endpoint: String,
        provider: Provider,
        secret: Arc<Secret>,
        synthetic: bool,
        request: SubscribeRequest,
    ) -> Self {
        Self {
            endpoint,
            provider,
            secret,
            synthetic,
            request,
            stream: None,
            sender: None,
        }
    }
}
#[async_trait]
impl Source for GrpcSource {
    async fn connect(&mut self) -> std::result::Result<(), Failure> {
        self.stream = None;
        self.sender = None;
        // Repeat pairing validation at the actual connection boundary, not just in CLI parsing.
        crate::config::validate_pair(self.provider, &self.endpoint, &self.secret, self.synthetic)
            .map_err(|_| Failure::Rejected)?;
        let e = Endpoint::from_shared(self.endpoint.clone())
            .map_err(|_| Failure::Transport)?
            .connect_timeout(Duration::from_secs(10))
            .tls_config(ClientTlsConfig::new().with_native_roots())
            .map_err(|_| Failure::Transport)?;
        let channel: Channel = e.connect().await.map_err(|_| Failure::Transport)?;
        let mut client = tonic::client::Grpc::new(channel)
            .max_decoding_message_size(crate::format::MAX_FRAME - 18);
        client.ready().await.map_err(|_| Failure::Transport)?;
        let (tx, rx) = mpsc::channel(8);
        tx.send(self.request.clone())
            .await
            .map_err(|_| Failure::Transport)?;
        let mut req = tonic::Request::new(tokio_stream::wrappers::ReceiverStream::new(rx));
        // Synthetic endpoints get no real credentials, ever.
        if !self.synthetic {
            req.metadata_mut().insert(
                self.provider.header(),
                self.secret
                    .expose()
                    .parse()
                    .map_err(|_| Failure::Authentication)?,
            );
        }
        let response = client
            .streaming(
                req,
                tonic::codegen::http::uri::PathAndQuery::from_static("/geyser.Geyser/Subscribe"),
                RawCodec,
            )
            .await
            .map_err(|s| Failure::classify(&s))?;
        self.stream = Some(response.into_inner());
        self.sender = Some(tx);
        Ok(())
    }
    async fn next(&mut self) -> std::result::Result<Option<Vec<u8>>, Failure> {
        self.stream
            .as_mut()
            .ok_or(Failure::Transport)?
            .message()
            .await
            .map_err(|s| Failure::classify(&s))
    }
    async fn ping(&mut self) -> std::result::Result<(), Failure> {
        self.sender
            .as_mut()
            .ok_or(Failure::Transport)?
            .send(SubscribeRequest {
                ping: Some(SubscribeRequestPing { id: 1 }),
                ..Default::default()
            })
            .await
            .map_err(|_| Failure::Transport)
    }
}
#[derive(Debug)]
pub enum Event {
    Raw {
        connection: u8,
        receive_ns: u64,
        bytes: Vec<u8>,
        appended: tokio::sync::oneshot::Sender<()>,
    },
    Gap {
        connection: u8,
        receive_ns: u64,
        gap: Gap,
    },
    Fatal(Failure),
}
pub fn backoff(attempt: u32, jitter: u64) -> Duration {
    Duration::from_millis(
        (250u64.saturating_mul(1u64 << attempt.min(7)))
            .saturating_add(jitter % 251)
            .min(30_000),
    )
}
fn gap(
    connection: u8,
    reason: String,
    previous_slot: Option<u64>,
    next_slot: Option<u64>,
) -> Event {
    Event::Gap {
        connection,
        receive_ns: now_ns(),
        gap: Gap {
            connection: if connection == 0 {
                "accounts"
            } else {
                "transactions"
            }
            .into(),
            reason,
            previous_slot,
            next_slot,
        },
    }
}
#[derive(Clone)]
pub struct Budget {
    received: Arc<std::sync::atomic::AtomicU64>,
    limit: u64,
}
impl Budget {
    pub fn new(limit: u64) -> Self {
        Self {
            received: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            limit,
        }
    }
    pub fn account(&self, bytes: u64) -> bool {
        self.received
            .fetch_add(bytes, std::sync::atomic::Ordering::SeqCst)
            .saturating_add(bytes)
            >= self.limit
    }
}
pub async fn drive(
    mut source: impl Source,
    connection: u8,
    tx: mpsc::Sender<Event>,
    mut stop: watch::Receiver<bool>,
    stall: Duration,
    max_failures: u32,
    budget: Budget,
) {
    let mut failures = 0;
    let mut reconnect = false;
    let mut slot = None;
    'outer: loop {
        let result =
            tokio::select! {biased;_ = stop.changed()=>break,result=source.connect()=>result};
        if let Err(e) = result {
            failures += 1;
            if tx
                .send(gap(connection, format!("connect_failed_{e:?}"), None, None))
                .await
                .is_err()
            {
                break;
            }
            if e.fatal() || failures >= max_failures {
                let _ = tx.send(Event::Fatal(e)).await;
                break;
            }
        } else {
            if reconnect
                && tx
                    .send(gap(connection, "reconnect".into(), None, None))
                    .await
                    .is_err()
            {
                break;
            }
            loop {
                let result = tokio::select! {biased;_=stop.changed()=>break 'outer,result=tokio::time::timeout(stall,source.next())=>result};
                match result {
                    Ok(Ok(Some(bytes))) => {
                        failures = 0;
                        let exhausted = budget.account(bytes.len() as u64);
                        let control_bytes = bytes.clone();
                        let receive_ns = now_ns();
                        let (appended, ack) = tokio::sync::oneshot::channel();
                        // Raw bytes are always queued before any interpretation or ping response.
                        if tx
                            .send(Event::Raw {
                                connection,
                                receive_ns,
                                bytes,
                                appended,
                            })
                            .await
                            .is_err()
                        {
                            break 'outer;
                        }
                        if ack.await.is_err() {
                            break 'outer;
                        }
                        if exhausted {
                            let _ = tx.send(Event::Fatal(Failure::ByteBudget)).await;
                            break 'outer;
                        }
                        let decoded = SubscribeUpdate::decode(control_bytes.as_slice());
                        if let Ok(u) = decoded {
                            match u.update_oneof {
                                Some(subscribe_update::UpdateOneof::Ping(_)) => {
                                    if tokio::select!{biased;_=stop.changed()=>break 'outer,result=source.ping()=>result}.is_err() {
                                        let _ = tx
                                            .send(gap(
                                                connection,
                                                "ping_disconnect".into(),
                                                None,
                                                None,
                                            ))
                                            .await;
                                        failures += 1;
                                        break;
                                    }
                                }
                                Some(subscribe_update::UpdateOneof::Slot(s)) if s.status == 0 => {
                                    if let Some(prev) = slot {
                                        if s.slot > prev + 1 {
                                            let _ = tx
                                                .send(gap(
                                                    connection,
                                                    "slot_discontinuity".into(),
                                                    Some(prev),
                                                    Some(s.slot),
                                                ))
                                                .await;
                                        }
                                    }
                                    slot = Some(slot.map_or(s.slot, |prev: u64| prev.max(s.slot)));
                                }
                                _ => {}
                            }
                        }
                    }
                    Err(_) => {
                        failures += 1;
                        let _ = tx.send(gap(connection, "stall".into(), None, None)).await;
                        let _ = tx
                            .send(gap(connection, "disconnect_after_stall".into(), None, None))
                            .await;
                        break;
                    }
                    Ok(Err(e)) => {
                        failures += 1;
                        let _ = tx
                            .send(gap(connection, format!("disconnect_{e:?}"), None, None))
                            .await;
                        if e.fatal() {
                            let _ = tx.send(Event::Fatal(e)).await;
                            break 'outer;
                        }
                        break;
                    }
                    Ok(Ok(None)) => {
                        failures += 1;
                        let _ = tx
                            .send(gap(connection, "disconnect_eof".into(), None, None))
                            .await;
                        break;
                    }
                }
            }
            if failures >= max_failures {
                let _ = tx.send(Event::Fatal(Failure::Transport)).await;
                break;
            }
        }
        reconnect = true;
        let delay = backoff(failures, rand::random());
        tokio::select! {biased;_=stop.changed()=>break,_=tokio::time::sleep(delay)=>{}}
    }
}
