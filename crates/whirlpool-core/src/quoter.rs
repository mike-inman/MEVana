use crate::math::{
    apply_input_fee, dx_raw_within_tick_ab, dy_raw_within_tick_ba, tick_to_sqrt_price_x64_approx,
    Q64_ONE,
};
use crate::tick_window::find_nearest_initialized_ticks;
use crate::tick_window::TickWindow;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Direction {
    AtoB,
    BtoA,
}

#[derive(Clone, Debug, Default)]
pub struct QuoteResult {
    pub out: u128,
    pub dx_consumed: u128,
    pub ticks_crossed: u32,
}

pub struct ClmmQuoter<'a> {
    pub sqrt_price_x64: u128,
    pub liquidity: u128,
    pub tick_current: i32,
    pub tick_spacing: i32,
    pub total_fee_bps: u16,
    pub window: TickWindow<'a>,
}

impl<'a> ClmmQuoter<'a> {
    // Conservative "size to next tick" in input units
    pub fn size_to_next_tick_in(&self, dir: Direction) -> Option<u128> {
        let (lower_opt, upper_opt) =
            find_nearest_initialized_ticks(&self.window, self.tick_current, self.tick_spacing);
        match dir {
            Direction::AtoB => {
                let next_lower = lower_opt?;
                let sp = self.sqrt_price_x64;
                let sp_next = tick_to_sqrt_price_x64_approx(next_lower);
                if sp_next == 0 || sp_next >= sp {
                    return None;
                }
                let dx_raw = dx_raw_within_tick_ab(sp, sp_next, self.liquidity);
                Some(apply_input_fee(dx_raw, self.total_fee_bps))
            }
            Direction::BtoA => {
                let next_upper = upper_opt?;
                let sp = self.sqrt_price_x64;
                let sp_next = tick_to_sqrt_price_x64_approx(next_upper);
                if sp_next <= sp {
                    return None;
                }
                let dx_raw = dy_raw_within_tick_ba(sp, sp_next, self.liquidity);
                Some(apply_input_fee(dx_raw, self.total_fee_bps))
            }
        }
    }

    // Full quoter: walk across multiple ticks until dx_in is consumed or max ticks reached
    pub fn quote(&self, dir: Direction, dx_in: u128, max_ticks: u32) -> QuoteResult {
        if dx_in == 0 {
            return QuoteResult::default();
        }

        let mut result = QuoteResult::default();
        let mut remaining_dx = dx_in;
        let mut current_tick = self.tick_current;
        let mut current_sqrt_price = self.sqrt_price_x64;
        let current_liquidity = self.liquidity;

        for _ in 0..max_ticks {
            if remaining_dx == 0 {
                break;
            }

            let boundary_result = self.quote_to_next_boundary_internal(
                dir,
                remaining_dx,
                current_tick,
                current_sqrt_price,
                current_liquidity,
            );

            result.out += boundary_result.out;
            result.dx_consumed += boundary_result.dx_consumed;
            result.ticks_crossed += boundary_result.ticks_crossed;

            remaining_dx = remaining_dx.saturating_sub(boundary_result.dx_consumed);

            // If we didn't reach the boundary, we're done
            if boundary_result.ticks_crossed == 0 {
                break;
            }

            // Move to next tick and update state
            match dir {
                Direction::AtoB => {
                    if let Some(next_tick) = self.find_next_initialized_tick(current_tick, dir) {
                        current_tick = next_tick;
                        current_sqrt_price = tick_to_sqrt_price_x64_approx(next_tick);
                        // Liquidity update would happen here in full implementation
                    } else {
                        break;
                    }
                }
                Direction::BtoA => {
                    if let Some(next_tick) = self.find_next_initialized_tick(current_tick, dir) {
                        current_tick = next_tick;
                        current_sqrt_price = tick_to_sqrt_price_x64_approx(next_tick);
                        // Liquidity update would happen here in full implementation
                    } else {
                        break;
                    }
                }
            }
        }

        result
    }

    // Helper to find next initialized tick in given direction
    fn find_next_initialized_tick(&self, current: i32, dir: Direction) -> Option<i32> {
        let (lower_opt, upper_opt) =
            find_nearest_initialized_ticks(&self.window, current, self.tick_spacing);
        match dir {
            Direction::AtoB => lower_opt,
            Direction::BtoA => upper_opt,
        }
    }

