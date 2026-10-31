//! Offsets derive from the pinned official IDLs in README.md and are checked by real fixtures.
use anyhow::{bail, ensure, Result};
use num_bigint::BigUint;
use num_traits::{ToPrimitive, Zero};
use whirlpool_core::decode::decode_whirlpool;

pub const ORCA: &str = "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc";
pub const RAYDIUM: &str = "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK";
pub const METEORA: &str = "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo";
const PRICE_SCALE: u32 = 24;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PoolPrice {
    pub mint0: String,
    pub mint1: String,
    pub price_scaled: BigUint,
    pub fee_bps_micro: Option<u64>,
    pub base_fee_bps_micro: Option<u64>,
    pub variable_fee_modeled: bool,
    pub config: Option<String>,
}

fn key(bytes: &[u8]) -> String {
    bs58::encode(bytes).into_string()
}
fn u16le(data: &[u8], at: usize) -> Result<u16> {
    Ok(u16::from_le_bytes(
        data.get(at..at + 2)
            .ok_or_else(|| anyhow::anyhow!("short account"))?
            .try_into()?,
    ))
}
fn u32le(data: &[u8], at: usize) -> Result<u32> {
    Ok(u32::from_le_bytes(
        data.get(at..at + 4)
            .ok_or_else(|| anyhow::anyhow!("short account"))?
            .try_into()?,
    ))
}
fn pubkey(data: &[u8], at: usize) -> Result<String> {
    Ok(key(data
        .get(at..at + 32)
        .ok_or_else(|| anyhow::anyhow!("short account"))?))
}
fn decimal_adjust(mut price: BigUint, decimals0: u8, decimals1: u8) -> BigUint {
    if decimals0 >= decimals1 {
        price *= BigUint::from(10u8).pow(u32::from(decimals0 - decimals1));
    } else {
        price /= BigUint::from(10u8).pow(u32::from(decimals1 - decimals0));
    }
    price
}
fn sqrt_price(sqrt: u128, d0: u8, d1: u8) -> Result<BigUint> {
    ensure!(sqrt > 0, "zero sqrt price");
    let mut p = BigUint::from(sqrt);
    p = p.clone() * p * BigUint::from(10u8).pow(PRICE_SCALE);
    p /= BigUint::from(2u8).pow(128);
    Ok(decimal_adjust(p, d0, d1))
}
fn bin_price(active: i32, step: u16, d0: u8, d1: u8) -> Result<BigUint> {
    ensure!(
        step > 0 && active.unsigned_abs() <= 1_000_000,
        "invalid bin price parameters"
    );
    let scale = BigUint::from(10u8).pow(30);
    let unit = BigUint::from(10_000u32);
    let numerator = &unit + BigUint::from(step);
    let mut factor = if active < 0 {
        &unit * &scale / &numerator
    } else {
        &numerator * &scale / &unit
    };
    let mut result = scale.clone();
    let mut exponent = active.unsigned_abs();
    while exponent > 0 {
        if exponent & 1 != 0 {
            result = result * &factor / &scale;
        }
        exponent >>= 1;
        if exponent > 0 {
            factor = &factor * &factor / &scale;
        }
    }
    let p = decimal_adjust(result / BigUint::from(10u8).pow(30 - PRICE_SCALE), d0, d1);
    ensure!(!p.is_zero(), "price underflow");
    Ok(p)
}

