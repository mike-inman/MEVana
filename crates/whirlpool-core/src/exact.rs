//! Integer-only standard-fee Whirlpool quotes. Legacy `quoter` is not used here.
use crate::{decode, pda, types::WhirlpoolMinimal};
use orca_whirlpools_core::{
    compute_swap, tick_index_to_sqrt_price, TickArrayFacade, TickArraySequence, TickFacade,
    WhirlpoolFacade, WhirlpoolRewardInfoFacade,
};
use solana_sdk::pubkey::Pubkey;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExactError {
    AdaptiveFeeUnsupported,
    MissingTickArray { start_tick_index: i32 },
    InvalidTickArray(String),
    InvalidPool,
    Quote(String),
}

impl std::fmt::Display for ExactError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for ExactError {}

#[derive(Debug, Clone)]
pub struct DecodedTickArray {
    pub whirlpool: Pubkey,
    pub facade: TickArrayFacade,
}

/// The Whirlpool tick-array PDA uses a decimal start-index seed. The legacy
/// `pda::derive_tick_array_address` helper uses LE bytes and is retained for
/// existing callers; captured array keys verify this derivation.
pub fn derive_tick_array_address(
    program_id: &Pubkey,
    pool_id: &Pubkey,
    start_tick_index: i32,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            b"tick_array",
            pool_id.as_ref(),
            start_tick_index.to_string().as_bytes(),
        ],
        program_id,
    )
}

fn tick_facade(tick: &crate::types::Tick) -> TickFacade {
    TickFacade {
        initialized: tick.initialized,
        liquidity_net: tick.liquidity_net,
        liquidity_gross: tick.liquidity_gross,
        fee_growth_outside_a: tick.fee_growth_outside_a,
        fee_growth_outside_b: tick.fee_growth_outside_b,
        reward_growths_outside: tick.reward_growths_outside,
    }
}

pub fn decode_array(data: &[u8]) -> Result<DecodedTickArray, ExactError> {
    if data.get(..8) == Some(decode::disc_tick_array().as_slice()) {
        let array = decode::decode_tick_array(data)
            .map_err(|e| ExactError::InvalidTickArray(e.to_string()))?;
        return Ok(DecodedTickArray {
            whirlpool: array.whirlpool,
            facade: TickArrayFacade {
                start_tick_index: array.start_tick_index,
                ticks: std::array::from_fn(|i| tick_facade(&array.ticks[i])),
            },
        });
    }
    if data.get(..8) != Some(decode::anchor_discriminator("account:DynamicTickArray").as_slice()) {
        return Err(ExactError::InvalidTickArray("wrong discriminator".into()));
    }
    if data.len() < 148 || data.len() > 148 + 88 * 112 || !(data.len() - 148).is_multiple_of(112) {
        return Err(ExactError::InvalidTickArray(
            "invalid dynamic length".into(),
        ));
    }
    let start_tick_index = i32::from_le_bytes(data[8..12].try_into().unwrap());
    let whirlpool = Pubkey::new_from_array(data[12..44].try_into().unwrap());
    let bitmap = u128::from_le_bytes(data[44..60].try_into().unwrap());
    let mut offset = 60;
    let mut ticks = [TickFacade::default(); 88];
    for (i, tick) in ticks.iter_mut().enumerate() {
        let tag = data[offset];
        offset += 1;
        if tag > 1 || (bitmap >> i) & 1 != u128::from(tag) {
            return Err(ExactError::InvalidTickArray(
                "invalid dynamic tick tag/bitmap".into(),
            ));
        }
        if tag == 1 {
            if data.len() - offset < 112 {
                return Err(ExactError::InvalidTickArray(
                    "truncated dynamic tick".into(),
                ));
            }
            let mut next_i128 = || {
                let value = i128::from_le_bytes(data[offset..offset + 16].try_into().unwrap());
                offset += 16;
                value
            };
            let liquidity_net = next_i128();
            let mut next_u128 = || {
                let value = u128::from_le_bytes(data[offset..offset + 16].try_into().unwrap());
                offset += 16;
                value
            };
            *tick = TickFacade {
                initialized: true,
                liquidity_net,
                liquidity_gross: next_u128(),
                fee_growth_outside_a: next_u128(),
                fee_growth_outside_b: next_u128(),
                reward_growths_outside: [next_u128(), next_u128(), next_u128()],
            };
        }
    }
    if offset != data.len() {
        return Err(ExactError::InvalidTickArray(
            "trailing dynamic bytes".into(),
        ));
    }
    Ok(DecodedTickArray {
        whirlpool,
        facade: TickArrayFacade {
            start_tick_index,
            ticks,
        },
    })
}