    // Original single-boundary quoter (now internal)
    fn quote_to_next_boundary_internal(
        &self,
        dir: Direction,
        dx_in: u128,
        _current_tick: i32,
        current_sqrt_price: u128,
        current_liquidity: u128,
    ) -> QuoteResult {
        if dx_in == 0 {
            return QuoteResult::default();
        }
        let fee_denom = 10_000u64;
        let fee_num = (fee_denom - self.total_fee_bps as u64) as u128;
        // effective post-fee input (currently only used in clipped forms below)

        let sp = current_sqrt_price;
        let (lower_opt, upper_opt) =
            find_nearest_initialized_ticks(&self.window, self.tick_current, self.tick_spacing);

        match dir {
            Direction::AtoB => {
                let next_lower = match lower_opt {
                    Some(t) => t,
                    None => return QuoteResult::default(),
                };
                let sp_next = tick_to_sqrt_price_x64_approx(next_lower);
                if sp_next == 0 || sp_next >= sp {
                    return QuoteResult::default();
                }

                // Max dx_raw to reach boundary
                let max_dx_raw = dx_raw_within_tick_ab(sp, sp_next, current_liquidity);
                // Re-map to input including fee (ceil)
                let max_dx_in = apply_input_fee(max_dx_raw, self.total_fee_bps);
                let dx_in_clip = dx_in.min(max_dx_in);

                // Recompute effective dx_raw from clipped dx_in
                let dx_eff_clip = (dx_in_clip as u128) * fee_num / (fee_denom as u128);
                // Within a tick: dy = L * (sp - sp_new), where sp_new solves dx_raw.
                // For small steps, linear approx: dy ≈ L * (sp - sp_next) * (dx_eff_clip / max_dx_raw)
                // Good enough for prefiltering; we’ll replace with exact step solver later.
                let dy_raw = if max_dx_raw > 0 {
                    (current_liquidity as u128)
                        .saturating_mul(sp.saturating_sub(sp_next))
                        .saturating_mul(dx_eff_clip)
                        / max_dx_raw
                } else {
                    0
                };

                QuoteResult {
                    out: dy_raw,
                    dx_consumed: dx_in_clip,
                    ticks_crossed: if dx_in_clip == max_dx_in { 1 } else { 0 },
                }
            }
            Direction::BtoA => {
                let next_upper = match upper_opt {
                    Some(t) => t,
                    None => return QuoteResult::default(),
                };
                let sp_next = tick_to_sqrt_price_x64_approx(next_upper);
                if sp_next <= sp {
                    return QuoteResult::default();
                }

                let max_dy_raw = dy_raw_within_tick_ba(sp, sp_next, current_liquidity);
                let max_dx_in = apply_input_fee(max_dy_raw, self.total_fee_bps);
                let dx_in_clip = dx_in.min(max_dx_in);

                let dx_eff_clip = (dx_in_clip as u128) * fee_num / (fee_denom as u128);
                // Within tick B->A: out in token A ≈ L * (sp_next - sp) * (dx_eff_clip / max_dy_raw) / (sp * sp_next)
                // For now use same linear fraction; exact solver will replace this.
                let num = (current_liquidity as u128)
                    .saturating_mul(sp_next.saturating_sub(sp))
                    .saturating_mul(dx_eff_clip);
                let dy_out = if max_dy_raw > 0 { num / max_dy_raw } else { 0 };

                QuoteResult {
                    out: dy_out,
                    dx_consumed: dx_in_clip,
                    ticks_crossed: if dx_in_clip == max_dx_in { 1 } else { 0 },
                }
            }
        }
    }

    // Keep original method for backward compatibility
    pub fn quote_to_next_boundary(&self, dir: Direction, dx_in: u128) -> QuoteResult {
        self.quote_to_next_boundary_internal(
            dir,
            dx_in,
            self.tick_current,
            self.sqrt_price_x64,
            self.liquidity,
        )
    }

    // Convenience method for single-tick quotes (most common case)
    pub fn quote_single_tick(&self, dir: Direction, dx_in: u128) -> QuoteResult {
        self.quote(dir, dx_in, 1)
    }

    // Method for getting marginal price (useful for edge comparison)
    pub fn marginal_price_q64(&self, dir: Direction) -> u128 {
        match dir {
            Direction::AtoB => {
                // Price = (sqrt_price)^2 = sqrt_price_x64^2 / 2^128
                let sp = self.sqrt_price_x64;
                if sp == 0 {
                    return 0;
                }
                let sp_u256 = crate::math::u128_to_u256(sp);
                let price_u256 = sp_u256 * sp_u256 / crate::math::u128_to_u256(Q64_ONE);
                price_u256.as_u128()
            }
            Direction::BtoA => {
                // Inverse price = 1 / marginal_price_q64(AtoB)
                let forward_price = self.marginal_price_q64(Direction::AtoB);
                if forward_price == 0 {
                    return 0;
                }
                let q64_squared =
                    crate::math::u128_to_u256(Q64_ONE) * crate::math::u128_to_u256(Q64_ONE);
                (q64_squared / crate::math::u128_to_u256(forward_price)).as_u128()
            }
        }
    }
}
