use solana_sdk::pubkey::Pubkey;

pub fn derive_tick_array_address(
    program_id: &Pubkey,
    whirlpool: &Pubkey,
    start_tick_index: i32,
) -> (Pubkey, u8) {
    let le = start_tick_index.to_le_bytes();
    Pubkey::find_program_address(&[b"tick_array", whirlpool.as_ref(), &le], program_id)
}

pub fn three_tick_arrays_around(tick_current: i32, tick_spacing: i32) -> (i32, i32, i32) {
    let per_array = 88;
    let span = tick_spacing * per_array;
    let floor_div = |x: i32, y: i32| {
        let mut q = x / y;
        if (x ^ y) < 0 && x % y != 0 {
            q -= 1;
        }
        q
    };
    let current_start = floor_div(tick_current, span) * span;
    (current_start - span, current_start, current_start + span)
}

pub fn actual_tick_index(start: i32, idx_in_array: usize, tick_spacing: i32) -> i32 {
    start + (idx_in_array as i32) * tick_spacing
}
