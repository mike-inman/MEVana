use axum::{
    extract::MatchedPath,
    http::{header::CONTENT_TYPE, Method, Request, StatusCode},
    response::{IntoResponse, Response},
};
use once_cell::sync::Lazy;
use prometheus::{
    proto::MetricFamily, Encoder, HistogramOpts, HistogramVec, IntCounter, IntCounterVec, IntGauge,
    Opts, Registry, TextEncoder,
};
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
    time::Instant,
};
use tower::{Layer, Service};

static REGISTRY: Lazy<Registry> = Lazy::new(Registry::new);

static HTTP_REQUESTS_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
    let opts = Opts::new(
        "mevana_wallet_vault_http_requests_total",
        "Total number of HTTP requests handled by the wallet-vault service",
    );
    let counter = IntCounterVec::new(opts, &["method", "path", "status"]).unwrap();
    REGISTRY
        .register(Box::new(counter.clone()))
        .expect("failed to register wallet-vault HTTP counter");
    counter
});

static HTTP_REQUEST_DURATION_SECONDS: Lazy<HistogramVec> = Lazy::new(|| {
    let opts = HistogramOpts::new(
        "mevana_wallet_vault_http_request_duration_seconds",
        "HTTP request duration for the wallet-vault service",
    )
    .buckets(vec![
        0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 7.5, 10.0,
    ]);
    let histogram = HistogramVec::new(opts, &["method", "path", "status"]).unwrap();
    REGISTRY
        .register(Box::new(histogram.clone()))
        .expect("failed to register wallet-vault histogram");
    histogram
});

static START_TIME_SECONDS: Lazy<IntGauge> = Lazy::new(|| {
    let gauge = IntGauge::new(
        "mevana_wallet_vault_start_time_seconds",
        "Unix timestamp when the wallet-vault service started",
    )
    .unwrap();
    gauge.set(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64,
    );
    REGISTRY
        .register(Box::new(gauge.clone()))
        .expect("failed to register wallet-vault start time gauge");
    gauge
});

static VAULT_OPERATIONS_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
    let counter = IntCounterVec::new(
        Opts::new(
            "mevana_wallet_vault_operations_total",
            "Total wallet-vault operations by type and status",
        ),
        &["operation", "status"],
    )
    .unwrap();
    REGISTRY
        .register(Box::new(counter.clone()))
        .expect("failed to register wallet-vault operations counter");
    counter
});

static KEY_EXPORTS_TOTAL: Lazy<IntCounter> = Lazy::new(|| {
    let counter = IntCounter::new(
        "mevana_wallet_vault_key_exports_total",
        "Total number of wallet key exports",
    )
    .unwrap();
    REGISTRY
        .register(Box::new(counter.clone()))
        .expect("failed to register key export counter");
    counter
});

#[derive(Clone)]
pub struct MetricsLayer;

impl MetricsLayer {
    pub fn new() -> Self {
        let _ = &*HTTP_REQUESTS_TOTAL;
        let _ = &*HTTP_REQUEST_DURATION_SECONDS;
        let _ = &*START_TIME_SECONDS;
        let _ = &*VAULT_OPERATIONS_TOTAL;
        let _ = &*KEY_EXPORTS_TOTAL;
        Self
    }
}

#[derive(Clone)]
pub struct MetricsService<S> {
    inner: S,
}

impl<S> Layer<S> for MetricsLayer {
    type Service = MetricsService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        MetricsService { inner }
    }
}

#[derive(Clone)]
struct HttpRequestLabels {
    method: Method,
    path: String,
    start: Instant,
}

impl HttpRequestLabels {
    fn new(method: Method, path: String) -> Self {
        Self {
            method,
            path,
            start: Instant::now(),
        }
    }
}

impl<S, B, ResBody> Service<Request<B>> for MetricsService<S>
where
    S: Service<Request<B>, Response = axum::http::Response<ResBody>> + Clone + Send + 'static,
    S::Future: Send + 'static,
    S::Error: Send + std::fmt::Display + 'static,
    B: Send + 'static,
    ResBody: Send + 'static,
{
    type Response = axum::http::Response<ResBody>;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<B>) -> Self::Future {
        let labels = HttpRequestLabels::new(req.method().clone(), matched_path(&req));
        let mut inner = self.inner.clone();
        let fut = inner.call(req);

        Box::pin(async move {
            match fut.await {
                Ok(response) => {
                    observe_http(&labels, response.status());
                    Ok(response)
                }
                Err(err) => {
                    observe_http(&labels, StatusCode::INTERNAL_SERVER_ERROR);
                    log::error!("wallet-vault request failed: {}", err);
                    Err(err)
                }
            }
        })
    }
}

fn matched_path<B>(req: &Request<B>) -> String {
    req.extensions()
        .get::<MatchedPath>()
        .map(|p| p.as_str().to_owned())
        .unwrap_or_else(|| req.uri().path().to_owned())
}

fn observe_http(labels: &HttpRequestLabels, status: StatusCode) {
    let duration = labels.start.elapsed();
    let label_values = [
        labels.method.as_str(),
        labels.path.as_str(),
        status.as_str(),
    ];

    HTTP_REQUESTS_TOTAL.with_label_values(&label_values).inc();
    HTTP_REQUEST_DURATION_SECONDS
        .with_label_values(&label_values)
        .observe(duration.as_secs_f64());
}

pub fn record_operation(operation: &'static str, success: bool) {
    let status = if success { "success" } else { "failure" };
    VAULT_OPERATIONS_TOTAL
        .with_label_values(&[operation, status])
        .inc();
}

pub fn record_key_export(success: bool) {
    if success {
        KEY_EXPORTS_TOTAL.inc();
    }
    record_operation("key_export", success);
}

pub async fn metrics_handler() -> Response {
    let metric_families: Vec<MetricFamily> = REGISTRY.gather();
    let mut buffer = Vec::new();
    let encoder = TextEncoder::new();

    match encoder.encode(&metric_families, &mut buffer) {
        Ok(()) => (
            StatusCode::OK,
            [(CONTENT_TYPE, encoder.format_type().to_string())],
            String::from_utf8(buffer).unwrap_or_default(),
        )
            .into_response(),
        Err(err) => {
            log::error!("failed to encode wallet-vault metrics: {}", err);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
