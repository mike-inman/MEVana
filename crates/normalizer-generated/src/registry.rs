
        mod program_jupiter { include!(concat!(env!("OUT_DIR"), "/program_jupiter.rs")); }
mod program_phoenix { include!(concat!(env!("OUT_DIR"), "/program_phoenix.rs")); }
mod program_whirlpool { include!(concat!(env!("OUT_DIR"), "/program_whirlpool.rs")); }
mod program_meteora { include!(concat!(env!("OUT_DIR"), "/program_meteora.rs")); }
mod program_raydium_amm { include!(concat!(env!("OUT_DIR"), "/program_raydium_amm.rs")); }


        static PROGRAM_IDS: once_cell::sync::Lazy<std::collections::HashMap<&'static str, &'static str>> =
            once_cell::sync::Lazy::new(|| {
                let mut map: std::collections::HashMap<&'static str, &'static str> = std::collections::HashMap::new();
                map.insert("program_jupiter", "JUP4Fb2cqiRUcaTHdrPC8h2gNsA2ETXiPDD33WcGuJB");
map.insert("program_phoenix", "PhoeNiXZ8ByJGLkxNfZRnkUfjvmuYqLR89jjFHGqdXY");
map.insert("program_whirlpool", "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc");
map.insert("program_meteora", "cpamdpZCGKUy5JxQXB4dcpGPiikHawvSWAd6mEn1sGG");
map.insert("program_raydium_amm", "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8");

                map
            });

        pub fn registry() -> Vec<Box<dyn normalizer_runtime::SwapExtractor>> {
            let mut boxes: Vec<Box<dyn normalizer_runtime::SwapExtractor>> = Vec::new();
            boxes.push(Box::new(program_jupiter::Extractor { program_id: PROGRAM_IDS.get("program_jupiter").unwrap() }));
boxes.push(Box::new(program_phoenix::Extractor { program_id: PROGRAM_IDS.get("program_phoenix").unwrap() }));
boxes.push(Box::new(program_whirlpool::Extractor { program_id: PROGRAM_IDS.get("program_whirlpool").unwrap() }));
boxes.push(Box::new(program_meteora::Extractor { program_id: PROGRAM_IDS.get("program_meteora").unwrap() }));
boxes.push(Box::new(program_raydium_amm::Extractor { program_id: PROGRAM_IDS.get("program_raydium_amm").unwrap() }));

            boxes
        }

        /// Get a decoder function for this program id, if available.
        pub fn decoder_for(pid: &str) -> Option<
            fn(&[u8], &[u8], &[String]) -> Option<(String, std::collections::HashMap<String, String>)>
        > {
            match pid {
                "JUP4Fb2cqiRUcaTHdrPC8h2gNsA2ETXiPDD33WcGuJB" => Some(program_jupiter::decode_anchor),
"PhoeNiXZ8ByJGLkxNfZRnkUfjvmuYqLR89jjFHGqdXY" => Some(program_phoenix::decode_anchor),
"whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc" => Some(program_whirlpool::decode_anchor),
"cpamdpZCGKUy5JxQXB4dcpGPiikHawvSWAd6mEn1sGG" => Some(program_meteora::decode_anchor),
"675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8" => Some(program_raydium_amm::decode_anchor),

                _ => None,
            }
        }
    