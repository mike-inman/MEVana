use std::hash::Hash;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum VenueId {
    OrcaWhirlpool = 1,
    RaydiumCpmm = 2,
    MeteoraDlmm = 3,
    Phoenix = 4,
}

impl VenueId {
    pub fn from_u32(value: u32) -> Option<Self> {
        match value {
            1 => Some(VenueId::OrcaWhirlpool),
            2 => Some(VenueId::RaydiumCpmm),
            3 => Some(VenueId::MeteoraDlmm),
            4 => Some(VenueId::Phoenix),
            _ => None,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            VenueId::OrcaWhirlpool => "OrcaWhirlpool",
            VenueId::RaydiumCpmm => "RaydiumCpmm",
            VenueId::MeteoraDlmm => "MeteoraDlmm",
            VenueId::Phoenix => "Phoenix",
        }
    }

    pub fn program_id(&self) -> &'static str {
        match self {
            VenueId::OrcaWhirlpool => "whirLbMi4jYofpSBpZUVG3WbNDLj1L9Vt1k98ajJRJm",
            VenueId::RaydiumCpmm => "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8",
            VenueId::MeteoraDlmm => "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo",
            VenueId::Phoenix => "PhoeNiXZ8ByJGLkxNfZRnkUfjvmuYqLR89jjFHGqdXY",
        }
    }
}

// Venue-specific update handlers
pub trait VenueHandler {
    fn handle_account_update(&self, account_data: &[u8], slot: u64, write_version: u64) -> anyhow::Result<Vec<crate::edge::EdgeDelta>>;
    fn venue_id(&self) -> VenueId;
}

pub struct OrcaWhirlpoolHandler;

impl VenueHandler for OrcaWhirlpoolHandler {
    fn handle_account_update(&self, _account_data: &[u8], _slot: u64, _write_version: u64) -> anyhow::Result<Vec<crate::edge::EdgeDelta>> {
        // TODO: Implement Orca-specific account parsing and EdgeDelta generation
        Ok(vec![])
    }

    fn venue_id(&self) -> VenueId {
        VenueId::OrcaWhirlpool
    }
}

pub struct RaydiumCpmmHandler;

impl VenueHandler for RaydiumCpmmHandler {
    fn handle_account_update(&self, _account_data: &[u8], _slot: u64, _write_version: u64) -> anyhow::Result<Vec<crate::edge::EdgeDelta>> {
        // TODO: Implement Raydium CPMM parsing
        Ok(vec![])
    }

    fn venue_id(&self) -> VenueId {
        VenueId::RaydiumCpmm
    }
}