pub fn pool_facade(pool: &WhirlpoolMinimal) -> Result<WhirlpoolFacade, ExactError> {
    if pool.tick_spacing == 0 {
        return Err(ExactError::InvalidPool);
    }
    if u16::from_le_bytes(pool.tick_spacing_seed) != pool.tick_spacing {
        return Err(ExactError::AdaptiveFeeUnsupported);
    }
    Ok(WhirlpoolFacade {
        tick_spacing: pool.tick_spacing,
        fee_rate: pool.fee_rate,
        protocol_fee_rate: pool.protocol_fee_rate,
        liquidity: pool.liquidity,
        sqrt_price: pool.sqrt_price,
        tick_current_index: pool.tick_current_index,
        fee_growth_global_a: pool.fee_growth_global_a,
        fee_growth_global_b: pool.fee_growth_global_b,
        reward_last_updated_timestamp: pool.reward_last_updated_timestamp,
        reward_infos: std::array::from_fn(|i| WhirlpoolRewardInfoFacade {
            emissions_per_second_x64: pool.reward_infos[i].emissions_per_second_x64,
            growth_global_x64: pool.reward_infos[i].growth_global_x64,
        }),
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExactQuote {
    pub input_amount: u64,
    pub output_amount: u64,
    pub fee_amount: u64,
}

fn quote(
    program_id: &Pubkey,
    pool_id: &Pubkey,
    pool: &WhirlpoolMinimal,
    arrays: &BTreeMap<Pubkey, DecodedTickArray>,
    amount: u64,
    a_to_b: bool,
    exact_in: bool,
) -> Result<ExactQuote, ExactError> {
    let facade = pool_facade(pool)?;
    let (_, current_start, _) =
        pda::three_tick_arrays_around(pool.tick_current_index, i32::from(pool.tick_spacing));
    let span = 88 * i32::from(pool.tick_spacing);
    let mut selected = Vec::new();
    let mut first_missing = None;
    for step in 0..6 {
        let start = if a_to_b {
            current_start - step * span
        } else {
            current_start + step * span
        };
        let (key, _) = derive_tick_array_address(program_id, pool_id, start);
        match arrays.get(&key) {
            Some(array)
                if array.whirlpool == *pool_id && array.facade.start_tick_index == start =>
            {
                selected.push(array.facade);
            }
            Some(_) => {
                return Err(ExactError::InvalidTickArray(
                    "array identity mismatch".into(),
                ))
            }
            None if step == 0 => {
                return Err(ExactError::MissingTickArray {
                    start_tick_index: start,
                })
            }
            None => {
                first_missing = Some(start);
                break;
            }
        }
    }
    selected.sort_by_key(|array| array.start_tick_index);
    let mut items = [None; 6];
    for (slot, array) in items.iter_mut().zip(selected) {
        *slot = Some(array);
    }
    let sequence = TickArraySequence::new(items, pool.tick_spacing)
        .map_err(|e| ExactError::Quote(format!("{e:?}")))?;
    // Bound the swap to the supplied arrays. With a zero limit the pinned core
    // re-targets the sequence edge forever once the arrays are exhausted; a
    // limit at that edge stops there, and the partial-fill check below reports
    // the missing array. Quotes that finish inside the arrays are unaffected.
    let edge = if a_to_b {
        sequence.start_index()
    } else {
        sequence.end_index()
    };
    let limit = tick_index_to_sqrt_price(edge);
    if (a_to_b && limit >= pool.sqrt_price) || (!a_to_b && limit <= pool.sqrt_price) {
        return Err(ExactError::MissingTickArray {
            start_tick_index: first_missing.unwrap_or(current_start),
        });
    }
    let result = compute_swap(amount, limit, facade, sequence, a_to_b, exact_in, 0)
        .map_err(|e| ExactError::Quote(format!("{e:?}")))?;
    let (input, output) = if a_to_b {
        (result.token_a, result.token_b)
    } else {
        (result.token_b, result.token_a)
    };
    if (exact_in && input != amount) || (!exact_in && output != amount) {
        let start = first_missing.unwrap_or_else(|| {
            if a_to_b {
                current_start - 6 * span
            } else {
                current_start + 6 * span
            }
        });
        return Err(ExactError::MissingTickArray {
            start_tick_index: start,
        });
    }
    Ok(ExactQuote {
        input_amount: input,
        output_amount: output,
        fee_amount: result.trade_fee,
    })
}

pub fn quote_exact_in(
    program_id: &Pubkey,
    pool_id: &Pubkey,
    pool: &WhirlpoolMinimal,
    arrays: &BTreeMap<Pubkey, DecodedTickArray>,
    input_amount: u64,
    a_to_b: bool,
) -> Result<ExactQuote, ExactError> {
    quote(
        program_id,
        pool_id,
        pool,
        arrays,
        input_amount,
        a_to_b,
        true,
    )
}

pub fn quote_exact_out(
    program_id: &Pubkey,
    pool_id: &Pubkey,
    pool: &WhirlpoolMinimal,
    arrays: &BTreeMap<Pubkey, DecodedTickArray>,
    output_amount: u64,
    a_to_b: bool,
) -> Result<ExactQuote, ExactError> {
    quote(
        program_id,
        pool_id,
        pool,
        arrays,
        output_amount,
        a_to_b,
        false,
    )
}
