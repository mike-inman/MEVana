use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{fs, path::Path};

#[derive(Debug, Deserialize)]
pub struct Profile {
    pub program_name: String,
    pub program_id: String,
    pub swap_instructions: Vec<SwapInstr>,
}

fn discriminator8(name: &str) -> [u8; 8] {
    // Anchor discriminator: first 8 bytes of sha256("global:<name>")
    let mut h = Sha256::new();
    h.update(format!("global:{}", name));
    let out = h.finalize();
    let mut d = [0u8; 8];
    d.copy_from_slice(&out[..8]);
    d
}

fn flatten_account_names(v: &serde_json::Value, out: &mut Vec<String>) {
    let Some(arr) = v.as_array() else {
        return;
    };
    for entry in arr {
        if let Some(name) = entry.get("name").and_then(|x| x.as_str()) {
            out.push(name.to_string());
        }
        if let Some(sub) = entry.get("accounts") {
            flatten_account_names(sub, out);
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct SwapInstr {
    pub name: String,
    pub kind: String, // "CLMM" | "CPMM" | "Aggregator" | "Other"
    pub pool_account_roles: Vec<String>,
    pub user_authority_roles: Vec<String>,
    #[serde(default)]
    pub token_accounts_roles_in: Vec<String>,
    #[serde(default)]
    pub token_accounts_roles_out: Vec<String>,
    #[serde(default)]
    pub clmm_state_account: Option<String>,
}

#[derive(Debug)]
struct Idl {
    program_name: String,
    program_id: String,
    raw: serde_json::Value, // we’ll parse more deeply in Step 3
}

fn load_idls(idl_dir: &Path) -> Result<Vec<Idl>> {
    let mut out = vec![];
    for entry in fs::read_dir(idl_dir)? {
        let path = entry?.path();
        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            let txt = fs::read_to_string(&path)?;
            let val: serde_json::Value = serde_json::from_str(&txt)?;
            // Best effort program_id/name (Anchor IDLs have "metadata" or "address")
            let program_id = val
                .pointer("/metadata/address")
                .or_else(|| val.pointer("/address"))
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let program_name = val
                .pointer("/name")
                .and_then(|v| v.as_str())
                .unwrap_or_else(|| path.file_stem().unwrap().to_str().unwrap())
                .to_string();
            out.push(Idl {
                program_name,
                program_id,
                raw: val,
            });
        }
    }
    Ok(out)
}

fn load_profiles(profiles_dir: &Path) -> Result<Vec<Profile>> {
    let mut out = vec![];
    for entry in fs::read_dir(profiles_dir)? {
        let path = entry?.path();
        if path.extension().and_then(|s| s.to_str()) == Some("yaml") {
            let txt = fs::read_to_string(&path)?;
            let prof: Profile = serde_yaml::from_str(&txt)?;
            out.push(prof);
        }
    }
    Ok(out)
}

pub fn generate(idl_dir: &Path, profiles_dir: &Path, out_dir: &Path) -> Result<()> {
    fs::create_dir_all(out_dir)?;

    let idls = load_idls(idl_dir).context("loading IDLs")?;
    let profiles = load_profiles(profiles_dir).context("loading profiles")?;

    // Join profiles to idls by program_id or program_name
    let mut modules = vec![];
    for profile in &profiles {
        fn norm(s: &str) -> String {
            s.to_ascii_lowercase().replace(['-', '_'], "")
        }

        let maybe_idl = idls.iter().find(|i| {
    // Prefer PID match when both sides have it
    (!profile.program_id.is_empty() && !i.program_id.is_empty() && i.program_id == profile.program_id)
    // Else fall back to normalized name match ("orca_whirlpool" ↔ "whirlpool")
    || norm(&i.program_name) == norm(&profile.program_name)
});

        let Some(idl) = maybe_idl else {
            eprintln!(
                "WARN: No IDL match for profile {} (pid: {}). Loaded IDLs: {}",
                profile.program_name,
                profile.program_id,
                idls.iter()
                    .map(|x| format!("{}:{}", x.program_name, x.program_id))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            continue; // skip instead of error
        };

        // If the IDL lacks an address, fall back to the profile’s program_id
        let effective_pid = if idl.program_id.is_empty() {
            &profile.program_id
        } else {
            &idl.program_id
        };

        // ...when you push to `modules`, use effective_pid:
        //

        // let maybe_idl = idls.iter().find(|i| {
        //     (!profile.program_id.is_empty() && i.program_id == profile.program_id)
        //         || i.program_name == profile.program_name
        // });

        // let Some(idl) = maybe_idl else {
        //     eprintln!(
        //         "WARN: No IDL match for profile {} (pid: {}) — skipping",
        //         profile.program_name, profile.program_id
        //     );
        //     continue;
        // };

        let mod_src = render_program_module(&profile, idl)?;
        let mod_name = format!("program_{}", profile.program_name.replace('-', "_"));
        let mod_path = out_dir.join(format!("{mod_name}.rs"));
        fs::write(&mod_path, mod_src)?;
        // modules.push((mod_name, profile.program_id.clone()));
        modules.push((mod_name, effective_pid.to_string()));
    }

    eprintln!("INFO: Loaded IDLs:");
    for i in &idls {
        eprintln!("  - name='{}' pid='{}'", i.program_name, i.program_id);
    }

    eprintln!("INFO: Loaded profiles:");
    for p in &profiles {
        eprintln!("  - name='{}' pid='{}'", p.program_name, p.program_id);
    }

    // registry.rs
    let registry_src = render_registry(&modules)?;
    fs::write(out_dir.join("registry.rs"), registry_src)?;

    Ok(())
}

fn render_program_module(profile: &Profile, idl: &Idl) -> Result<String> {
    // Build decode arms only for the swap instructions listed in the profile
    let mut decode_arms = String::new();
    let mut extract_arms = String::new();

    // Look up IDL instructions map: name -> flattened account roles
    let mut idl_roles: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    if let Some(instrs) = idl.raw.get("instructions").and_then(|x| x.as_array()) {
        for i in instrs {
            if let Some(name) = i.get("name").and_then(|x| x.as_str()) {
                let mut roles = Vec::new();
                if let Some(accs) = i.get("accounts") {
                    flatten_account_names(accs, &mut roles);
                }
                idl_roles.insert(name.to_string(), roles);
            }
        }
    }

    for instr in &profile.swap_instructions {
        // ------------- Decoder arm -------------
        let disc = discriminator8(&instr.name);
        let disc_bytes = disc
            .iter()
            .map(|b| format!("{}u8", b))
            .collect::<Vec<_>>()
            .join(", ");
        let roles = idl_roles.get(&instr.name).cloned().unwrap_or_default();
        let roles_lit = roles
            .iter()
            .map(|r| format!("\"{}\"", r))
            .collect::<Vec<_>>()
            .join(", ");

        decode_arms.push_str(&format!(
            r#"
            if &ix_data[..8] == &[{disc}] {{
                // IDL-declared role order for {name}
                const ROLES: &[&str] = &[{roles}];
                let mut map = std::collections::HashMap::new();
                for (i, role) in ROLES.iter().enumerate() {{
                    if let Some(ai) = account_indices.get(i) {{
                        if let Some(pk) = account_keys.get(*ai as usize) {{
                            map.insert((*role).to_string(), pk.clone());
                        }}
                    }}
                }}
                return Some(("{name}".to_string(), map));
            }}
        "#,
            disc = disc_bytes,
            name = instr.name,
            roles = roles_lit
        ));

        // ------------- Extractor arm -------------
        extract_arms.push_str(&format!(
            r#"
            "{name}" => {{
                // Pool: first matching role
                let pool_id = ctx.decoded_accounts_by_role
                    .as_ref()
                    .and_then(|m| [{pool_roles}].iter().find_map(|r| m.get(*r)))
                    .cloned()
                    .unwrap_or_default();
                let pool_resolved = !pool_id.is_empty();

                // User authority (hint)
                let user_auth = ctx.decoded_accounts_by_role
                    .as_ref()
                    .and_then(|m| [{auth_roles}].iter().find_map(|r| m.get(*r)))
                    .cloned();

                // Infer amounts from meta (owner hint if available)
                let inferred = rt::infer_amounts_from_meta(
                    ctx.token_balance_changes,
                    user_auth.as_deref()
                )?;

                let price = rt::compute_raw_price(inferred.amount_in, inferred.amount_out);

                Some(rt::NormalizedSwapV1 {{
                    slot: ctx.slot,
                    block_time_unix: ctx.block_time_unix,
                    signature: ctx.signature.clone(),
                    ix_index: ctx.ix_index,
                    program_id: self.program_id.to_string(),
                    dex: "{dex}".to_string(),
                    pool_id,
                    swap_kind: {swap_kind},
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
                    quality: rt::QualityFlags {{
                        idl_decoded: ctx.decoded_name.is_some(),
                        from_profile: true,
                        amounts_from_meta: true,
                        pool_resolved,
                    }},
                }})
            }}"#,
            name = instr.name,
            dex = profile.program_name,
            pool_roles = instr
                .pool_account_roles
                .iter()
                .map(|r| format!("\"{r}\""))
                .collect::<Vec<_>>()
                .join(", "),
            auth_roles = instr
                .user_authority_roles
                .iter()
                .map(|r| format!("\"{r}\""))
                .collect::<Vec<_>>()
                .join(", "),
            swap_kind = match instr.kind.as_str() {
                "CLMM" => "rt::SwapKind::Clmm",
                "CPMM" => "rt::SwapKind::Cpmm",
                "Aggregator" => "rt::SwapKind::Aggregator",
                _ => "rt::SwapKind::Other",
            },
        ));
        extract_arms.push(',');
    }

    let src = format!(
        r#"
        use normalizer_runtime as rt;

        /// Return (instruction_name, accounts_by_role) if this instruction matches a known method.
        pub fn decode_anchor(ix_data: &[u8], account_indices: &[u8], account_keys: &[String])
            -> Option<(String, std::collections::HashMap<String, String>)>
        {{
            if ix_data.len() < 8 {{ return None; }}
            {decode_arms}
            None
        }}

        pub struct Extractor {{
            pub program_id: &'static str,
        }}

        impl rt::SwapExtractor for Extractor {{
            fn program_id(&self) -> &str {{ self.program_id }}

            fn try_extract(&self, ctx: &rt::TxContext) -> Option<rt::NormalizedSwapV1> {{
                let name = ctx.decoded_name.as_deref()?;
                match name {{
                    {extract_arms}
                    _ => None,
                }}
            }}
        }}
    "#
    );

    Ok(src)
}

fn render_registry(mods: &[(String, String)]) -> Result<String> {
    let mut mod_includes = String::new();
    let mut inits = String::new();
    let mut decoder_matches = String::new();
    let mut pid_map_entries = String::new();

    for (mod_name, pid) in mods {
        mod_includes.push_str(&format!(
            "mod {m} {{ include!(concat!(env!(\"OUT_DIR\"), \"/{m}.rs\")); }}\n",
            m = mod_name
        ));
        inits.push_str(&format!(
            "boxes.push(Box::new({m}::Extractor {{ program_id: PROGRAM_IDS.get(\"{m}\").unwrap() }}));\n",
            m = mod_name
        ));
        decoder_matches.push_str(&format!(
            "\"{pid}\" => Some({m}::decode_anchor),\n",
            pid = pid,
            m = mod_name
        ));
        pid_map_entries.push_str(&format!(
            "map.insert(\"{m}\", \"{pid}\");\n",
            m = mod_name,
            pid = pid
        ));
    }

    let src = format!(
        r#"
        {mod_includes}

        static PROGRAM_IDS: once_cell::sync::Lazy<std::collections::HashMap<&'static str, &'static str>> =
            once_cell::sync::Lazy::new(|| {{
                let mut map: std::collections::HashMap<&'static str, &'static str> = std::collections::HashMap::new();
                {pid_map_entries}
                map
            }});

        pub fn registry() -> Vec<Box<dyn normalizer_runtime::SwapExtractor>> {{
            let mut boxes: Vec<Box<dyn normalizer_runtime::SwapExtractor>> = Vec::new();
            {inits}
            boxes
        }}

        /// Get a decoder function for this program id, if available.
        pub fn decoder_for(pid: &str) -> Option<
            fn(&[u8], &[u8], &[String]) -> Option<(String, std::collections::HashMap<String, String>)>
        > {{
            match pid {{
                {decoder_matches}
                _ => None,
            }}
        }}
    "#
    );

    Ok(src)
}
