
        use normalizer_runtime as rt;

        /// Return (instruction_name, accounts_by_role) if this instruction matches a known method.
        pub fn decode_anchor(ix_data: &[u8], account_indices: &[u8], account_keys: &[String])
            -> Option<(String, std::collections::HashMap<String, String>)>
        {
            if ix_data.len() < 8 { return None; }
            
            if &ix_data[..8] == &[248u8, 198u8, 158u8, 145u8, 225u8, 117u8, 135u8, 200u8] {
                // IDL-declared role order for swap
                const ROLES: &[&str] = &["tokenProgram", "tokenAuthority", "whirlpool", "tokenOwnerAccountA", "tokenVaultA", "tokenOwnerAccountB", "tokenVaultB", "tickArray0", "tickArray1", "tickArray2", "oracle"];
                let mut map = std::collections::HashMap::new();
                for (i, role) in ROLES.iter().enumerate() {
                    if let Some(ai) = account_indices.get(i) {
                        if let Some(pk) = account_keys.get(*ai as usize) {
                            map.insert((*role).to_string(), pk.clone());
                        }
                    }
                }
                return Some(("swap".to_string(), map));
            }
        
            if &ix_data[..8] == &[114u8, 113u8, 45u8, 226u8, 179u8, 239u8, 106u8, 225u8] {
                // IDL-declared role order for swapV2
                const ROLES: &[&str] = &["tokenProgramA", "tokenProgramB", "memoProgram", "tokenAuthority", "whirlpool", "tokenMintA", "tokenMintB", "tokenOwnerAccountA", "tokenVaultA", "tokenOwnerAccountB", "tokenVaultB", "tickArray0", "tickArray1", "tickArray2", "oracle"];
                let mut map = std::collections::HashMap::new();
                for (i, role) in ROLES.iter().enumerate() {
                    if let Some(ai) = account_indices.get(i) {
                        if let Some(pk) = account_keys.get(*ai as usize) {
                            map.insert((*role).to_string(), pk.clone());
                        }
                    }
                }
                return Some(("swapV2".to_string(), map));
            }
        
            if &ix_data[..8] == &[147u8, 44u8, 53u8, 153u8, 51u8, 79u8, 60u8, 26u8] {
                // IDL-declared role order for twoHopSwap
                const ROLES: &[&str] = &["tokenProgram", "tokenAuthority", "whirlpoolOne", "whirlpoolTwo", "tokenOwnerAccountOneA", "tokenVaultOneA", "tokenOwnerAccountOneB", "tokenVaultOneB", "tokenOwnerAccountTwoA", "tokenVaultTwoA", "tokenOwnerAccountTwoB", "tokenVaultTwoB", "tickArrayOne0", "tickArrayOne1", "tickArrayOne2", "tickArrayTwo0", "tickArrayTwo1", "tickArrayTwo2", "oracleOne", "oracleTwo"];
                let mut map = std::collections::HashMap::new();
                for (i, role) in ROLES.iter().enumerate() {
                    if let Some(ai) = account_indices.get(i) {
                        if let Some(pk) = account_keys.get(*ai as usize) {
                            map.insert((*role).to_string(), pk.clone());
                        }
                    }
                }
                return Some(("twoHopSwap".to_string(), map));
            }
        
            if &ix_data[..8] == &[162u8, 207u8, 246u8, 2u8, 179u8, 76u8, 64u8, 139u8] {
                // IDL-declared role order for twoHopSwapV2
                const ROLES: &[&str] = &["whirlpoolOne", "whirlpoolTwo", "tokenMintInput", "tokenMintIntermediate", "tokenMintOutput", "tokenProgramInput", "tokenProgramIntermediate", "tokenProgramOutput", "tokenOwnerAccountInput", "tokenVaultOneInput", "tokenVaultOneIntermediate", "tokenVaultTwoIntermediate", "tokenVaultTwoOutput", "tokenOwnerAccountOutput", "tokenAuthority", "tickArrayOne0", "tickArrayOne1", "tickArrayOne2", "tickArrayTwo0", "tickArrayTwo1", "tickArrayTwo2", "oracleOne", "oracleTwo", "memoProgram"];
                let mut map = std::collections::HashMap::new();
                for (i, role) in ROLES.iter().enumerate() {
                    if let Some(ai) = account_indices.get(i) {
                        if let Some(pk) = account_keys.get(*ai as usize) {
                            map.insert((*role).to_string(), pk.clone());
                        }
                    }
                }
                return Some(("twoHopSwapV2".to_string(), map));
            }
        
            None
        }

        pub struct Extractor {
            pub program_id: &'static str,
        }

        impl rt::SwapExtractor for Extractor {
            fn program_id(&self) -> &str { self.program_id }

            fn try_extract(&self, ctx: &rt::TxContext) -> Option<rt::NormalizedSwapV1> {
                let name = ctx.decoded_name.as_deref()?;
                match name {
                    
            "swap" => {
                // Pool: first matching role
                let pool_id = ctx.decoded_accounts_by_role
                    .as_ref()
                    .and_then(|m| ["whirlpool", "pool"].iter().find_map(|r| m.get(*r)))
                    .cloned()
                    .unwrap_or_default();
                let pool_resolved = !pool_id.is_empty();

                // User authority (hint)
                let user_auth = ctx.decoded_accounts_by_role
                    .as_ref()
                    .and_then(|m| ["tokenAuthority", "authority", "owner"].iter().find_map(|r| m.get(*r)))
                    .cloned();

                // Infer amounts from meta (owner hint if available)
                let inferred = rt::infer_amounts_from_meta(
                    ctx.token_balance_changes,
                    user_auth.as_deref()
                )?;

                let price = rt::compute_raw_price(inferred.amount_in, inferred.amount_out);

                Some(rt::NormalizedSwapV1 {
                    slot: ctx.slot,
                    block_time_unix: ctx.block_time_unix,
                    signature: ctx.signature.clone(),
                    ix_index: ctx.ix_index,
                    program_id: self.program_id.to_string(),
                    dex: "whirlpool".to_string(),
                    pool_id,
                    swap_kind: rt::SwapKind::Clmm,
                    token_in_mint: inferred.token_in_mint,
                    token_out_mint: inferred.token_out_mint,
                    amount_in: inferred.amount_in,
                    amount_out: inferred.amount_out,
                    fee_bps_estimate: None,
                    price,
                    tick_index: None,
                    sqrt_price_x64: None,
                    user_authority: user_auth,
                    success: ctx.meta_ok,
                    quality: rt::QualityFlags {
                        idl_decoded: ctx.decoded_name.is_some(),
                        from_profile: true,
                        amounts_from_meta: true,
                        pool_resolved,
                    },
                })
            },
            "swapV2" => {
                // Pool: first matching role
                let pool_id = ctx.decoded_accounts_by_role
                    .as_ref()
                    .and_then(|m| ["whirlpool"].iter().find_map(|r| m.get(*r)))
                    .cloned()
                    .unwrap_or_default();
                let pool_resolved = !pool_id.is_empty();

                // User authority (hint)
                let user_auth = ctx.decoded_accounts_by_role
                    .as_ref()
                    .and_then(|m| ["tokenAuthority"].iter().find_map(|r| m.get(*r)))
                    .cloned();

                // Infer amounts from meta (owner hint if available)
                let inferred = rt::infer_amounts_from_meta(
                    ctx.token_balance_changes,
                    user_auth.as_deref()
                )?;

                let price = rt::compute_raw_price(inferred.amount_in, inferred.amount_out);

                Some(rt::NormalizedSwapV1 {
                    slot: ctx.slot,
                    block_time_unix: ctx.block_time_unix,
                    signature: ctx.signature.clone(),
                    ix_index: ctx.ix_index,
                    program_id: self.program_id.to_string(),
                    dex: "whirlpool".to_string(),
                    pool_id,
                    swap_kind: rt::SwapKind::Clmm,
                    token_in_mint: inferred.token_in_mint,
                    token_out_mint: inferred.token_out_mint,
                    amount_in: inferred.amount_in,
                    amount_out: inferred.amount_out,
                    fee_bps_estimate: None,
                    price,
                    tick_index: None,
                    sqrt_price_x64: None,
                    user_authority: user_auth,
                    success: ctx.meta_ok,
                    quality: rt::QualityFlags {
                        idl_decoded: ctx.decoded_name.is_some(),
                        from_profile: true,
                        amounts_from_meta: true,
                        pool_resolved,
                    },
                })
            },
            "twoHopSwap" => {
                // Pool: first matching role
                let pool_id = ctx.decoded_accounts_by_role
                    .as_ref()
                    .and_then(|m| ["whirlpoolOne", "whirlpoolTwo"].iter().find_map(|r| m.get(*r)))
                    .cloned()
                    .unwrap_or_default();
                let pool_resolved = !pool_id.is_empty();

                // User authority (hint)
                let user_auth = ctx.decoded_accounts_by_role
                    .as_ref()
                    .and_then(|m| ["tokenAuthority"].iter().find_map(|r| m.get(*r)))
                    .cloned();

                // Infer amounts from meta (owner hint if available)
                let inferred = rt::infer_amounts_from_meta(
                    ctx.token_balance_changes,
                    user_auth.as_deref()
                )?;

                let price = rt::compute_raw_price(inferred.amount_in, inferred.amount_out);

                Some(rt::NormalizedSwapV1 {
                    slot: ctx.slot,
                    block_time_unix: ctx.block_time_unix,
                    signature: ctx.signature.clone(),
                    ix_index: ctx.ix_index,
                    program_id: self.program_id.to_string(),
                    dex: "whirlpool".to_string(),
                    pool_id,
                    swap_kind: rt::SwapKind::Clmm,
                    token_in_mint: inferred.token_in_mint,
                    token_out_mint: inferred.token_out_mint,
                    amount_in: inferred.amount_in,
                    amount_out: inferred.amount_out,
                    fee_bps_estimate: None,
                    price,
                    tick_index: None,
                    sqrt_price_x64: None,
                    user_authority: user_auth,
                    success: ctx.meta_ok,
                    quality: rt::QualityFlags {
                        idl_decoded: ctx.decoded_name.is_some(),
                        from_profile: true,
                        amounts_from_meta: true,
                        pool_resolved,
                    },
                })
            },
            "twoHopSwapV2" => {
                // Pool: first matching role
                let pool_id = ctx.decoded_accounts_by_role
                    .as_ref()
                    .and_then(|m| ["whirlpoolOne", "whirlpoolTwo"].iter().find_map(|r| m.get(*r)))
                    .cloned()
                    .unwrap_or_default();
                let pool_resolved = !pool_id.is_empty();

                // User authority (hint)
                let user_auth = ctx.decoded_accounts_by_role
                    .as_ref()
                    .and_then(|m| ["tokenAuthority"].iter().find_map(|r| m.get(*r)))
                    .cloned();

                // Infer amounts from meta (owner hint if available)
                let inferred = rt::infer_amounts_from_meta(
                    ctx.token_balance_changes,
                    user_auth.as_deref()
                )?;

                let price = rt::compute_raw_price(inferred.amount_in, inferred.amount_out);

                Some(rt::NormalizedSwapV1 {
                    slot: ctx.slot,
                    block_time_unix: ctx.block_time_unix,
                    signature: ctx.signature.clone(),
                    ix_index: ctx.ix_index,
                    program_id: self.program_id.to_string(),
                    dex: "whirlpool".to_string(),
                    pool_id,
                    swap_kind: rt::SwapKind::Clmm,
                    token_in_mint: inferred.token_in_mint,
                    token_out_mint: inferred.token_out_mint,
                    amount_in: inferred.amount_in,
                    amount_out: inferred.amount_out,
                    fee_bps_estimate: None,
                    price,
                    tick_index: None,
                    sqrt_price_x64: None,
                    user_authority: user_auth,
                    success: ctx.meta_ok,
                    quality: rt::QualityFlags {
                        idl_decoded: ctx.decoded_name.is_some(),
                        from_profile: true,
                        amounts_from_meta: true,
                        pool_resolved,
                    },
                })
            },
                    _ => None,
                }
            }
        }
    