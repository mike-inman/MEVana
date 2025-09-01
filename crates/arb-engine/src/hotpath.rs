use crossbeam_queue::ArrayQueue;
use std::sync::Arc;
use tokio::sync::Notify;

use crate::models::ArbSignal;

/// Single-producer (engine) → multi-consumer (executors) ring. Bounded to avoid blow-ups.
pub struct SignalQueue {
    q: Arc<ArrayQueue<ArbSignal>>,
    notify: Arc<Notify>,
}

impl SignalQueue {
    pub fn new(cap: usize) -> Self {
        Self {
            q: Arc::new(ArrayQueue::new(cap)),
            notify: Arc::new(Notify::new()),
        }
    }
    pub fn sender(&self) -> SignalTx {
        SignalTx {
            q: self.q.clone(),
            notify: self.notify.clone(),
        }
    }
    pub fn receiver(&self) -> SignalRx {
        SignalRx {
            q: self.q.clone(),
            notify: self.notify.clone(),
        }
    }
}

pub struct SignalTx {
    q: Arc<ArrayQueue<ArbSignal>>,
    notify: Arc<Notify>,
}
pub struct SignalRx {
    q: Arc<ArrayQueue<ArbSignal>>,
    notify: Arc<Notify>,
}

impl SignalTx {
    #[inline]
    pub fn try_send(&self, s: ArbSignal) -> bool {
        let ok = self.q.push(s).is_ok();
        if ok {
            self.notify.notify_waiters();
        }
        ok
    }

    /// Clone this SignalTx for sharing between components
    pub fn clone(&self) -> Self {
        SignalTx {
            q: self.q.clone(),
            notify: self.notify.clone(),
        }
    }
}

impl SignalRx {
    #[inline]
    pub fn try_recv(&self) -> Option<ArbSignal> {
        // OLD: self.q.pop().ok()
        self.q.pop()
    }

    pub async fn recv_async(&self) -> ArbSignal {
        loop {
            if let Some(s) = self.q.pop() {
                return s;
            }
            self.notify.notified().await;
        }
    }
}

// impl SignalRx {
//     #[inline]
//     pub fn try_recv(&self) -> Option<ArbSignal> {
//         self.q.pop().ok()
//     }
// }
