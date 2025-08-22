use crate::types::{TickArray, WhirlpoolMinimal};
use anyhow::{anyhow, Result};
use borsh::BorshDeserialize;

pub fn anchor_discriminator(name: &str) -> [u8; 8] {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(name.as_bytes());
    let hash = hasher.finalize();
    let mut disc = [0u8; 8];
    disc.copy_from_slice(&hash[..8]);
    disc
}

#[inline]
pub fn disc_whirlpool() -> [u8; 8] {
    anchor_discriminator("account:Whirlpool")
}

#[inline]
pub fn disc_tick_array() -> [u8; 8] {
    anchor_discriminator("account:TickArray")
}

pub fn decode_whirlpool(data: &[u8]) -> Result<WhirlpoolMinimal> {
    if data.len() < 8 || data[..8] != disc_whirlpool() {
        return Err(anyhow!("wrong discriminator for Whirlpool"));
    }
    let mut cursor = &data[8..];
    Ok(WhirlpoolMinimal::deserialize(&mut cursor)?)
}

pub fn decode_tick_array(data: &[u8]) -> Result<TickArray> {
    if data.len() < 8 || data[..8] != disc_tick_array() {
        return Err(anyhow!("wrong discriminator for TickArray"));
    }
    let mut cursor = &data[8..];
    Ok(TickArray::deserialize(&mut cursor)?)
}
