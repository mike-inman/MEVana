
        use normalizer_runtime as rt;

        /// Return (instruction_name, accounts_by_role) if this instruction matches a known method.
        pub fn decode_anchor(ix_data: &[u8], account_indices: &[u8], account_keys: &[String])
            -> Option<(String, std::collections::HashMap<String, String>)>
        {
            if ix_data.len() < 8 { return None; }
            
            if &ix_data[..8] == &[229u8, 23u8, 203u8, 151u8, 122u8, 227u8, 173u8, 42u8] {
                // IDL-declared role order for route
                const ROLES: &[&str] = &["tokenProgram", "userTransferAuthority", "destinationTokenAccount"];
                let mut map = std::collections::HashMap::new();
                for (i, role) in ROLES.iter().enumerate() {
                    if let Some(ai) = account_indices.get(i) {
                        if let Some(pk) = account_keys.get(*ai as usize) {
                            map.insert((*role).to_string(), pk.clone());
                        }
                    }
                }
                return Some(("route".to_string(), map));
            }
        
            if &ix_data[..8] == &[77u8, 229u8, 199u8, 187u8, 92u8, 145u8, 7u8, 134u8] {
                // IDL-declared role order for whirlpoolSwap
                const ROLES: &[&str] = &["swapProgram", "tokenProgram", "tokenAuthority", "whirlpool", "tokenOwnerAccountA", "tokenVaultA", "tokenOwnerAccountB", "tokenVaultB", "tickArray0", "tickArray1", "tickArray2", "oracle"];
                let mut map = std::collections::HashMap::new();
                for (i, role) in ROLES.iter().enumerate() {
                    if let Some(ai) = account_indices.get(i) {
                        if let Some(pk) = account_keys.get(*ai as usize) {
                            map.insert((*role).to_string(), pk.clone());
                        }
                    }
                }
                return Some(("whirlpoolSwap".to_string(), map));
            }
        
            if &ix_data[..8] == &[112u8, 235u8, 170u8, 105u8, 219u8, 12u8, 140u8, 83u8] {
                // IDL-declared role order for raydiumSwap
                const ROLES: &[&str] = &["swapProgram", "tokenProgram", "ammId", "ammAuthority", "ammOpenOrders", "poolCoinTokenAccount", "poolPcTokenAccount", "serumProgramId", "serumMarket", "serumBids", "serumAsks", "serumEventQueue", "serumCoinVaultAccount", "serumPcVaultAccount", "serumVaultSigner", "userSourceTokenAccount", "userDestinationTokenAccount", "userSourceOwner"];
                let mut map = std::collections::HashMap::new();
                for (i, role) in ROLES.iter().enumerate() {
                    if let Some(ai) = account_indices.get(i) {
                        if let Some(pk) = account_keys.get(*ai as usize) {
                            map.insert((*role).to_string(), pk.clone());
                        }
                    }
                }
                return Some(("raydiumSwap".to_string(), map));
            }
        
            if &ix_data[..8] == &[186u8, 235u8, 91u8, 26u8, 238u8, 128u8, 195u8, 125u8] {
                // IDL-declared role order for raydiumClmmSwap
                const ROLES: &[&str] = &["swapProgram", "payer", "ammConfig", "poolState", "inputTokenAccount", "outputTokenAccount", "inputVault", "outputVault", "observationState", "tokenProgram", "tickArray"];
                let mut map = std::collections::HashMap::new();
                for (i, role) in ROLES.iter().enumerate() {
                    if let Some(ai) = account_indices.get(i) {
                        if let Some(pk) = account_keys.get(*ai as usize) {
                            map.insert((*role).to_string(), pk.clone());
                        }
                    }
                }
                return Some(("raydiumClmmSwap".to_string(), map));
            }
        
            if &ix_data[..8] == &[154u8, 10u8, 145u8, 107u8, 182u8, 195u8, 218u8, 225u8] {
                // IDL-declared role order for meteoraSwap
                const ROLES: &[&str] = &["swapProgram", "pool", "userSourceToken", "userDestinationToken", "aVault", "bVault", "aTokenVault", "bTokenVault", "aVaultLpMint", "bVaultLpMint", "aVaultLp", "bVaultLp", "adminTokenFee", "user", "vaultProgram", "tokenProgram"];
                let mut map = std::collections::HashMap::new();
                for (i, role) in ROLES.iter().enumerate() {
                    if let Some(ai) = account_indices.get(i) {
                        if let Some(pk) = account_keys.get(*ai as usize) {
                            map.insert((*role).to_string(), pk.clone());
                        }
                    }
                }
                return Some(("meteoraSwap".to_string(), map));
            }
        
            if &ix_data[..8] == &[203u8, 194u8, 12u8, 192u8, 232u8, 82u8, 28u8, 146u8] {
                // IDL-declared role order for phoenixSwap
                const ROLES: &[&str] = &["swapProgram", "logAuthority", "market", "trader", "baseAccount", "quoteAccount", "baseVault", "quoteVault", "tokenProgram"];
                let mut map = std::collections::HashMap::new();
                for (i, role) in ROLES.iter().enumerate() {
                    if let Some(ai) = account_indices.get(i) {
                        if let Some(pk) = account_keys.get(*ai as usize) {
                            map.insert((*role).to_string(), pk.clone());
                        }
                    }
                }
                return Some(("phoenixSwap".to_string(), map));
            }
        
            if &ix_data[..8] == &[183u8, 166u8, 76u8, 79u8, 88u8, 200u8, 51u8, 22u8] {
                // IDL-declared role order for lifinitySwap
                const ROLES: &[&str] = &["swapProgram", "authority", "amm", "userTransferAuthority", "sourceInfo", "destinationInfo", "swapSource", "swapDestination", "poolMint", "feeAccount", "tokenProgram", "pythAccount", "pythPcAccount", "configAccount"];
                let mut map = std::collections::HashMap::new();
                for (i, role) in ROLES.iter().enumerate() {
                    if let Some(ai) = account_indices.get(i) {
                        if let Some(pk) = account_keys.get(*ai as usize) {
                            map.insert((*role).to_string(), pk.clone());
                        }
                    }
                }
                return Some(("lifinitySwap".to_string(), map));
            }
        
            if &ix_data[..8] == &[7u8, 30u8, 184u8, 54u8, 247u8, 107u8, 242u8, 223u8] {
                // IDL-declared role order for invariantSwap
                const ROLES: &[&str] = &["swapProgram", "state", "pool", "tickmap", "accountX", "accountY", "reserveX", "reserveY", "owner", "programAuthority", "tokenProgram"];
                let mut map = std::collections::HashMap::new();
                for (i, role) in ROLES.iter().enumerate() {
                    if let Some(ai) = account_indices.get(i) {
                        if let Some(pk) = account_keys.get(*ai as usize) {
                            map.insert((*role).to_string(), pk.clone());
                        }
                    }
                }
                return Some(("invariantSwap".to_string(), map));
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
                    
            "route" => {
                // Pool: first matching role
                let pool_id = ctx.decoded_accounts_by_role
                    .as_ref()
                    .and_then(|m| ["destinationTokenAccount"].iter().find_map(|r| m.get(*r)))
                    .cloned()
                    .unwrap_or_default();
                let pool_resolved = !pool_id.is_empty();

                // User authority (hint)
                let user_auth = ctx.decoded_accounts_by_role
                    .as_ref()
                    .and_then(|m| ["userTransferAuthority"].iter().find_map(|r| m.get(*r)))
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
                    dex: "jupiter".to_string(),
                    pool_id,
                    swap_kind: rt::SwapKind::Aggregator,
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
            "whirlpoolSwap" => {
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
                    .and_then(|m| ["userTransferAuthority"].iter().find_map(|r| m.get(*r)))
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
                    dex: "jupiter".to_string(),
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
            "raydiumSwap" => {
                // Pool: first matching role
                let pool_id = ctx.decoded_accounts_by_role
                    .as_ref()
                    .and_then(|m| ["amm", "poolState"].iter().find_map(|r| m.get(*r)))
                    .cloned()
                    .unwrap_or_default();
                let pool_resolved = !pool_id.is_empty();

                // User authority (hint)
                let user_auth = ctx.decoded_accounts_by_role
                    .as_ref()
                    .and_then(|m| ["userTransferAuthority"].iter().find_map(|r| m.get(*r)))
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
                    dex: "jupiter".to_string(),
                    pool_id,
                    swap_kind: rt::SwapKind::Cpmm,
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
            "raydiumClmmSwap" => {
                // Pool: first matching role
                let pool_id = ctx.decoded_accounts_by_role
                    .as_ref()
                    .and_then(|m| ["poolState"].iter().find_map(|r| m.get(*r)))
                    .cloned()
                    .unwrap_or_default();
                let pool_resolved = !pool_id.is_empty();

                // User authority (hint)
                let user_auth = ctx.decoded_accounts_by_role
                    .as_ref()
                    .and_then(|m| ["userTransferAuthority"].iter().find_map(|r| m.get(*r)))
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
                    dex: "jupiter".to_string(),
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
            "meteoraSwap" => {
                // Pool: first matching role
                let pool_id = ctx.decoded_accounts_by_role
                    .as_ref()
                    .and_then(|m| ["lbPair"].iter().find_map(|r| m.get(*r)))
                    .cloned()
                    .unwrap_or_default();
                let pool_resolved = !pool_id.is_empty();

                // User authority (hint)
                let user_auth = ctx.decoded_accounts_by_role
                    .as_ref()
                    .and_then(|m| ["userTransferAuthority"].iter().find_map(|r| m.get(*r)))
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
                    dex: "jupiter".to_string(),
                    pool_id,
                    swap_kind: rt::SwapKind::Other,
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
            "phoenixSwap" => {
                // Pool: first matching role
                let pool_id = ctx.decoded_accounts_by_role
                    .as_ref()
                    .and_then(|m| ["market"].iter().find_map(|r| m.get(*r)))
                    .cloned()
                    .unwrap_or_default();
                let pool_resolved = !pool_id.is_empty();

                // User authority (hint)
                let user_auth = ctx.decoded_accounts_by_role
                    .as_ref()
                    .and_then(|m| ["userTransferAuthority"].iter().find_map(|r| m.get(*r)))
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
                    dex: "jupiter".to_string(),
                    pool_id,
                    swap_kind: rt::SwapKind::Other,
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
            "lifinitySwap" => {
                // Pool: first matching role
                let pool_id = ctx.decoded_accounts_by_role
                    .as_ref()
                    .and_then(|m| ["pool"].iter().find_map(|r| m.get(*r)))
                    .cloned()
                    .unwrap_or_default();
                let pool_resolved = !pool_id.is_empty();

                // User authority (hint)
                let user_auth = ctx.decoded_accounts_by_role
                    .as_ref()
                    .and_then(|m| ["userTransferAuthority"].iter().find_map(|r| m.get(*r)))
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
                    dex: "jupiter".to_string(),
                    pool_id,
                    swap_kind: rt::SwapKind::Other,
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
            "invariantSwap" => {
                // Pool: first matching role
                let pool_id = ctx.decoded_accounts_by_role
                    .as_ref()
                    .and_then(|m| ["pool"].iter().find_map(|r| m.get(*r)))
                    .cloned()
                    .unwrap_or_default();
                let pool_resolved = !pool_id.is_empty();

                // User authority (hint)
                let user_auth = ctx.decoded_accounts_by_role
                    .as_ref()
                    .and_then(|m| ["userTransferAuthority"].iter().find_map(|r| m.get(*r)))
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
                    dex: "jupiter".to_string(),
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
    