use anyhow::{Result, Context};
use solana_sdk::pubkey::Pubkey;
use std::convert::TryInto;
use borsh::BorshDeserialize;
use serde::Deserialize;
use reqwest;

/// Orca Whirlpool Tick structure (matches whirlpool-core)
#[derive(Clone, Debug, Default, Copy)]
pub struct Tick {
    pub liquidity_net: i128,        // 16
    pub liquidity_gross: u128,      // 16
    pub fee_growth_outside_a: u128, // 16
    pub fee_growth_outside_b: u128, // 16
    pub reward_growths_outside: [u128; 3], // 48
}

/// Orca Whirlpool TickArray structure
#[derive(Clone, Debug)]
pub struct TickArray {
    pub start_tick_index: i32, // 4
    pub whirlpool: Pubkey,     // 32
    pub ticks: [Tick; 88],     // 88 * 112 bytes
}

/// Whirlpool Position account (what we're actually receiving)
#[derive(Clone, Debug)]
pub struct Position {
    pub position_mint: Pubkey,
    pub whirlpool: Pubkey,
    pub liquidity: u128,
    pub tick_lower_index: i32,
    pub tick_upper_index: i32,
}

/// Whirlpool account type enum
#[derive(Clone, Debug)]
pub enum WhirlpoolAccountType {
    TickArray(TickArray),
    Position(Position),
    Pool,
}

#[inline]
pub fn tick_is_initialized(t: &Tick) -> bool {
    t.liquidity_gross > 0
}

/// Shyft API response structure for account data
#[derive(Deserialize)]
struct ShyftAccountResponse {
    success: bool,
    result: Option<ShyftAccountData>,
}

#[derive(Deserialize)]
struct ShyftAccountData {
    pubkey: String,
    account: ShyftAccount,
}

#[derive(Deserialize)]
struct ShyftAccount {
    data: Vec<String>, // Base64 encoded account data
    executable: bool,
    lamports: u64,
    owner: String,
}

/// Parser for Orca Whirlpool TickArray account data with Shyft API fallback
pub struct TickArrayParser {
    shyft_client: Option<reqwest::Client>,
    shyft_api_key: Option<String>,
}

impl TickArrayParser {
    // Known Whirlpool discriminators
    const TICK_ARRAY_DISCRIMINATOR: [u8; 8] = [0x24, 0x42, 0xf5, 0xb9, 0x1c, 0x9a, 0x3f, 0x99];
    const TICK_ARRAY_DISCRIMINATOR_ALT: [u8; 8] = [69, 97, 189, 190, 110, 7, 66, 187]; // Found in live data
    const POSITION_DISCRIMINATOR: [u8; 8] = [0x11, 0xd8, 0xf6, 0x8e, 0xe1, 0xc7, 0xda, 0x38]; // [17, 216, 246, 142, 225, 199, 218, 56]
    const WHIRLPOOL_DISCRIMINATOR: [u8; 8] = [0x63, 0x14, 0x93, 0x54, 0x85, 0x35, 0x11, 0x4a];
    const WHIRLPOOL_CONFIG_DISCRIMINATOR: [u8; 8] = [63, 149, 209, 12, 225, 128, 99, 9]; // Found in live data

    /// Create new parser with optional Shyft API fallback
    pub fn new() -> Self {
        let shyft_api_key = std::env::var("SHYFT_API_KEY").ok();
        let shyft_client = if shyft_api_key.is_some() {
            Some(reqwest::Client::new())
        } else {
            None
        };

        Self {
            shyft_client,
            shyft_api_key,
        }
    }

    /// Create parser without Shyft fallback (for static methods)
    pub fn default_instance() -> Self {
        Self {
            shyft_client: None,
            shyft_api_key: None,
        }
    }

    /// Parse raw account data - detects account type by discriminator
    pub fn parse_account(account_data: &[u8]) -> Result<WhirlpoolAccountType> {
        if account_data.len() < 8 {
            return Err(anyhow::anyhow!("Account data too short for discriminator"));
        }

        let discriminator = &account_data[0..8];
        
        if discriminator == &Self::TICK_ARRAY_DISCRIMINATOR || discriminator == &Self::TICK_ARRAY_DISCRIMINATOR_ALT {
            let tick_array = Self::parse_tick_array_flexible(account_data)?;
            Ok(WhirlpoolAccountType::TickArray(tick_array))
        } else if discriminator == &Self::POSITION_DISCRIMINATOR {
            let position = Self::parse_position(account_data)?;
            Ok(WhirlpoolAccountType::Position(position))
        } else if discriminator == &Self::WHIRLPOOL_DISCRIMINATOR {
            Ok(WhirlpoolAccountType::Pool) // Pool parsing handled separately
        } else if discriminator == &Self::WHIRLPOOL_CONFIG_DISCRIMINATOR {
            Ok(WhirlpoolAccountType::Pool) // Config accounts - treat as pool for now
        } else {
            Err(anyhow::anyhow!(
                "Unknown Whirlpool account type. Discriminator: {:?}",
                discriminator
            ))
        }
    }

