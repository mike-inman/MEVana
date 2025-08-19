//! Masterlist of Solana DEX Program IDs
//! This module contains all known program IDs for Orca/Whirlpool and Raydium
//! to enable accurate swap transaction detection.

use lazy_static::lazy_static;
use std::collections::HashSet;

lazy_static! {
    /// Orca Whirlpool Program IDs
    pub static ref ORCA_WHIRLPOOL_PROGRAMS: HashSet<&'static str> = {
        let mut set = HashSet::new();
        // Main Orca Whirlpool program
        set.insert("whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc");
        // Orca Whirlpool v2 (if different)
        set.insert("whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc");
        // Orca Whirlpool v3 (if different)
        set.insert("whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc");
        set
    };

    /// Raydium Program IDs
    pub static ref RAYDIUM_PROGRAMS: HashSet<&'static str> = {
        let mut set = HashSet::new();
        // Raydium AMM v4
        set.insert("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8");
        // Raydium AMM v3
        set.insert("9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM");
        // Raydium AMM v2
        set.insert("5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1");
        // Raydium CLMM (Concentrated Liquidity)
        set.insert("CAMMCzo5YL8w4VFF8KVHrK22GGUQpJ2VxqKd5qjqjqjq");
        // Raydium CPMM (Constant Product)
        set.insert("CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C");
        // Raydium Launchpad
        set.insert("9rpQHSyFVM1dkkHFQ2TtTzPEW7DVmEyPmN8wVniqJtuC");
        set
    };

    /// All DEX Programs Combined
    pub static ref ALL_DEX_PROGRAMS: HashSet<&'static str> = {
        let mut set = HashSet::new();
        set.extend(ORCA_WHIRLPOOL_PROGRAMS.iter());
        set.extend(RAYDIUM_PROGRAMS.iter());
        set
    };
}

/// Check if a program ID is an Orca/Whirlpool program
pub fn is_orca_whirlpool(program_id: &str) -> bool {
    ORCA_WHIRLPOOL_PROGRAMS.contains(program_id)
}

/// Check if a program ID is a Raydium program
pub fn is_raydium(program_id: &str) -> bool {
    RAYDIUM_PROGRAMS.contains(program_id)
}

/// Check if a program ID is any known DEX program
pub fn is_dex_program(program_id: &str) -> bool {
    ALL_DEX_PROGRAMS.contains(program_id)
}

/// Get the DEX type for a program ID
pub fn get_dex_type(program_id: &str) -> Option<&'static str> {
    if is_orca_whirlpool(program_id) {
        Some("Orca Whirlpool")
    } else if is_raydium(program_id) {
        Some("Raydium")
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_orca_whirlpool_detection() {
        assert!(is_orca_whirlpool(
            "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc"
        ));
        assert!(!is_orca_whirlpool("11111111111111111111111111111111"));
    }

    #[test]
    fn test_raydium_detection() {
        assert!(is_raydium("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8"));
        assert!(!is_raydium("11111111111111111111111111111111"));
    }

    #[test]
    fn test_dex_type_detection() {
        assert_eq!(
            get_dex_type("whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc"),
            Some("Orca Whirlpool")
        );
        assert_eq!(
            get_dex_type("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8"),
            Some("Raydium")
        );
        assert_eq!(get_dex_type("11111111111111111111111111111111"), None);
    }
}
