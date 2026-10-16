use crate::models::ArbSignal;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

pub const MAX_RECENT: usize = 256;
pub const MAX_ADVICE: usize = 100;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AdviceEntry {
    pub id: u64,
    pub tag: String,
    pub message: String,
    pub ts_ms: u64,
}

static RECENT_SIGNALS: OnceLock<Mutex<VecDeque<ArbSignal>>> = OnceLock::new();
static ADVICE_LOG: OnceLock<Mutex<VecDeque<AdviceEntry>>> = OnceLock::new();
static ADVICE_ID: OnceLock<std::sync::atomic::AtomicU64> = OnceLock::new();
static SIGNAL_LOG_FILE: OnceLock<Mutex<Option<File>>> = OnceLock::new();
static ADVICE_LOG_FILE: OnceLock<Mutex<Option<File>>> = OnceLock::new();
// Keep each write and its subsequent rotation together after releasing the file lock.
static LOG_WRITE_LOCK: Mutex<()> = Mutex::new(());
// Track current log date + size for cheap rotation checks
static LOG_STATE: OnceLock<Mutex<LogState>> = OnceLock::new();

#[derive(Debug, Clone)]
struct LogState {
    date_ymd: String,
    signals_bytes: u64,
    advice_bytes: u64,
}

fn recent() -> &'static Mutex<VecDeque<ArbSignal>> {
    RECENT_SIGNALS.get_or_init(|| Mutex::new(VecDeque::with_capacity(MAX_RECENT)))
}
fn advice() -> &'static Mutex<VecDeque<AdviceEntry>> {
    ADVICE_LOG.get_or_init(|| Mutex::new(VecDeque::with_capacity(MAX_ADVICE)))
}
fn advice_id() -> &'static std::sync::atomic::AtomicU64 {
    ADVICE_ID.get_or_init(|| std::sync::atomic::AtomicU64::new(1))
}

fn init_file(path: &str) -> Option<File> {
    OpenOptions::new().create(true).append(true).open(path).ok()
}
fn ensure_logs_init() {
    SIGNAL_LOG_FILE.get_or_init(|| {
        let dir = std::env::var("AI_LOG_DIR").unwrap_or_else(|_| "./ai_logs".into());
        std::fs::create_dir_all(&dir).ok();
        let mut p = PathBuf::from(&dir);
        p.push("signals.jsonl");
        Mutex::new(init_file(p.to_string_lossy().as_ref()))
    });
    ADVICE_LOG_FILE.get_or_init(|| {
        let dir = std::env::var("AI_LOG_DIR").unwrap_or_else(|_| "./ai_logs".into());
        std::fs::create_dir_all(&dir).ok();
        let mut p = PathBuf::from(&dir);
        p.push("advice.jsonl");
        Mutex::new(init_file(p.to_string_lossy().as_ref()))
    });
    LOG_STATE.get_or_init(|| {
        Mutex::new(LogState {
            date_ymd: current_date(),
            signals_bytes: 0,
            advice_bytes: 0,
        })
    });
}

fn current_date() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    // Simple UTC day derivation
    let days = now / 86_400; // days since epoch
                             // Convert days back to yyyy-mm-dd via chrono if available; keep lightweight by manual calc (approx) or feature gate.
                             // To avoid dependency, we just store day index; rotate when value changes.
    format!("d{}", days)
}

