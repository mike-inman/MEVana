use crate::plugin::{FlashLoanProvider, ProviderId};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

static REGISTRY: Lazy<RwLock<HashMap<String, Arc<dyn FlashLoanProvider>>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

pub fn register_provider(provider: Arc<dyn FlashLoanProvider>) -> bool {
    let id = provider.id();
    let mut guard = REGISTRY
        .write()
        .expect("flash loan provider registry poisoned");
    guard.insert(id.as_str().to_string(), provider).is_none()
}

pub fn get_provider(id: &ProviderId) -> Option<Arc<dyn FlashLoanProvider>> {
    let guard = REGISTRY
        .read()
        .expect("flash loan provider registry poisoned");
    guard.get(id.as_str()).cloned()
}

pub fn providers() -> Vec<Arc<dyn FlashLoanProvider>> {
    let guard = REGISTRY
        .read()
        .expect("flash loan provider registry poisoned");
    guard.values().cloned().collect()
}

pub fn has_provider(id: &ProviderId) -> bool {
    let guard = REGISTRY
        .read()
        .expect("flash loan provider registry poisoned");
    guard.contains_key(id.as_str())
}

#[cfg(test)]
pub fn clear_registry() {
    let mut guard = REGISTRY
        .write()
        .expect("flash loan provider registry poisoned");
    guard.clear();
}