    /// Fetch account data from Shyft API as fallback
    pub async fn fetch_account_data_from_shyft(&self, pubkey: &str) -> Result<Vec<u8>> {
        let client = self.shyft_client.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Shyft client not available"))?;
        let api_key = self.shyft_api_key.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Shyft API key not available"))?;

        let url = format!(
            "https://api.shyft.to/sol/v1/account/info?network=mainnet-beta&account={}",
            pubkey
        );

        let response = client
            .get(&url)
            .header("X-API-KEY", api_key)
            .send()
            .await
            .context("Failed to fetch from Shyft API")?;

        let shyft_response: ShyftAccountResponse = response
            .json()
            .await
            .context("Failed to parse Shyft response")?;

        if !shyft_response.success {
            return Err(anyhow::anyhow!("Shyft API returned unsuccessful response"));
        }

        let account_data = shyft_response.result
            .ok_or_else(|| anyhow::anyhow!("No account data in Shyft response"))?
            .account
            .data;

        if account_data.is_empty() {
            return Err(anyhow::anyhow!("Empty account data from Shyft"));
        }

        // Decode base64 account data
        use base64::{Engine as _, engine::general_purpose};
        let decoded_data = general_purpose::STANDARD
            .decode(&account_data[0])
            .context("Failed to decode base64 account data")?;

        Ok(decoded_data)
    }

    /// Parse account with Shyft fallback if needed
    pub async fn parse_account_with_fallback(&self, pubkey: &str, account_data: Option<&[u8]>) -> Result<WhirlpoolAccountType> {
        // Try with provided data first
        if let Some(data) = account_data {
            match Self::parse_account(data) {
                Ok(result) => return Ok(result),
                Err(e) => {
                    log::warn!("Direct parsing failed for {}: {}. Trying Shyft fallback...", pubkey, e);
                }
            }
        }

        // Fallback to Shyft API
        let fallback_data = self.fetch_account_data_from_shyft(pubkey).await?;
        Self::parse_account(&fallback_data)
    }