fn rotate_if_needed(kind: &str, added: usize) {
    let max_bytes: u64 = std::env::var("AI_LOG_MAX_BYTES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(10 * 1024 * 1024); // 10MB
    let max_files: usize = std::env::var("AI_LOG_MAX_ROTATIONS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(5);
    if let Some(state_lock) = LOG_STATE.get() {
        let mut rotate_flag = false;
        let mut fname = String::new();
        let mut is_signals = false;
        {
            let mut st = state_lock.lock().unwrap();
            let cur_date = current_date();
            let date_changed = cur_date != st.date_ymd;
            if kind == "signals" {
                st.signals_bytes += added as u64;
                if date_changed || st.signals_bytes >= max_bytes {
                    rotate_flag = true;
                    st.date_ymd = cur_date;
                    st.signals_bytes = 0;
                    fname = "signals.jsonl".into();
                    is_signals = true;
                }
            } else {
                st.advice_bytes += added as u64;
                if date_changed || st.advice_bytes >= max_bytes {
                    rotate_flag = true;
                    st.date_ymd = cur_date;
                    st.advice_bytes = 0;
                    fname = "advice.jsonl".into();
                }
            }
        }
        if !rotate_flag {
            return;
        }
        let dir = std::env::var("AI_LOG_DIR").unwrap_or_else(|_| "./ai_logs".into());
        for i in (1..max_files).rev() {
            let from = format!("{}/{}.{}", dir, fname, i);
            let to = format!("{}/{}.{}", dir, fname, i + 1);
            if std::path::Path::new(&from).exists() {
                let _ = std::fs::rename(&from, &to);
            }
        }
        let cur = format!("{}/{}", dir, fname);
        if std::path::Path::new(&cur).exists() {
            let _ = std::fs::rename(&cur, format!("{}/{}.1", dir, fname));
        }
        let new_file = init_file(&cur);
        if is_signals {
            if let Some(lock) = SIGNAL_LOG_FILE.get() {
                if let Ok(mut opt) = lock.lock() {
                    *opt = new_file;
                }
            }
        } else {
            if let Some(lock) = ADVICE_LOG_FILE.get() {
                if let Ok(mut opt) = lock.lock() {
                    *opt = new_file;
                }
            }
        }
    }
}

pub fn record_recent_signal(sig: ArbSignal) {
    ensure_logs_init();
    let mut q = recent().lock().unwrap();
    if q.len() == MAX_RECENT {
        q.pop_front();
    }
    q.push_back(sig.clone());
    // Persist
    let _write_guard = LOG_WRITE_LOCK.lock().unwrap();
    let mut written_bytes = None;
    if let Some(lock) = SIGNAL_LOG_FILE.get() {
        if let Ok(mut fopt) = lock.lock() {
            if let Some(f) = fopt.as_mut() {
                if let Ok(line) = serde_json::to_string(&sig) {
                    if writeln!(f, "{}", line).is_ok() && f.flush().is_ok() {
                        written_bytes = Some(line.len() + 1);
                    }
                }
            }
        }
    }
    if let Some(bytes) = written_bytes {
        rotate_if_needed("signals", bytes);
    }
}

pub fn list_recent(limit: usize) -> Vec<ArbSignal> {
    let q = recent().lock().unwrap();
    let len = q.len();
    let take = limit.min(len);
    q.iter().rev().take(take).cloned().collect()
}

pub fn add_advice(tag: String, message: String) -> AdviceEntry {
    ensure_logs_init();
    use std::time::{SystemTime, UNIX_EPOCH};
    let id = advice_id().fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let ts_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    let entry = AdviceEntry {
        id,
        tag,
        message,
        ts_ms,
    };
    let mut q = advice().lock().unwrap();
    if q.len() == MAX_ADVICE {
        q.pop_front();
    }
    q.push_back(entry.clone());
    let _write_guard = LOG_WRITE_LOCK.lock().unwrap();
    let mut written_bytes = None;
    if let Some(lock) = ADVICE_LOG_FILE.get() {
        if let Ok(mut fopt) = lock.lock() {
            if let Some(f) = fopt.as_mut() {
                if let Ok(line) = serde_json::to_string(&entry) {
                    if writeln!(f, "{}", line).is_ok() && f.flush().is_ok() {
                        written_bytes = Some(line.len() + 1);
                    }
                }
            }
        }
    }
    if let Some(bytes) = written_bytes {
        rotate_if_needed("advice", bytes);
    }
    entry
}

pub fn list_advice(limit: usize) -> Vec<AdviceEntry> {
    let q = advice().lock().unwrap();
    let len = q.len();
    let take = limit.min(len);
    q.iter().rev().take(take).cloned().collect()
}
