use anyhow::{Result, Context};
use solana_sdk::pubkey::Pubkey;
use std::convert::TryInto;
use whirlpool_core::types::{TickArray, Tick};

/// Parser for Orca Whirlpool TickArray account data
pub struct TickArrayParser;

impl TickArrayParser {
    /// Parse raw account data into a TickArray struct
    pub fn parse(account_data: &[u8]) -> Result<TickArray> {
        if account_data.len() < 8 {
            return Err(anyhow::anyhow!("Account data too short for discriminator"));
        }

        // Check discriminator (first 8 bytes should match TickArray discriminator)
        let expected_discriminator = [0x5f, 0x96, 0x8a, 0x96, 0x8e, 0x1a, 0x3f, 0x8b]; // tick_array discriminator
        let discriminator = &account_data[0..8];
        
        if discriminator != expected_discriminator {
            return Err(anyhow::anyhow!(
                "Invalid discriminator. Expected: {:?}, got: {:?}",
                expected_discriminator,
                discriminator
            ));
        }

        let mut offset = 8;

        // Parse start_tick_index (i32)
        let start_tick_index = i32::from_le_bytes(
            account_data[offset..offset + 4]
                .try_into()
                .context("Failed to read start_tick_index")?
        );
        offset += 4;

        // Parse whirlpool (Pubkey - 32 bytes)
        let whirlpool_bytes: [u8; 32] = account_data[offset..offset + 32]
            .try_into()
            .context("Failed to read whirlpool pubkey")?;
        let whirlpool = Pubkey::from(whirlpool_bytes);
        offset += 32;

        // Parse ticks array (88 ticks)
        let mut ticks = Vec::with_capacity(88);
        
        for i in 0..88 {
            let tick = Self::parse_tick(&account_data[offset..])?;
            ticks.push(tick);
            offset += Self::tick_size(); // Each tick is a fixed size
        }

        Ok(TickArray {
            start_tick_index,
            ticks,
            whirlpool,
        })
    }

    /// Parse a single tick from account data
    fn parse_tick(data: &[u8]) -> Result<Tick> {
        if data.len() < Self::tick_size() {
            return Err(anyhow::anyhow!("Insufficient data for tick"));
        }

        let mut offset = 0;

        // Parse initialized (bool - 1 byte)
        let initialized = data[offset] != 0;
        offset += 1;

        // Parse liquidity_net (i128 - 16 bytes)
        let liquidity_net_bytes: [u8; 16] = data[offset..offset + 16]
            .try_into()
            .context("Failed to read liquidity_net")?;
        let liquidity_net = i128::from_le_bytes(liquidity_net_bytes);
        offset += 16;

        // Parse liquidity_gross (u128 - 16 bytes)
        let liquidity_gross_bytes: [u8; 16] = data[offset..offset + 16]
            .try_into()
            .context("Failed to read liquidity_gross")?;
        let liquidity_gross = u128::from_le_bytes(liquidity_gross_bytes);
        offset += 16;

        // Parse fee_growth_outside_a (u128 - 16 bytes)
        let fee_growth_outside_a_bytes: [u8; 16] = data[offset..offset + 16]
            .try_into()
            .context("Failed to read fee_growth_outside_a")?;
        let fee_growth_outside_a = u128::from_le_bytes(fee_growth_outside_a_bytes);
        offset += 16;

        // Parse fee_growth_outside_b (u128 - 16 bytes)
        let fee_growth_outside_b_bytes: [u8; 16] = data[offset..offset + 16]
            .try_into()
            .context("Failed to read fee_growth_outside_b")?;
        let fee_growth_outside_b = u128::from_le_bytes(fee_growth_outside_b_bytes);

        Ok(Tick {
            initialized,
            liquidity_net,
            liquidity_gross,
            fee_growth_outside_a,
            fee_growth_outside_b,
        })
    }

    /// Size of a single tick in bytes
    fn tick_size() -> usize {
        1 + 16 + 16 + 16 + 16 // bool + i128 + u128 + u128 + u128
    }

    /// Validate that the tick array is properly structured
    pub fn validate_tick_array(tick_array: &TickArray) -> Result<()> {
        // Check that start_tick_index is properly aligned
        let tick_spacing = Self::infer_tick_spacing(tick_array)?;
        let expected_array_size = 88 * tick_spacing;
        
        if tick_array.start_tick_index % expected_array_size != 0 {
            return Err(anyhow::anyhow!(
                "start_tick_index {} not properly aligned for tick_spacing {}",
                tick_array.start_tick_index,
                tick_spacing
            ));
        }

        // Check that we have exactly 88 ticks
        if tick_array.ticks.len() != 88 {
            return Err(anyhow::anyhow!(
                "Expected 88 ticks, got {}",
                tick_array.ticks.len()
            ));
        }

        // Validate liquidity constraints
        for (i, tick) in tick_array.ticks.iter().enumerate() {
            if tick.initialized {
                // Initialized ticks should have some liquidity gross
                if tick.liquidity_gross == 0 {
                    return Err(anyhow::anyhow!(
                        "Initialized tick at index {} has zero liquidity_gross",
                        i
                    ));
                }
            } else {
                // Uninitialized ticks should have zero liquidity
                if tick.liquidity_gross != 0 || tick.liquidity_net != 0 {
                    return Err(anyhow::anyhow!(
                        "Uninitialized tick at index {} has non-zero liquidity",
                        i
                    ));
                }
            }
        }

        Ok(())
    }

