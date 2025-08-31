use crossbeam_queue::ArrayQueue;
use std::sync::Arc;

use crate::models::ArbSignal;

/// Single-producer (engine) → multi-consumer (executors) ring. Bounded to avoid blow-ups.
pub struct SignalQueue {
    q: Arc<ArrayQueue<ArbSignal>>,
}

impl SignalQueue {
    pub fn new(cap: usize) -> Self {
        Self {
            q: Arc::new(ArrayQueue::new(cap)),
        }
    }
    pub fn sender(&self) -> SignalTx {
        SignalTx { q: self.q.clone() }
    }
    pub fn receiver(&self) -> SignalRx {
        SignalRx { q: self.q.clone() }
    }
}

pub struct SignalTx {
    q: Arc<ArrayQueue<ArbSignal>>,
}
pub struct SignalRx {
    q: Arc<ArrayQueue<ArbSignal>>,
}

impl SignalTx {
    #[inline]
    pub fn try_send(&self, s: ArbSignal) -> bool {
        self.q.push(s).is_ok()
    }
}

impl SignalRx {
    #[inline]
    pub fn try_recv(&self) -> Option<ArbSignal> {
        // OLD: self.q.pop().ok()
        self.q.pop()
    }
}

// impl SignalRx {
//     #[inline]
//     pub fn try_recv(&self) -> Option<ArbSignal> {
//         self.q.pop().ok()
//     }
// }
