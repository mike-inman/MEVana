use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

fn readiness_flag() -> &'static AtomicBool {
    static FLAG: OnceLock<AtomicBool> = OnceLock::new();
    FLAG.get_or_init(|| AtomicBool::new(false))
}

pub fn set_ready() {
    readiness_flag().store(true, Ordering::SeqCst);
    log::info!("readiness_state changed=ready");
}

#[allow(dead_code)]
pub fn is_ready() -> bool {
    readiness_flag().load(Ordering::SeqCst)
}
