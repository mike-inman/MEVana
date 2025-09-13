use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use once_cell::sync::Lazy;

static AUDIT_WRITER: Lazy<Mutex<Option<BufWriter<File>>>> = Lazy::new(|| Mutex::new(None));

fn init_if_needed() {
    let mut guard = AUDIT_WRITER.lock().unwrap();
    if guard.is_some() {
        return;
    }
    let path = std::env::var("AUDIT_LOG_PATH").unwrap_or_else(|_| "executor_audit.log".into());
    match OpenOptions::new().create(true).append(true).open(&path) {
        Ok(f) => {
            *guard = Some(BufWriter::new(f));
            log::info!("audit_log_initialized path={}", path);
        }
        Err(e) => {
            log::warn!("audit_log_init_failed path={} err={}", path, e);
        }
    }
}

pub fn log_event(
    action: &str,
    remote: Option<&std::net::SocketAddr>,
    subject: Option<&str>,
    token_ok: bool,
    authorized: bool,
    extra: Option<&serde_json::Value>,
) {
    init_if_needed();
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64();
    let mut obj = serde_json::json!({
        "ts": ts,
        "action": action,
        "remote": remote.map(|r| r.to_string()),
        "subject": subject,
        "token_ok": token_ok,
        "authorized": authorized,
    });
    if let Some(extra_v) = extra {
        if let Some(map) = obj.as_object_mut() {
            if let Some(extra_map) = extra_v.as_object() {
                for (k, v) in extra_map {
                    map.insert(k.clone(), v.clone());
                }
            }
        }
    }
    if let Ok(line) = serde_json::to_string(&obj) {
        if let Some(writer) = AUDIT_WRITER.lock().unwrap().as_mut() {
            let _ = writeln!(writer, "{}", line);
            let _ = writer.flush();
        }
    }
}
