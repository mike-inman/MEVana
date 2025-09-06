use sha2::Digest;
use std::fs;
use std::path::Path;

#[derive(Debug, serde::Deserialize)]
struct Profile {
    program_name: String,
    program_id: String,
    swap_instructions: Vec<SwapInstr>,
}

#[derive(Debug, serde::Deserialize)]
struct SwapInstr {
    name: String,
    kind: String,
    pool_account_roles: Vec<String>,
    user_authority_roles: Vec<String>,
    #[serde(default)]
    token_accounts_roles_in: Vec<String>,
    #[serde(default)]
    token_accounts_roles_out: Vec<String>,
}

fn discriminator8(name: &str) -> [u8; 8] {
    let mut h = sha2::Sha256::new();
    h.update(format!("global:{}", name));
    let out = h.finalize();
    let mut d = [0u8; 8];
    d.copy_from_slice(&out[..8]);
    d
}

fn generate_extractors(
    profiles_dir: &Path,
    out_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(out_dir)?;

    // Load profiles
    let mut profiles = vec![];
    for entry in fs::read_dir(profiles_dir)? {
        let path = entry?.path();
        if path.extension().and_then(|s| s.to_str()) == Some("yaml") {
            let txt = fs::read_to_string(&path)?;
            let prof: Profile = serde_yaml::from_str(&txt)?;
            profiles.push(prof);
        }
    }

    // Generate modules for each profile
    let mut modules = vec![];
    for profile in &profiles {
        let effective_pid = &profile.program_id;
        if effective_pid.is_empty() {
            eprintln!(
                "WARN: Profile {} has no program_id - skipping",
                profile.program_name
            );
            continue;
        }

        let mod_src = generate_program_module(profile)?;
        let mod_name = format!("program_{}", profile.program_name.replace('-', "_"));
        let mod_path = out_dir.join(format!("{mod_name}.rs"));
        fs::write(&mod_path, mod_src)?;
        modules.push((mod_name, effective_pid.to_string()));
    }

    // Generate registry
    let registry_src = generate_registry(&modules)?;
    fs::write(out_dir.join("registry.rs"), registry_src)?;

    Ok(())
}

