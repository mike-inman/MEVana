use once_cell::sync::Lazy;
use sha2::{Digest, Sha256};
use std::sync::Mutex;

#[derive(Clone, Default)]
pub struct HashChainLogger {
    state: std::sync::Arc<Mutex<HashChainState>>,
}

#[derive(Default)]
struct HashChainState {
    last_hash: Option<[u8; 32]>,
    pending: Vec<String>,
}

impl HashChainLogger {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn append(&self, line: &str) {
        let mut st = self.state.lock().unwrap();
        st.pending.push(line.to_string());
    }

    pub fn checkpoint(&self) -> (String, usize) {
        let mut st = self.state.lock().unwrap();
        if st.pending.is_empty() {
            return (st.last_hash.map(hex::encode).unwrap_or_default(), 0);
        }
        let mut hasher = Sha256::new();
        if let Some(prev) = st.last_hash {
            hasher.update(prev);
        }
        for line in &st.pending {
            hasher.update(line.as_bytes());
            hasher.update(b"\n");
        }
        let new_hash: [u8; 32] = hasher.finalize().into();
        st.last_hash = Some(new_hash);
        let count = st.pending.len();
        st.pending.clear();
        (hex::encode(new_hash), count)
    }
}

pub static GLOBAL_HASH_LOGGER: Lazy<HashChainLogger> = Lazy::new(HashChainLogger::new);

pub fn global_logger() -> &'static HashChainLogger {
    &GLOBAL_HASH_LOGGER
}
