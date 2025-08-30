
        mod program_raydium_clmm { include!(concat!(env!("OUT_DIR"), "/program_raydium_clmm.rs")); }
mod program_jupiter { include!(concat!(env!("OUT_DIR"), "/program_jupiter.rs")); }
mod program_phoenix { include!(concat!(env!("OUT_DIR"), "/program_phoenix.rs")); }
mod program_raydium_amm { include!(concat!(env!("OUT_DIR"), "/program_raydium_amm.rs")); }
mod program_pump_amm { include!(concat!(env!("OUT_DIR"), "/program_pump_amm.rs")); }
mod program_meteora_damm { include!(concat!(env!("OUT_DIR"), "/program_meteora_damm.rs")); }
mod program_pump_fun { include!(concat!(env!("OUT_DIR"), "/program_pump_fun.rs")); }
mod program_whirlpool { include!(concat!(env!("OUT_DIR"), "/program_whirlpool.rs")); }
mod program_gfx_ssl_v2 { include!(concat!(env!("OUT_DIR"), "/program_gfx_ssl_v2.rs")); }
mod program_openbook_v2 { include!(concat!(env!("OUT_DIR"), "/program_openbook_v2.rs")); }


        static PROGRAM_IDS: once_cell::sync::Lazy<std::collections::HashMap<&'static str, &'static str>> =
            once_cell::sync::Lazy::new(|| {
                let mut map: std::collections::HashMap<&'static str, &'static str> = std::collections::HashMap::new();
                map.insert("program_raydium_clmm", "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK");
map.insert("program_jupiter", "JUP4Fb2cqiRUcaTHdrPC8h2gNsA2ETXiPDD33WcGuJB");
map.insert("program_phoenix", "PhoeNiXZ8ByJGLkxNfZRnkUfjvmuYqLR89jjFHGqdXY");
map.insert("program_raydium_amm", "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8");
map.insert("program_pump_amm", "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA");
map.insert("program_meteora_damm", "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhT68b6Y1LfEiNX");
map.insert("program_pump_fun", "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P");
map.insert("program_whirlpool", "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc");
map.insert("program_gfx_ssl_v2", "GFXsSL5sSaDfNFQUYsHekbWBW1TsFdjDYzACh62tEHxn");
map.insert("program_openbook_v2", "opnb2LAfJYbRMAHHvqjCwQxanZn7ReEHp1k81EohpZb");

                map
            });

        pub fn registry() -> Vec<Box<dyn normalizer_runtime::SwapExtractor>> {
            let mut boxes: Vec<Box<dyn normalizer_runtime::SwapExtractor>> = Vec::new();
            boxes.push(Box::new(program_raydium_clmm::Extractor { program_id: PROGRAM_IDS.get("program_raydium_clmm").unwrap() }));
boxes.push(Box::new(program_jupiter::Extractor { program_id: PROGRAM_IDS.get("program_jupiter").unwrap() }));
boxes.push(Box::new(program_phoenix::Extractor { program_id: PROGRAM_IDS.get("program_phoenix").unwrap() }));
boxes.push(Box::new(program_raydium_amm::Extractor { program_id: PROGRAM_IDS.get("program_raydium_amm").unwrap() }));
boxes.push(Box::new(program_pump_amm::Extractor { program_id: PROGRAM_IDS.get("program_pump_amm").unwrap() }));
boxes.push(Box::new(program_meteora_damm::Extractor { program_id: PROGRAM_IDS.get("program_meteora_damm").unwrap() }));
boxes.push(Box::new(program_pump_fun::Extractor { program_id: PROGRAM_IDS.get("program_pump_fun").unwrap() }));
boxes.push(Box::new(program_whirlpool::Extractor { program_id: PROGRAM_IDS.get("program_whirlpool").unwrap() }));
boxes.push(Box::new(program_gfx_ssl_v2::Extractor { program_id: PROGRAM_IDS.get("program_gfx_ssl_v2").unwrap() }));
boxes.push(Box::new(program_openbook_v2::Extractor { program_id: PROGRAM_IDS.get("program_openbook_v2").unwrap() }));

            boxes
        }

        pub fn decoder_for(pid: &str) -> Option<
            fn(&[u8], &[u8], &[String]) -> Option<(String, std::collections::HashMap<String, String>)>
        > {
            match pid {
                "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK" => Some(program_raydium_clmm::decode_anchor),
"JUP4Fb2cqiRUcaTHdrPC8h2gNsA2ETXiPDD33WcGuJB" => Some(program_jupiter::decode_anchor),
"PhoeNiXZ8ByJGLkxNfZRnkUfjvmuYqLR89jjFHGqdXY" => Some(program_phoenix::decode_anchor),
"675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8" => Some(program_raydium_amm::decode_anchor),
"pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA" => Some(program_pump_amm::decode_anchor),
"LBUZKhRxPF3XUpBCjp4YzTKgLccjZhT68b6Y1LfEiNX" => Some(program_meteora_damm::decode_anchor),
"6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P" => Some(program_pump_fun::decode_anchor),
"whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc" => Some(program_whirlpool::decode_anchor),
"GFXsSL5sSaDfNFQUYsHekbWBW1TsFdjDYzACh62tEHxn" => Some(program_gfx_ssl_v2::decode_anchor),
"opnb2LAfJYbRMAHHvqjCwQxanZn7ReEHp1k81EohpZb" => Some(program_openbook_v2::decode_anchor),

                _ => None,
            }
        }
    