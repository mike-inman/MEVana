use serde::{Deserialize, Serialize};

// VenueId enum - separate module to avoid protobuf conflicts
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
}
