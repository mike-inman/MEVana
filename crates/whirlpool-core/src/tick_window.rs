use crate::pda::actual_tick_index;
use crate::types::tick_is_initialized;
use crate::types::{Tick, TickArray};

#[derive(Clone, Debug)]
pub struct TickArrayView<'a> {
    pub start_tick_index: i32,
    pub ticks: &'a [Tick; 88],
}

#[derive(Clone, Debug, Default)]
pub struct TickWindow<'a> {
    pub lower: Option<TickArrayView<'a>>,
    pub current: Option<TickArrayView<'a>>,
    pub upper: Option<TickArrayView<'a>>,
}

fn floor_div_i32(x: i32, y: i32) -> i32 {
    let mut q = x / y;
    if (x ^ y) < 0 && x % y != 0 {
        q -= 1;
    }
    q
}
fn ceil_div_i32(x: i32, y: i32) -> i32 {
    let mut q = x / y;
    if (x ^ y) >= 0 && x % y != 0 {
        q += 1;
    }
    q
}

pub fn find_next_lower_initialized_tick(
    window: &TickWindow,
    tick_current: i32,
    tick_spacing: i32,
) -> Option<i32> {
    let cur = window.current.as_ref()?;
    let off = floor_div_i32(tick_current - cur.start_tick_index, tick_spacing).clamp(0, 87);
    for i in (0..=off as usize).rev() {
        let t = &cur.ticks[i];
        if tick_is_initialized(t) {
            return Some(actual_tick_index(cur.start_tick_index, i, tick_spacing));
        }
    }
    if let Some(low) = window.lower.as_ref() {
        for i in (0..88).rev() {
            if tick_is_initialized(&low.ticks[i]) {
                return Some(actual_tick_index(low.start_tick_index, i, tick_spacing));
            }
        }
    }
    None
}

pub fn find_next_upper_initialized_tick(
    window: &TickWindow,
    tick_current: i32,
    tick_spacing: i32,
) -> Option<i32> {
    let cur = window.current.as_ref()?;
    let off = ceil_div_i32(tick_current + 1 - cur.start_tick_index, tick_spacing).clamp(0, 88);
    for i in off as usize..88 {
        if tick_is_initialized(&cur.ticks[i]) {
            return Some(actual_tick_index(cur.start_tick_index, i, tick_spacing));
        }
    }
    if let Some(up) = window.upper.as_ref() {
        for i in 0..88 {
            if tick_is_initialized(&up.ticks[i]) {
                return Some(actual_tick_index(up.start_tick_index, i, tick_spacing));
            }
        }
    }
    None
}

pub fn find_nearest_initialized_ticks(
    window: &TickWindow,
    tick_current: i32,
    tick_spacing: i32,
) -> (Option<i32>, Option<i32>) {
    (
        find_next_lower_initialized_tick(window, tick_current, tick_spacing),
        find_next_upper_initialized_tick(window, tick_current, tick_spacing),
    )
}