pub fn decode_pool(
    venue: &str,
    owner: &str,
    data: &[u8],
    d0: u8,
    d1: u8,
    config_data: Option<&[u8]>,
) -> Result<PoolPrice> {
    ensure!(d0 <= 18 && d1 <= 18, "invalid token decimals");
    match venue {
        "Orca" => {
            ensure!(
                owner == ORCA && data.len() == 653,
                "invalid Whirlpool owner or length"
            );
            let pool = decode_whirlpool(data)?;
            let fee = u64::from(pool.fee_rate) * 10_000;
            Ok(PoolPrice {
                mint0: pool.token_mint_a.to_string(),
                mint1: pool.token_mint_b.to_string(),
                price_scaled: sqrt_price(pool.sqrt_price, d0, d1)?,
                fee_bps_micro: Some(fee),
                base_fee_bps_micro: Some(fee),
                variable_fee_modeled: true,
                config: None,
            })
        }
        "Raydium" => {
            ensure!(
                owner == RAYDIUM && data.len() == 1544,
                "invalid CLMM owner or length"
            );
            ensure!(
                data[..8] == [247, 237, 227, 245, 215, 195, 222, 70],
                "wrong PoolState discriminator"
            );
            ensure!(data[233] == d0 && data[234] == d1, "CLMM decimal mismatch");
            let config = pubkey(data, 9)?;
            let raw_fee = match config_data {
                Some(c) => {
                    ensure!(
                        c.len() == 117 && c[..8] == [218, 244, 33, 104, 203, 203, 43, 111],
                        "wrong AmmConfig layout"
                    );
                    Some(u64::from(u32le(c, 47)?) * 10_000)
                }
                None => None,
            };
            let dynamic = data[1096..1176].iter().any(|b| *b != 0);
            let sqrt = u128::from_le_bytes(data[253..269].try_into()?);
            Ok(PoolPrice {
                mint0: pubkey(data, 73)?,
                mint1: pubkey(data, 105)?,
                price_scaled: sqrt_price(sqrt, d0, d1)?,
                fee_bps_micro: if dynamic { None } else { raw_fee },
                base_fee_bps_micro: raw_fee,
                variable_fee_modeled: !dynamic,
                config: Some(config),
            })
        }
        "Meteora" => {
            ensure!(
                owner == METEORA && data.len() == 904,
                "invalid DLMM owner or length"
            );
            ensure!(
                data[..8] == [33, 11, 49, 98, 181, 101, 177, 13],
                "wrong LbPair discriminator"
            );
            let active = i32::from_le_bytes(data[76..80].try_into()?);
            let step = u16le(data, 80)?;
            let base = u64::from(u16le(data, 8)?);
            let power = u32::from(data[34]);
            ensure!(power <= 9, "unsupported base fee power");
            let fee = base
                .checked_mul(u64::from(step))
                .and_then(|v| v.checked_mul(100))
                .and_then(|v| v.checked_mul(10u64.pow(power)))
                .ok_or_else(|| anyhow::anyhow!("base fee overflow"))?;
            Ok(PoolPrice {
                mint0: pubkey(data, 88)?,
                mint1: pubkey(data, 120)?,
                price_scaled: bin_price(active, step, d0, d1)?,
                fee_bps_micro: None,
                base_fee_bps_micro: Some(fee),
                variable_fee_modeled: false,
                config: None,
            })
        }
        _ => bail!("unsupported venue"),
    }
}

pub fn relative_difference_bps(a: &BigUint, b: &BigUint) -> Result<u64> {
    ensure!(!a.is_zero() && !b.is_zero(), "zero price");
    let delta = if a >= b { a - b } else { b - a };
    (delta * BigUint::from(20_000u32) / (a + b))
        .to_u64()
        .ok_or_else(|| anyhow::anyhow!("gap overflow"))
}

#[cfg(test)]
mod tests {
    use super::*;
    const SOL: &str = "So11111111111111111111111111111111111111112";
    const USDC: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
    #[test]
    fn real_mainnet_fixtures_agree_with_whirlpool_within_one_percent() {
        let orca = decode_pool(
            "Orca",
            ORCA,
            include_bytes!("../tests/fixtures/orca_sol_usdc.bin"),
            9,
            6,
            None,
        )
        .unwrap();
        let ray = decode_pool(
            "Raydium",
            RAYDIUM,
            include_bytes!("../tests/fixtures/raydium_sol_usdc.bin"),
            9,
            6,
            Some(include_bytes!("../tests/fixtures/raydium_config.bin")),
        )
        .unwrap();
        let met = decode_pool(
            "Meteora",
            METEORA,
            include_bytes!("../tests/fixtures/meteora_sol_usdc.bin"),
            9,
            6,
            None,
        )
        .unwrap();
        for p in [&orca, &ray, &met] {
            assert_eq!(p.mint0, SOL);
            assert_eq!(p.mint1, USDC);
            assert!(relative_difference_bps(&p.price_scaled, &orca.price_scaled).unwrap() < 100);
        }
        assert_eq!(ray.fee_bps_micro, Some(4_000_000));
        assert_eq!(met.base_fee_bps_micro, Some(4_000_000));
        assert_eq!(met.fee_bps_micro, None);
    }
    #[test]
    fn corrupt_layout_is_rejected() {
        let mut raw = include_bytes!("../tests/fixtures/raydium_sol_usdc.bin").to_vec();
        raw[0] = 0;
        assert!(decode_pool("Raydium", RAYDIUM, &raw, 9, 6, None).is_err());
    }
}
