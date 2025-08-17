
        use normalizer_runtime as rt;

        /// Return (instruction_name, accounts_by_role) if this instruction matches a known method.
        pub fn decode_anchor(ix_data: &[u8], account_indices: &[u8], account_keys: &[String])
            -> Option<(String, std::collections::HashMap<String, String>)>
        {
            if ix_data.len() < 8 { return None; }
            
            if &ix_data[..8] == &[248u8, 198u8, 158u8, 145u8, 225u8, 117u8, 135u8, 200u8] {
                // IDL-declared role order for swap
                const ROLES: &[&str] = &[];
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
                    .and_then(|m| ["amm", "pool_state", "pool"].iter().find_map(|r| m.get(*r)))
                    .cloned()
                    .unwrap_or_default();
                let pool_resolved = !pool_id.is_empty();

                // User authority (hint)
                let user_auth = ctx.decoded_accounts_by_role
                    .as_ref()
                    .and_then(|m| ["user_owner", "authority", "signer"].iter().find_map(|r| m.get(*r)))
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
                    dex: "raydium_amm".to_string(),
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
                    _ => None,
                }
            }
        }
    