    /// Parse raw account data into a TickArray struct (flexible discriminator)
    pub fn parse_tick_array_flexible(account_data: &[u8]) -> Result<TickArray> {
        if account_data.len() < 8 {
            return Err(anyhow::anyhow!("Account data too short for discriminator"));
        }

        let discriminator = &account_data[0..8];
        if discriminator != &Self::TICK_ARRAY_DISCRIMINATOR && discriminator != &Self::TICK_ARRAY_DISCRIMINATOR_ALT {
            return Err(anyhow::anyhow!(
                "Invalid TickArray discriminator. Expected: {:?} or {:?}, got: {:?}",
                Self::TICK_ARRAY_DISCRIMINATOR,
                Self::TICK_ARRAY_DISCRIMINATOR_ALT,
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

        // Parse ticks array (88 ticks) - need to use array instead of Vec
        let mut ticks: [Tick; 88] = [Tick::default(); 88];
        
        for i in 0..88 {
            let tick = Self::parse_tick(&account_data[offset..])?;
            ticks[i] = tick;
            offset += Self::tick_size(); // Each tick is 112 bytes
        }

        Ok(TickArray {
            start_tick_index,
            whirlpool,
            ticks,
        })
    }

    /// Parse raw account data into a TickArray struct
    pub fn parse_tick_array(account_data: &[u8]) -> Result<TickArray> {
        if account_data.len() < 8 {
            return Err(anyhow::anyhow!("Account data too short for discriminator"));
        }

        let discriminator = &account_data[0..8];
        if discriminator != &Self::TICK_ARRAY_DISCRIMINATOR {
            return Err(anyhow::anyhow!(
                "Invalid TickArray discriminator. Expected: {:?}, got: {:?}",
                Self::TICK_ARRAY_DISCRIMINATOR,
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

        // Parse ticks array (88 ticks) - need to use array instead of Vec
        let mut ticks: [Tick; 88] = [Tick::default(); 88];
        
        for i in 0..88 {
            let tick = Self::parse_tick(&account_data[offset..])?;
            ticks[i] = tick;
            offset += Self::tick_size(); // Each tick is 112 bytes
        }

        Ok(TickArray {
            start_tick_index,
            whirlpool,
            ticks,
        })
    }

    /// Parse a position account (what we're actually receiving)
    pub fn parse_position(account_data: &[u8]) -> Result<Position> {
        if account_data.len() < 148 {
            return Err(anyhow::anyhow!("Position account data too short: {} bytes", account_data.len()));
        }

        let discriminator = &account_data[0..8];
        if discriminator != &Self::POSITION_DISCRIMINATOR {
            return Err(anyhow::anyhow!(
                "Invalid Position discriminator. Expected: {:?}, got: {:?}",
                Self::POSITION_DISCRIMINATOR,
                discriminator
            ));
        }

        let mut offset = 8;

        // Position mint (32 bytes)
        let position_mint = Pubkey::from(<[u8; 32]>::try_from(&account_data[offset..offset+32])?);
        offset += 32;

        // Whirlpool (32 bytes)
        let whirlpool = Pubkey::from(<[u8; 32]>::try_from(&account_data[offset..offset+32])?);
        offset += 32;

        // Liquidity (16 bytes)
        let liquidity = u128::from_le_bytes(<[u8; 16]>::try_from(&account_data[offset..offset+16])?);
        offset += 16;

        // Tick lower index (4 bytes)
        let tick_lower_index = i32::from_le_bytes(<[u8; 4]>::try_from(&account_data[offset..offset+4])?);
        offset += 4;

        // Tick upper index (4 bytes)
        let tick_upper_index = i32::from_le_bytes(<[u8; 4]>::try_from(&account_data[offset..offset+4])?);

        Ok(Position {
            position_mint,
            whirlpool,
            liquidity,
            tick_lower_index,
            tick_upper_index,
        })
    }

    /// Parse a single tick from account data (matches whirlpool-core structure)
    fn parse_tick(data: &[u8]) -> Result<Tick> {
        if data.len() < Self::tick_size() {
            return Err(anyhow::anyhow!("Insufficient data for tick"));
        }

        let mut offset = 0;

        // Parse liquidity_net (i128 - 16 bytes)
        let liquidity_net = i128::from_le_bytes(<[u8; 16]>::try_from(&data[offset..offset + 16])?);
        offset += 16;

        // Parse liquidity_gross (u128 - 16 bytes) 
        let liquidity_gross = u128::from_le_bytes(<[u8; 16]>::try_from(&data[offset..offset + 16])?);
        offset += 16;

        // Parse fee_growth_outside_a (u128 - 16 bytes)
        let fee_growth_outside_a = u128::from_le_bytes(<[u8; 16]>::try_from(&data[offset..offset + 16])?);
        offset += 16;

        // Parse fee_growth_outside_b (u128 - 16 bytes)
        let fee_growth_outside_b = u128::from_le_bytes(<[u8; 16]>::try_from(&data[offset..offset + 16])?);
        offset += 16;

        // Parse reward_growths_outside (3 * u128 - 48 bytes)
        let mut reward_growths_outside = [0u128; 3];
        for i in 0..3 {
            reward_growths_outside[i] = u128::from_le_bytes(<[u8; 16]>::try_from(&data[offset..offset + 16])?);
            offset += 16;
        }

        Ok(Tick {
            liquidity_net,
            liquidity_gross,
            fee_growth_outside_a,
            fee_growth_outside_b,
            reward_growths_outside,
        })
    }

    /// Size of a single tick in bytes (matches whirlpool-core)
    fn tick_size() -> usize {
        16 + 16 + 16 + 16 + 48 // i128 + u128 + u128 + u128 + 3*u128 = 112 bytes
    }

    /// Validate that the tick array is properly structured
    pub fn validate_tick_array(tick_array: &TickArray) -> Result<()> {
        // Check that we have exactly 88 ticks
        if tick_array.ticks.len() != 88 {
            return Err(anyhow::anyhow!(
                "Expected 88 ticks, got {}",
                tick_array.ticks.len()
            ));
        }

        // Validate liquidity constraints  
        for (i, tick) in tick_array.ticks.iter().enumerate() {
            if tick_is_initialized(tick) {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tick_size_calculation() {
        assert_eq!(TickArrayParser::tick_size(), 65); // 1 + 16 + 16 + 16 + 16
    }
}