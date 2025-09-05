use anyhow::Result;
use log::info;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use tokio::signal;
use uuid::Uuid;

mod account_registry;
mod audit_log;
mod bundle_builder;
mod config;
mod execution_engine;
mod flash_loan;
mod hash_chain_logger;
mod hotpath_ws;
mod live_stream;
mod metrics;
mod opportunity_processor;
mod readiness;
mod shyft_client;
mod token_registry;
mod wallet_service; // new live edge ingestion (opt-in)

use config::ExecutorConfig;
use execution_engine::ExecutionEngine;
use hash_chain_logger::global_logger;
use metrics::MetricsServer;
use parking_lot::Mutex as PLMutex;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenvy::dotenv();
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    info!("🚀 Starting MEVana Executor Service");

    let config = ExecutorConfig::load()?;
    info!("Configuration loaded successfully");

    // Geyser-only diagnostic mode: skip DB, migrations, engine init, wallet, kafka; just stream.
    let geyser_only = std::env::var("GEYSER_ONLY")
        .ok()
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    if geyser_only {
        info!("geyser_only_mode_enabled - skipping DB, engine, wallet, kafka; streaming only");
        if std::env::var("ENABLE_LIVE_EDGE_STREAM")
            .ok()
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false)
        {
            let endpt_res = std::env::var("GEYSER_ENDPOINT").or_else(|_| std::env::var("ENDPOINT"));
            let x_token_res = std::env::var("GEYSER_X_TOKEN").or_else(|_| std::env::var("X_TOKEN"));
            if let (Ok(endpt), Ok(x_token)) = (endpt_res, x_token_res) {
                log::info!(
                    "live_edge_stream_enabled endpoint={} token_present=1 (geyser_only)",
                    endpt
                );
                let (_handle, mut rx) = live_stream::start_live_stream(&endpt, &x_token).await?;
                tokio::spawn(async move {
                    use log::info;
                    let mut count: u64 = 0;
                    while let Some(snap) = rx.recv().await {
                        count += 1;
                        if count <= 20 {
                            info!("snapshot_sample={:?}", snap);
                        }
                        if count % 1000 == 0 {
                            info!("live_snapshots_received count={}", count);
                        }
                    }
                });
                // Park main until Ctrl+C
                signal::ctrl_c().await.ok();
                info!("geyser_only_mode_exiting");
                return Ok(());
            } else {
                log::warn!(
                    "geyser_only_mode_missing_env GEYSER_ENDPOINT or GEYSER_X_TOKEN not set"
                );
                return Ok(());
            }
        } else {
            log::warn!("geyser_only_mode ENABLE_LIVE_EDGE_STREAM not enabled - nothing to do");
            return Ok(());
        }
    }

    let metrics_server = Arc::new(MetricsServer::new(config.metrics_port)?);
    let metrics_clone = Arc::clone(&metrics_server);
    let _metrics_handle = tokio::spawn(async move { metrics_clone.start().await });

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://mev:mevpassword@localhost:5433/mevdb".into());
    let disable_db = std::env::var("DISABLE_DB")
        .ok()
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    let pg_pool = if disable_db {
        log::info!(
            "db_disabled_using_connect_lazy DISABLE_DB=1 url={}",
            database_url
        );
        PgPoolOptions::new().connect_lazy(&database_url)?
    } else {
        PgPool::connect(&database_url).await?
    };
    if !disable_db
        && if let Ok(md) = std::fs::metadata("executor-service/migrations") {
            md.is_dir()
        } else {
            false
        }
    {
        if let Ok(entries) = std::fs::read_dir("executor-service/migrations") {
            let mut applied = 0u32;
            let mut paths: Vec<_> = entries.flatten().map(|e| e.path()).collect();
            paths.sort();
            fn split_pg_statements(input: &str) -> Vec<String> {
                let mut out = Vec::new();
                let mut buf = String::new();
                let mut chars = input.chars().peekable();
                let mut in_dollar = false;
                while let Some(c) = chars.next() {
                    if c == '$' && chars.peek() == Some(&'$') {
                        // toggle $$ block
                        buf.push(c);
                        buf.push(chars.next().unwrap());
                        in_dollar = !in_dollar;
                        continue;
                    }
                    if c == ';' && !in_dollar {
                        let trimmed = buf.trim();
                        if !trimmed.is_empty() && !trimmed.starts_with("--") {
                            out.push(trimmed.to_string());
                        }
                        buf.clear();
                    } else {
                        buf.push(c);
                    }
                }
                let trimmed = buf.trim();
                if !trimmed.is_empty() && !trimmed.starts_with("--") {
                    out.push(trimmed.to_string());
                }
                out
            }
            for path in paths {
                if path.extension().and_then(|s| s.to_str()) == Some("sql") {
                    if let Ok(sql_raw) = std::fs::read_to_string(&path) {
                        let statements = split_pg_statements(&sql_raw);
                        let mut ok = true;
                        for stmt in statements {
                            if let Err(e) = sqlx::query(&stmt).execute(&pg_pool).await {
                                log::warn!(
                                    "migration_apply_failed path={:?} snippet='{}' err={}",
                                    path,
                                    stmt.chars().take(80).collect::<String>(),
                                    e
                                );
                                ok = false;
                                break;
                            }
                        }
                        if ok {
                            applied += 1;
                        }
                    }
                }
            }
            log::info!("postgres_migrations_applied count={}", applied);
        }
    }

    let git_commit = std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                String::from_utf8(o.stdout).ok()
            } else {
                None
            }
        })
        .map(|s| s.trim().to_string());
    let cfg_bytes = serde_json::to_vec(&config).unwrap_or_default();
    use sha2::Digest;
    let mut sha = sha2::Sha256::new();
    sha.update(&cfg_bytes);
    let config_hash = format!("{:x}", sha.finalize());
    std::env::set_var("CONFIG_HASH", &config_hash);
    let run_id = Uuid::new_v4();
    let redacted: std::collections::BTreeMap<String, String> = std::env::vars()
        .filter(|(k, _)| k.contains("KEY") || k.contains("TOKEN") || k.contains("SECRET"))
        .map(|(k, _)| (k, "***REDACTED***".into()))
        .collect();
    if !disable_db {
        if let Err(e) = sqlx::query("INSERT INTO run_provenance (run_id, git_commit, config_hash, env_redacted, version) VALUES ($1,$2,$3,$4,$5)")
            .bind(run_id)
            .bind(git_commit.clone())
            .bind(&config_hash)
            .bind(serde_json::to_value(&redacted).unwrap_or(serde_json::json!({})))
            .bind(env!("CARGO_PKG_VERSION"))
            .execute(&pg_pool).await
        {
            log::warn!("run_provenance_insert_failed err={}", e);
        } else {
            log::info!("run_provenance_inserted run_id={} config_hash={}", run_id, config_hash);
        }
    } else {
        log::info!("run_provenance_skipped disable_db=true");
    }

    let execution_engine =
        Arc::new(ExecutionEngine::new(config, Arc::clone(&metrics_server), pg_pool.clone()).await?);
    info!("Execution engine initialized");
    execution_engine
        .token_registry
        .start_background_enrichment(token_registry::NoopEnricher)
        .await;

    // Optional live edge stream (geyser) startup
    if std::env::var("ENABLE_LIVE_EDGE_STREAM")
        .ok()
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
    {
        // Endpoint precedence: GEYSER_ENDPOINT > ENDPOINT
        let endpt_res = std::env::var("GEYSER_ENDPOINT").or_else(|_| std::env::var("ENDPOINT"));
        let x_token_res = std::env::var("GEYSER_X_TOKEN").or_else(|_| std::env::var("X_TOKEN"));
        if let (Ok(endpt), Ok(x_token)) = (endpt_res, x_token_res) {
            log::info!(
                "live_edge_stream_enabled endpoint={} token_present=1",
                endpt
            );
            let (_handle, mut rx) = live_stream::start_live_stream(&endpt, &x_token).await?;
            // Minimal in-process edge detector (currently only OrcaWhirlpool edges built).
            use arb_engine::edge_integration::EdgeArbitrageDetector;
            use arb_engine::hotpath::SignalQueue;
            use live_stream::snapshot_to_edges;
            let q = Arc::new(SignalQueue::new(1024));
            let detector = Arc::new(EdgeArbitrageDetector::new(q.sender()));
            tokio::spawn(async move {
                use log::info;
                let mut count: u64 = 0;
                while let Some(snap) = rx.recv().await {
                    for edge in snapshot_to_edges(&snap) {
                        if let Err(e) = detector.process_edge_update(edge) {
                            log::warn!("edge_update_failed err={}", e);
                        }
                    }
                    count += 1;
                    if count % 1000 == 0 {
                        info!("live_snapshots_received count={}", count);
                    }
                }
            });
        } else {
            log::warn!("live_edge_stream_missing_env GEYSER_ENDPOINT or GEYSER_X_TOKEN not set");
        }
    }
    // Initialize live trading flag based on env (will still honor dry-run + allow_live gating)
    if std::env::var("EXECUTION_LIVE_FLAG_INITIAL")
        .ok()
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
    {
        execution_engine.enable_live_trading();
    } else {
        execution_engine.disable_live_trading();
    }

    // Admin + registry HTTP (optional mTLS via feature + env ADMIN_MTLS_ENABLE)
    {
        let engine_ref_outer = Arc::clone(&execution_engine);
        let admin_token = std::env::var("ADMIN_API_TOKEN").ok();
        let admin_token_arc = Arc::new(admin_token);
        // Optional allowlist of client certificate subjects (comma separated)
        let allowed_subjects: Option<std::collections::HashSet<String>> =
            std::env::var("ADMIN_ALLOWED_SUBJECTS").ok().map(|v| {
                v.split(',')
                    .filter_map(|s| {
                        let t = s.trim();
                        if t.is_empty() {
                            None
                        } else {
                            Some(t.to_string())
                        }
                    })
                    .collect()
            });
        let rate_limiter: Arc<PLMutex<HashMap<String, (u64, std::time::Instant)>>> =
            Arc::new(PLMutex::new(HashMap::new()));
        fn allow(
            addr: &str,
            rl: &Arc<PLMutex<HashMap<String, (u64, std::time::Instant)>>>,
            limit: u64,
            window_secs: u64,
        ) -> bool {
            let mut m = rl.lock();
            let now = std::time::Instant::now();
            let e = m.entry(addr.to_string()).or_insert((0, now));
            if now.duration_since(e.1).as_secs() >= window_secs {
                *e = (0, now);
            }
            if e.0 < limit {
                e.0 += 1;
                true
            } else {
                false
            }
        }
        let enable_mtls_env = std::env::var("ADMIN_MTLS_ENABLE")
            .ok()
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);
        #[cfg(not(feature = "admin-mtls"))]
        if enable_mtls_env {
            log::warn!("ADMIN_MTLS_ENABLE set but feature 'admin-mtls' not compiled");
        }
        tokio::spawn(async move {
            use warp::http::StatusCode;
            use warp::Filter;
            let engine_ref = Arc::clone(&engine_ref_outer);
            let rl_get = Arc::clone(&rate_limiter);
            let get_token = warp::path!("token" / String)
                .and(warp::get())
                .and(warp::addr::remote())
                .map(move |mint: String, remote: Option<SocketAddr>| {
                    engine_ref.metrics().admin_requests_total.inc();
                    if let Some(addr) = remote {
                        if !allow(&addr.ip().to_string(), &rl_get, 120, 60) {
                            engine_ref.metrics().admin_rate_limited_total.inc();
                            return warp::reply::json(&serde_json::json!({"error":"rate_limited"}));
                        }
                    }
                    if let Some(meta) = engine_ref.token_registry.get(&mint) {
                        warp::reply::json(&serde_json::json!({
                            "mint": meta.mint,
                            "symbol": meta.symbol,
                            "name": meta.name,
                            "decimals": meta.decimals,
                            "logo_url": meta.logo_url,
                            "first_seen": meta.first_seen.to_rfc3339(),
                            "last_refreshed": meta.last_refreshed.to_rfc3339(),
                            "needs_refresh": meta.needs_refresh,
                            "tags": meta.tags,
                        }))
                    } else {
                        warp::reply::json(&serde_json::json!({"error":"unknown_mint"}))
                    }
                });
            let engine_ref2 = Arc::clone(&engine_ref_outer);
            let rl_refresh = Arc::clone(&rate_limiter);
            let allowed_subjects_refresh = allowed_subjects.clone();
            let admin_token_arc_refresh = Arc::clone(&admin_token_arc);
            let refresh = warp::path!("admin"/"registry"/"refresh")
                .and(warp::post())
                .and(warp::addr::remote())
                .and(warp::header::optional::<String>("x-admin-token"))
                .and(warp::header::optional::<String>("x-client-subject"))
                .and_then(move |remote: Option<SocketAddr>, provided: Option<String>, client_subj: Option<String>| {
                    let engine_clone = Arc::clone(&engine_ref2);
            let expected = (*admin_token_arc_refresh).clone();
                    let rl_inner = Arc::clone(&rl_refresh);
                    let allowlist = allowed_subjects_refresh.clone();
                    async move {
                        engine_clone.metrics().admin_requests_total.inc();
                        engine_clone.metrics().admin_refresh_requests_total.inc();
                        if let Some(addr) = remote {
                            if !allow(&addr.ip().to_string(), &rl_inner, 30, 60) {
                                engine_clone.metrics().admin_rate_limited_total.inc();
                                return Ok::<_, warp::Rejection>(warp::reply::with_status(
                                    warp::reply::json(&serde_json::json!({"status":"rate_limited"})),
                                    StatusCode::TOO_MANY_REQUESTS,
                                ));
                            }
                        }
                        // Evaluate subject allowlist, capturing decision
                        let subject_ok = if let Some(list) = &allowlist {
                            match client_subj.as_ref().map(|s| s.trim()) {
                                Some(subj) => {
                                    if list.contains(subj) { engine_clone.metrics().admin_subject_allowed_total.inc(); true } else { engine_clone.metrics().admin_unauthorized_total.inc(); false }
                                },
                                None => { engine_clone.metrics().admin_subject_missing_total.inc(); engine_clone.metrics().admin_unauthorized_total.inc(); false }
                            }
                        } else { true };
                        // Evaluate token
                        let token_ok = match (&expected, &provided) {
                            (Some(exp), Some(got)) if got == exp => { engine_clone.metrics().admin_token_valid_total.inc(); true },
                            (Some(_), _) => { engine_clone.metrics().admin_token_invalid_total.inc(); false },
                            (None, _) => true,
                        };
                        // Dual-auth rule: if both an allowlist AND a token are configured, require BOTH pass.
                        // If only one mechanism configured, require that one.
                        let authorized = if expected.is_some() && allowlist.is_some() {
                            engine_clone.metrics().admin_dual_auth_required_total.inc();
                            subject_ok && token_ok
                        } else if expected.is_some() {
                            token_ok
                        } else if allowlist.is_some() {
                            subject_ok
                        } else { true };
                        if !authorized {
                            engine_clone.metrics().admin_unauthorized_total.inc();
                            let status_detail = if !subject_ok && allowlist.is_some() { "unauthorized_subject" } else { "unauthorized" };
                            return Ok::<_, warp::Rejection>(warp::reply::with_status(
                                warp::reply::json(&serde_json::json!({"status":status_detail})),
                                StatusCode::UNAUTHORIZED,
                            ));
                        }
                        let resp = match engine_clone.persist_registry_state().await {
                            Ok(hash) => warp::reply::with_status(
                                warp::reply::json(&serde_json::json!({"status":"ok","registry_hash":hash})),
                                StatusCode::OK,
                            ),
                            Err(e) => warp::reply::with_status(
                                warp::reply::json(&serde_json::json!({"status":"error","error":e.to_string()})),
                                StatusCode::INTERNAL_SERVER_ERROR,
                            ),
                        };
                        crate::audit_log::log_event("registry_refresh", remote.as_ref(), client_subj.as_deref(), token_ok, authorized, None);
                        Ok::<_, warp::Rejection>(resp)
                    }
                });
            // Trading status (read-only) endpoint with same auth gating as refresh (so no info leak)
            let engine_ref_status = Arc::clone(&engine_ref_outer);
            let allowed_subjects_status = allowed_subjects.clone();
            let admin_token_arc_status = Arc::clone(&admin_token_arc);
            let trading_status = warp::path!("admin" / "trading" / "status")
                .and(warp::get())
                .and(warp::addr::remote())
                .and(warp::header::optional::<String>("x-admin-token"))
                .and(warp::header::optional::<String>("x-client-subject"))
                .and_then(
                    move |remote: Option<SocketAddr>,
                          provided: Option<String>,
                          client_subj: Option<String>| {
                        let engine_clone = Arc::clone(&engine_ref_status);
                        let expected = (*admin_token_arc_status).clone();
                        let allowlist = allowed_subjects_status.clone();
                        async move {
                            let subject_ok = if let Some(list) = &allowlist {
                                client_subj
                                    .as_ref()
                                    .map(|s| list.contains(s.trim()))
                                    .unwrap_or(false)
                            } else {
                                true
                            };
                            let token_ok = match (&expected, &provided) {
                                (Some(exp), Some(got)) if got == exp => true,
                                (Some(_), _) => false,
                                (None, _) => true,
                            };
                            let authorized = if expected.is_some() && allowlist.is_some() {
                                subject_ok && token_ok
                            } else if expected.is_some() {
                                token_ok
                            } else if allowlist.is_some() {
                                subject_ok
                            } else {
                                true
                            };
                            if !authorized {
                                return Ok::<_, warp::Rejection>(warp::reply::with_status(
                                    warp::reply::json(
                                        &serde_json::json!({"status":"unauthorized"}),
                                    ),
                                    StatusCode::UNAUTHORIZED,
                                ));
                            }
                            let live_flag = engine_clone.live_trading_status();
                            let dry_run = std::env::var("EXECUTION_DRY_RUN")
                                .ok()
                                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                                .unwrap_or(true);
                            let allow_live = std::env::var("EXECUTION_ALLOW_LIVE")
                                .ok()
                                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                                .unwrap_or(false);
                            let effective_live = !dry_run && allow_live && live_flag;
                            crate::audit_log::log_event(
                                "trading_status",
                                remote.as_ref(),
                                client_subj.as_deref(),
                                token_ok,
                                authorized,
                                None,
                            );
                            Ok::<_, warp::Rejection>(warp::reply::with_status(
                                warp::reply::json(&serde_json::json!({
                                    "status":"ok",
                                    "trading_enabled_flag": live_flag,
                                    "dry_run": dry_run,
                                    "allow_live_env": allow_live,
                                    "effective_live": effective_live
                                })),
                                StatusCode::OK,
                            ))
                        }
                    },
                );
            // Trading control endpoints (enable/disable live trading kill switch)
            let engine_ref_trading_enable = Arc::clone(&engine_ref_outer);
            let allowed_subjects_enable = allowed_subjects.clone();
            let admin_token_arc_enable = Arc::clone(&admin_token_arc);
            let trading_enable = warp::path!("admin" / "trading" / "enable")
                .and(warp::post())
                .and(warp::addr::remote())
                .and(warp::header::optional::<String>("x-admin-token"))
                .and(warp::header::optional::<String>("x-client-subject"))
                .and_then(
                    move |remote: Option<SocketAddr>,
                          provided: Option<String>,
                          client_subj: Option<String>| {
                        let engine_clone = Arc::clone(&engine_ref_trading_enable);
                        let expected = (*admin_token_arc_enable).clone();
                        let allowlist = allowed_subjects_enable.clone();
                        async move {
                            engine_clone.metrics().admin_requests_total.inc();
                            // Reuse auth logic (simplified: must be authorized like refresh endpoint)
                            let subject_ok = if let Some(list) = &allowlist {
                                match client_subj.as_ref().map(|s| s.trim()) {
                                    Some(subj) => list.contains(subj),
                                    None => false,
                                }
                            } else {
                                true
                            };
                            let token_ok = match (&expected, &provided) {
                                (Some(exp), Some(got)) if got == exp => true,
                                (Some(_), _) => false,
                                (None, _) => true,
                            };
                            let authorized = if expected.is_some() && allowlist.is_some() {
                                subject_ok && token_ok
                            } else if expected.is_some() {
                                token_ok
                            } else if allowlist.is_some() {
                                subject_ok
                            } else {
                                true
                            };
                            if !authorized {
                                engine_clone.metrics().admin_unauthorized_total.inc();
                                return Ok::<_, warp::Rejection>(warp::reply::with_status(
                                    warp::reply::json(
                                        &serde_json::json!({"status":"unauthorized"}),
                                    ),
                                    StatusCode::UNAUTHORIZED,
                                ));
                            }
                            engine_clone.enable_live_trading();
                            log::info!(
                                "audit_event action=trading_enable remote={:?} subject={:?}",
                                remote,
                                client_subj
                            );
                            crate::audit_log::log_event(
                                "trading_enable",
                                remote.as_ref(),
                                client_subj.as_deref(),
                                token_ok,
                                authorized,
                                None,
                            );
                            Ok::<_, warp::Rejection>(warp::reply::with_status(
                                warp::reply::json(
                                    &serde_json::json!({"status":"ok","trading_enabled":true}),
                                ),
                                StatusCode::OK,
                            ))
                        }
                    },
                );
            let engine_ref_trading_disable = Arc::clone(&engine_ref_outer);
            let allowed_subjects_disable = allowed_subjects.clone();
            let admin_token_arc_disable = Arc::clone(&admin_token_arc);
            let trading_disable = warp::path!("admin" / "trading" / "disable")
                .and(warp::post())
                .and(warp::addr::remote())
                .and(warp::header::optional::<String>("x-admin-token"))
                .and(warp::header::optional::<String>("x-client-subject"))
                .and_then(
                    move |remote: Option<SocketAddr>,
                          provided: Option<String>,
                          client_subj: Option<String>| {
                        let engine_clone = Arc::clone(&engine_ref_trading_disable);
                        let expected = (*admin_token_arc_disable).clone();
                        let allowlist = allowed_subjects_disable.clone();
                        async move {
                            engine_clone.metrics().admin_requests_total.inc();
                            let subject_ok = if let Some(list) = &allowlist {
                                match client_subj.as_ref().map(|s| s.trim()) {
                                    Some(subj) => list.contains(subj),
                                    None => false,
                                }
                            } else {
                                true
                            };
                            let token_ok = match (&expected, &provided) {
                                (Some(exp), Some(got)) if got == exp => true,
                                (Some(_), _) => false,
                                (None, _) => true,
                            };
                            let authorized = if expected.is_some() && allowlist.is_some() {
                                subject_ok && token_ok
                            } else if expected.is_some() {
                                token_ok
                            } else if allowlist.is_some() {
                                subject_ok
                            } else {
                                true
                            };
                            if !authorized {
                                engine_clone.metrics().admin_unauthorized_total.inc();
                                return Ok::<_, warp::Rejection>(warp::reply::with_status(
                                    warp::reply::json(
                                        &serde_json::json!({"status":"unauthorized"}),
                                    ),
                                    StatusCode::UNAUTHORIZED,
                                ));
                            }
                            engine_clone.disable_live_trading();
                            log::info!(
                                "audit_event action=trading_disable remote={:?} subject={:?}",
                                remote,
                                client_subj
                            );
                            crate::audit_log::log_event(
                                "trading_disable",
                                remote.as_ref(),
                                client_subj.as_deref(),
                                token_ok,
                                authorized,
                                None,
                            );
                            Ok::<_, warp::Rejection>(warp::reply::with_status(
                                warp::reply::json(
                                    &serde_json::json!({"status":"ok","trading_enabled":false}),
                                ),
                                StatusCode::OK,
                            ))
                        }
                    },
                );
            let route = get_token
                .or(refresh)
                .or(trading_status)
                .or(trading_enable)
                .or(trading_disable);
            let port = std::env::var("TOKEN_REGISTRY_PORT")
                .ok()
                .and_then(|v| v.parse::<u16>().ok())
                .unwrap_or(9898);
            let addr: SocketAddr = format!("0.0.0.0:{}", port).parse().unwrap();
            #[cfg(feature = "admin-mtls")]
            if enable_mtls_env {
                use rustls::{RootCertStore, ServerConfig};
                use std::{fs::File, io::BufReader};
                use tokio::sync::{oneshot, watch};
                use tokio_rustls::TlsAcceptor;

                // channel for dynamic config reload
                // We'll load real config right away so channel starts valid
                let initial_cfg = load_server_config();
                let (cfg_tx, mut cfg_rx) = watch::channel::<Arc<ServerConfig>>(initial_cfg);
                // shutdown signal
                let (shutdown_tx, mut shutdown_rx) = oneshot::channel::<()>();
                // store sender for later graceful shutdown
                let global_shutdown_tx = shutdown_tx;
                // put in a global OnceLock so ctrl-c handler can trigger it
                static ADMIN_SHUTDOWN: std::sync::OnceLock<
                    tokio::sync::Mutex<Option<oneshot::Sender<()>>>,
                > = std::sync::OnceLock::new();
                ADMIN_SHUTDOWN.get_or_init(|| tokio::sync::Mutex::new(Some(global_shutdown_tx)));

                // helper to load config
                fn load_server_config() -> Arc<ServerConfig> {
                    let cert_path = std::env::var("ADMIN_TLS_CERT").expect("ADMIN_TLS_CERT");
                    let key_path = std::env::var("ADMIN_TLS_KEY").expect("ADMIN_TLS_KEY");
                    let ca_path = std::env::var("ADMIN_TLS_CA").expect("ADMIN_TLS_CA");
                    let mut cert_rd = BufReader::new(File::open(cert_path).expect("open cert"));
                    let certs = rustls_pemfile::certs(&mut cert_rd)
                        .filter_map(|c| c.ok())
                        .collect::<Vec<_>>();
                    let mut key_rd = BufReader::new(File::open(key_path).expect("open key"));
                    let key = rustls_pemfile::pkcs8_private_keys(&mut key_rd)
                        .next()
                        .expect("key")
                        .expect("key parse");
                    let mut roots = RootCertStore::empty();
                    let mut ca_rd = BufReader::new(File::open(ca_path).expect("open ca"));
                    for c in rustls_pemfile::certs(&mut ca_rd) {
                        if let Ok(b) = c {
                            let _ = roots.add(rustls::pki_types::CertificateDer::from(b));
                        }
                    }
                    let client_verifier =
                        rustls::server::WebPkiClientVerifier::builder(roots.into())
                            .build()
                            .expect("client verifier");
                    let sc = ServerConfig::builder()
                        .with_client_cert_verifier(client_verifier)
                        .with_single_cert(
                            certs
                                .into_iter()
                                .map(rustls::pki_types::CertificateDer::from)
                                .collect(),
                            rustls::pki_types::PrivateKeyDer::Pkcs8(key.into()),
                        )
                        .expect("tls");
                    Arc::new(sc)
                }
                // initial load already performed

                // spawn SIGHUP listener to reload TLS config
                #[cfg(unix)]
                {
                    let cfg_tx2 = cfg_tx.clone();
                    tokio::spawn(async move {
                        use tokio::signal::unix::{signal, SignalKind};
                        if let Ok(mut hup) = signal(SignalKind::hangup()) {
                            while hup.recv().await.is_some() {
                                match std::panic::catch_unwind(|| load_server_config()) {
                                    Ok(cfg) => {
                                        let _ = cfg_tx2.send(cfg);
                                        log::info!("mtls_config_reloaded");
                                    }
                                    Err(_) => log::warn!("mtls_config_reload_panic"),
                                }
                            }
                        }
                    });
                }

                let make_svc = warp::service(route);
                let incoming = tokio::net::TcpListener::bind(addr).await.expect("bind");
                log::info!(
                    "admin_https_mtls_listen port={} addr={} (reloadable)",
                    port,
                    addr
                );
                tokio::spawn(async move {
                    loop {
                        tokio::select! {
                            biased;
                            _ = &mut shutdown_rx => { break; }
                            accept_res = incoming.accept() => {
                                let (tcp, peer) = match accept_res { Ok(v)=>v, Err(e)=> { log::warn!("accept_error err={}" , e); continue; } };
                                let svc = make_svc.clone();
                                let mut cfg_watch = cfg_rx.clone();
                                tokio::spawn(async move {
                                    let current_cfg = cfg_watch.borrow().clone();
                                    let acceptor = TlsAcceptor::from(current_cfg);
                                    if let Ok(tls_stream) = acceptor.accept(tcp).await {
                                        let peer_cert = tls_stream.get_ref().1.peer_certificates().and_then(|v| v.first().cloned());
                                        let mut client_cn: Option<String> = None;
                                        if let Some(cert) = peer_cert {
                                            if let Ok((_, parsed)) = x509_parser::parse_x509_certificate(cert.as_ref()) {
                                                if let Some(cn) = parsed.subject().iter_common_name().next().and_then(|cn| cn.as_str().ok()) {
                                                    client_cn = Some(cn.to_string());
                                                    log::info!("mtls_client subject_cn='{}' peer='{}'", cn, peer);
                                                }
                                            }
                                        }
                                        // Inject CN via header wrapper by spawning hyper service manually
                                        use hyper::service::Service;
                                        let http = hyper::server::conn::http1::Builder::new();
                                        let inner = svc.clone();
                                        let wrapped = hyper::service::service_fn(move |mut req: hyper::Request<hyper::Body>| {
                                            let mut inner = inner.clone();
                                            if let Some(cn_val) = client_cn.clone() { req.headers_mut().insert("x-client-subject", hyper::header::HeaderValue::from_str(&cn_val).unwrap_or_else(|_| hyper::header::HeaderValue::from_static("invalid"))); }
                                            inner.call(req)
                                        });
                                        let _ = http.serve_connection(tls_stream, wrapped).await;
                                    } else {
                                        log::warn!("mtls_handshake_failed peer={:?}", peer);
                                    }
                                });
                            }
                        }
                    }
                    log::info!("admin_mtls_accept_loop_shutdown");
                });
            } else {
                log::info!("admin_http_listen port={} addr={} (no mTLS)", port, addr);
                warp::serve(route).run(addr).await;
            }
            #[cfg(not(feature = "admin-mtls"))]
            {
                log::info!(
                    "admin_http_listen port={} addr={} (feature admin-mtls disabled)",
                    port,
                    addr
                );
                warp::serve(route).run(addr).await;
            }
        });
    }

    // Cooperative shutdown broadcast channel
    let (shutdown_tx, mut shutdown_rx) = tokio::sync::broadcast::channel::<()>(4);
    let engine_handle = {
        let engine = Arc::clone(&execution_engine);
        let mut rx = shutdown_tx.subscribe();
        tokio::spawn(async move {
            tokio::select! {
                _ = engine.start() => {},
                _ = rx.recv() => { log::info!("engine_shutdown_signal_received"); }
            }
        })
    };

    if !disable_db {
        // registry persist loop
        let engine = Arc::clone(&execution_engine);
        tokio::spawn(async move {
            let interval = std::env::var("REGISTRY_PERSIST_INTERVAL_SECS")
                .ok()
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(900);
            loop {
                if let Err(e) = engine.persist_registry_state().await {
                    log::warn!("registry_persist_loop_failed err={}", e);
                } else {
                    log::info!("registry_persisted interval_secs={}", interval);
                }
                tokio::time::sleep(std::time::Duration::from_secs(interval)).await;
            }
        });
    }

    if !disable_db {
        // hash chain checkpoint
        let db = pg_pool.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                let (hash, lines) = global_logger().checkpoint();
                if lines == 0 {
                    continue;
                }
                let _ = sqlx::query("INSERT INTO hash_chain_checkpoints (chain_hash, lines_aggregated) VALUES ($1,$2)").bind(&hash).bind(lines as i32).execute(&db).await;
            }
        });
    }

    if !disable_db {
        // PnL heartbeat hourly
        let db = pg_pool.clone();
        tokio::spawn(async move {
            loop {
                let day = chrono::Utc::now().date_naive();
                if let Err(e) = sqlx::query("SELECT upsert_daily_pnl($1,$2)")
                    .bind(day)
                    .bind(0i64)
                    .execute(&db)
                    .await
                {
                    log::warn!("daily_pnl_heartbeat_failed err={}", e);
                }
                tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
            }
        });
    }

    #[cfg(unix)]
    {
        // SIGHUP reload placeholder
        let _eng = Arc::clone(&execution_engine);
        tokio::spawn(async move {
            use tokio::signal::unix::{signal, SignalKind};
            if let Ok(mut hup) = signal(SignalKind::hangup()) {
                while hup.recv().await.is_some() {
                    log::info!("sighup_received: reload placeholder");
                }
            }
        });
    }

    info!("Executor service running. Press Ctrl+C to stop.");
    // Mark readiness once core services initialized (DB & engine constructed)
    readiness::set_ready();
    signal::ctrl_c().await?;
    info!("Shutdown signal received");
    // attempt graceful shutdown of admin mTLS loop (if enabled) before aborting engine
    #[cfg(feature = "admin-mtls")]
    if let Some(lock) = {
        use std::sync::OnceLock;
        extern crate tokio as _tokio; // ensure tokio in scope
        static ADMIN_SHUTDOWN: OnceLock<
            tokio::sync::Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
        > = OnceLock::new();
        ADMIN_SHUTDOWN.get()
    } {
        if let Some(tx) = lock.lock().await.take() {
            let _ = tx.send(());
        }
    }
    let _ = shutdown_tx.send(()); // signal cooperative shutdown
                                  // Wait briefly for tasks
    if tokio::time::timeout(std::time::Duration::from_secs(3), async {
        let _ = &mut shutdown_rx;
    })
    .await
    .is_err()
    {
        log::warn!("shutdown_timeout_engine_forcing_abort");
        engine_handle.abort();
    }
    if !disable_db {
        if let Err(e) = execution_engine.persist_registry_state().await {
            log::warn!("registry_persist_flush_failed err={}", e);
        }
    }
    info!("Executor service stopped");
    Ok(())
}
