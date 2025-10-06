use std::{
    collections::VecDeque,
    time::Instant,
};

use super::SwapSummary;

const TARGET_SLOT_INTERVAL_MS: f64 = 400.0;
const MAX_SAMPLES: usize = 120;

pub struct NetworkTelemetry {
    block_time_intervals_ms: VecDeque<f64>,
    instant_intervals_ms: VecDeque<f64>,
    slot_intervals: VecDeque<f64>,
    last_slot: Option<u64>,
    last_block_time: Option<i64>,
    last_wallclock: Option<Instant>,
}

impl NetworkTelemetry {
    pub fn new() -> Self {
        Self {
            block_time_intervals_ms: VecDeque::with_capacity(MAX_SAMPLES),
            instant_intervals_ms: VecDeque::with_capacity(MAX_SAMPLES),
            slot_intervals: VecDeque::with_capacity(MAX_SAMPLES),
            last_slot: None,
            last_block_time: None,
            last_wallclock: None,
        }
    }
}

impl Default for NetworkTelemetry {
    fn default() -> Self {
        Self::new()
    }
}

impl NetworkTelemetry {
    pub fn record(&mut self, summary: Option<&SwapSummary>, slot: u64) {
        if let Some(previous_slot) = self.last_slot {
            let slot_delta = slot.saturating_sub(previous_slot) as f64;
            if slot_delta > 0.0 {
                push_with_limit(&mut self.slot_intervals, slot_delta);
            }
        }
        self.last_slot = Some(slot);

        if let Some(summary) = summary {
            if let Some(block_time) = summary.block_time_unix {
                if let Some(previous_block_time) = self.last_block_time {
                    let delta = (block_time - previous_block_time) as f64 * 1000.0;
                    if delta.is_finite() && delta > 0.0 {
                        push_with_limit(&mut self.block_time_intervals_ms, delta);
                    }
                }
                self.last_block_time = Some(block_time);
                self.last_wallclock = None; // prefer chain-derived timing when available
                return;
            }
        }

        let now = Instant::now();
        if let Some(previous) = self.last_wallclock {
            let delta = now.duration_since(previous).as_secs_f64() * 1000.0;
            if delta.is_finite() && delta > 0.0 {
                push_with_limit(&mut self.instant_intervals_ms, delta);
            }
        }
        self.last_wallclock = Some(now);
    }

    pub fn congestion_score(&self) -> f64 {
        let average_ms = if !self.block_time_intervals_ms.is_empty() {
            mean(&self.block_time_intervals_ms)
        } else if !self.instant_intervals_ms.is_empty() {
            mean(&self.instant_intervals_ms)
        } else if !self.slot_intervals.is_empty() {
            mean(&self.slot_intervals) * TARGET_SLOT_INTERVAL_MS
        } else {
            return 0.4; // neutral when no data yet
        };

        let ratio = average_ms / TARGET_SLOT_INTERVAL_MS;
        if ratio <= 0.9 {
            0.2
        } else if ratio <= 1.2 {
            0.4
        } else if ratio <= 1.6 {
            0.6
        } else if ratio <= 2.0 {
            0.8
        } else {
            1.0
        }
    }

}

fn push_with_limit(deque: &mut VecDeque<f64>, value: f64) {
    if deque.len() >= MAX_SAMPLES {
        deque.pop_front();
    }
    deque.push_back(value);
}

fn mean(values: &VecDeque<f64>) -> f64 {
    let sum: f64 = values.iter().sum();
    sum / values.len() as f64
}