    /// Infer tick spacing from the tick array structure
    /// This is a heuristic based on common Orca pool configurations
    fn infer_tick_spacing(tick_array: &TickArray) -> Result<i32> {
        // Common tick spacings: 1, 8, 64, 128
        let common_spacings = [1, 8, 64, 128];
        
        for &spacing in &common_spacings {
            let array_size = 88 * spacing;
            if tick_array.start_tick_index % array_size == 0 {
                return Ok(spacing);
            }
        }
        
        // Fallback: try to infer from initialized ticks
        let mut initialized_indices = Vec::new();
        for (i, tick) in tick_array.ticks.iter().enumerate() {
            if tick.initialized {
                let tick_index = tick_array.start_tick_index + (i as i32);
                initialized_indices.push(tick_index);
            }
        }
        
        if initialized_indices.len() >= 2 {
            // Find GCD of differences between initialized tick indices
            let mut differences = Vec::new();
            for i in 1..initialized_indices.len() {
                let diff = (initialized_indices[i] - initialized_indices[i-1]).abs();
                if diff > 0 {
                    differences.push(diff);
                }
            }
            
            if !differences.is_empty() {
                let gcd = differences.into_iter().reduce(|acc, x| gcd(acc, x)).unwrap();
                if common_spacings.contains(&gcd) {
                    return Ok(gcd);
                }
            }
        }
        
        Err(anyhow::anyhow!("Could not infer tick spacing"))
    }
}

/// Utility function to compute GCD
fn gcd(a: i32, b: i32) -> i32 {
    if b == 0 { a } else { gcd(b, a % b) }
}

/// Integration helper for the main EdgeAssembler
impl super::EdgeAssembler {
    /// Updated handler for tick array messages
    pub async fn handle_tick_array_update(&self, data: &serde_json::Value) -> Result<()> {
        if let Some(account_data_b64) = data.get("account").and_then(|v| v.as_str()) {
            // Decode base64 account data
            let account_data = base64::decode(account_data_b64)
                .context("Failed to decode base64 account data")?;
            
            // Parse tick array
            let tick_array = TickArrayParser::parse(&account_data)
                .context("Failed to parse tick array")?;
            
            // Validate the parsed data
            TickArrayParser::validate_tick_array(&tick_array)
                .context("Tick array validation failed")?;
            
            let ta_pubkey = data
                .get("pubkey")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Missing pubkey in tick array update"))?;
            
            let slot = data.get("slot").and_then(|v| v.as_u64()).unwrap_or(0);
            let write_version = data.get("write_version").and_then(|v| v.as_u64()).unwrap_or(0);
            
            // Process the update
            self.update_tick_array_and_emit_edges(ta_pubkey, &tick_array, slot, write_version).await?;
            
            log::info!(
                "Processed tick array update: {} (pool: {}, start_tick: {}, initialized_ticks: {})",
                ta_pubkey,
                tick_array.whirlpool,
                tick_array.start_tick_index,
                tick_array.ticks.iter().filter(|t| t.initialized).count()
            );
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tick_size_calculation() {
        assert_eq!(TickArrayParser::tick_size(), 65); // 1 + 16 + 16 + 16 + 16
    }

    #[test]
    fn test_gcd() {
        assert_eq!(gcd(48, 18), 6);
        assert_eq!(gcd(64, 128), 64);
        assert_eq!(gcd(1, 8), 1);
    }

    #[test]
    fn test_tick_spacing_inference() {
        // Test with a properly aligned tick array
        let mut tick_array = TickArray {
            start_tick_index: -11264, // Aligned for tick_spacing = 64 (88 * 64 = 5632)
            whirlpool: Pubkey::default(),
            ticks: vec![Tick::default(); 88],
        };
        
        // Add some initialized ticks at proper intervals
        tick_array.ticks[0].initialized = true;
        tick_array.ticks[0].liquidity_gross = 1000;
        tick_array.ticks[8].initialized = true;
        tick_array.ticks[8].liquidity_gross = 2000;
        
        assert_eq!(TickArrayParser::infer_tick_spacing(&tick_array).unwrap(), 64);
    }
}