fn generate_program_module(profile: &Profile) -> Result<String, Box<dyn std::error::Error>> {
    let mut decode_arms = String::new();
    let mut extract_arms = String::new();

    // Create roles based on profile information
    for instr in &profile.swap_instructions {
        let mut roles = Vec::new();
        roles.extend(instr.pool_account_roles.clone());
        roles.extend(instr.user_authority_roles.clone());
        roles.extend(instr.token_accounts_roles_in.clone());
        roles.extend(instr.token_accounts_roles_out.clone());
        roles.sort();
        roles.dedup();

        // Generate decoder arm
        let disc = discriminator8(&instr.name);
        let disc_bytes = disc
            .iter()
            .map(|b| format!("{}u8", b))
            .collect::<Vec<_>>()
            .join(", ");
        let roles_lit = roles
            .iter()
            .map(|r| format!("\"{}\"", r))
            .collect::<Vec<_>>()
            .join(", ");

        decode_arms.push_str(&format!(
            r#"
            if &ix_data[..8] == &[{disc}] {{
                let mut map = std::collections::HashMap::new();
                for (i, role) in [{roles}].iter().enumerate() {{
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
            roles = roles_lit,
            name = instr.name
        ));

        // Generate extractor arm
        let pool_roles = instr
            .pool_account_roles
            .iter()
            .map(|r| format!("\"{r}\""))
            .collect::<Vec<_>>()
            .join(", ");
        let auth_roles = instr
            .user_authority_roles
            .iter()
            .map(|r| format!("\"{r}\""))
            .collect::<Vec<_>>()
            .join(", ");
        let swap_kind = match instr.kind.as_str() {
            "CLMM" => "rt::SwapKind::Clmm",
            "CPMM" => "rt::SwapKind::Cpmm",
            "Aggregator" => "rt::SwapKind::Aggregator",
            _ => "rt::SwapKind::Other",
        };

        extract_arms.push_str(&format!(
            r#"
            "{name}" => {{
                let pool_id = ctx.decoded_accounts_by_role
                    .as_ref()
                    .and_then(|m| [{pool_roles}].iter().find_map(|r| m.get(*r)))
                    .cloned()
                    .unwrap_or_default();

                let user_auth = ctx.decoded_accounts_by_role
                    .as_ref()
                    .and_then(|m| [{auth_roles}].iter().find_map(|r| m.get(*r)))
                    .cloned();

                let inferred = rt::infer_amounts_from_meta(ctx.token_balance_changes, user_auth.as_deref())?;

                Some(rt::NormalizedSwapV1 {{
                    slot: ctx.slot,
                    block_time_unix: ctx.block_time_unix,
                    signature: ctx.signature.clone(),
                    ix_index: ctx.ix_index,
                    program_id: "{pid}".to_string(),
                    dex: "{dex}".to_string(),
                    pool_id,
                    swap_kind: {swap_kind},
                    token_in_mint: inferred.token_in_mint,
                    token_out_mint: inferred.token_out_mint,
                    amount_in: inferred.amount_in,
                    amount_out: inferred.amount_out,
                    fee_bps_estimate: None,
                    price: (inferred.amount_out as f64) / (inferred.amount_in as f64),
                    tick_index: None,
                    sqrt_price_x64: None,
                    user_authority: user_auth,
                    success: ctx.meta_ok,
                    quality: rt::QualityFlags {{
                        idl_decoded: ctx.decoded_name.is_some(),
                        from_profile: true,
                        amounts_from_meta: true,
                        pool_resolved: !pool_id.is_empty(),
                    }},
                }})
            }}"#,
            name = instr.name,
            pid = profile.program_id,
            dex = profile.program_name,
            pool_roles = pool_roles,
            auth_roles = auth_roles,
            swap_kind = swap_kind
        ));
        extract_arms.push(',');
    }

    let src = format!(
        r#"
        use normalizer_runtime as rt;

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

fn generate_registry(modules: &[(String, String)]) -> Result<String, Box<dyn std::error::Error>> {
    let mut mod_includes = String::new();
    let mut inits = String::new();
    let mut decoder_matches = String::new();
    let mut pid_map_entries = String::new();

    for (mod_name, pid) in modules {
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // TODO: Re-enable IDL sync once network issues are resolved
    // mevana_idl_registry::sync_idls();

    // Locate profiles directory
    let profiles_dir = std::path::PathBuf::from("../normalizer-codegen/profiles");

    // Get the OUT_DIR where Cargo expects build artifacts
    let out_dir = std::env::var("OUT_DIR")?;
    let out_crate = std::path::PathBuf::from(out_dir);

    // Generate the registry and program modules
    generate_extractors(&profiles_dir, &out_crate)?;

    // Write a small lib that re-exports the registry:
    let lib_rs = r#"
        pub mod registry;
        // pub mod program_raydium_clmm; ...
        pub use registry::{registry, decoder_for};
    "#;
    std::fs::write(out_crate.join("lib.rs"), lib_rs)?;

    // Copy registry.rs to the workspace root so normalizer-service can find it
    let workspace_registry =
        std::path::PathBuf::from("../../normalizer-service/src/generated_registry.rs");
    if let Some(parent) = workspace_registry.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::copy(out_crate.join("registry.rs"), &workspace_registry)?;
    println!("cargo:rerun-if-changed={}", workspace_registry.display());

    Ok(())
}

// fn main() {
//     // Ensure IDLs are synced before codegen
//     mevana_idl_registry::sync_idls();

//     // Locate `<workspace>/idls`
//     let idls_dir = std::path::PathBuf::from(std::env::var("IDL_REGISTRY_DIR")
//         .unwrap_or_else(|_| "idls".to_string()));

//     // Do your scanning & generate:
//     //   - src/registry.rs
//     //   - src/program_*/*.rs
//     let out_crate = std::path::PathBuf::from("../normalizer-generated/src");
//     std::fs::create_dir_all(&out_crate).unwrap();

//     // Pseudo-code:
//     let registry_src = codegen::build_registry(&idls_dir)?;
//     std::fs::write(out_crate.join("registry.rs"), registry_src)?;

//     // Optionally write a small lib that re-exports the registry:
//     let lib_rs = r#"
//         pub mod registry;
//         // pub mod program_raydium_clmm; ...
//         pub use registry::{registry, decoder_for};
//     "#;
//     std::fs::write(out_crate.join("lib.rs"), lib_rs).unwrap();
// }
