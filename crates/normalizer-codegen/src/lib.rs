use anyhow::{Context, Result};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Deserialize)]
struct Profile {
    program_name: String,
    program_id: String,
    idl_file: String,
    swap_instructions: Vec<IxProfile>,
}

#[derive(Debug, Deserialize)]
struct IxProfile {
    name: String,
    kind: String, // "CLMM" | "CPMM" | "AGGREGATOR" | ...
    pool_account_roles: Option<Vec<String>>,
    user_authority_roles: Option<Vec<String>>,
    token_accounts_roles_in: Option<Vec<String>>,
    token_accounts_roles_out: Option<Vec<String>>,
    clmm_state_account: Option<String>,
}

#[derive(Debug)]
#[allow(dead_code)]
struct Idl {
    name: String,
    instructions: Vec<IdlIx>,
}
#[derive(Debug)]
struct IdlIx {
    name: String,
    accounts: Vec<String>, // ordered idl account names
}

fn load_idl(path: &Path) -> Result<Idl> {
    let raw = fs::read_to_string(path).with_context(|| format!("read idl {}", path.display()))?;
    let v: serde_json::Value = serde_json::from_str(&raw)?;
    let name = v
        .get("name")
        .and_then(|x| x.as_str())
        .unwrap_or("unknown")
        .to_string();
    let mut out = Vec::new();
    if let Some(ixs) = v.get("instructions").and_then(|x| x.as_array()) {
        for ix in ixs {
            let ix_name = ix
                .get("name")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            let mut accs = Vec::new();
            if let Some(arr) = ix.get("accounts").and_then(|x| x.as_array()) {
                for a in arr {
                    if let Some(n) = a.get("name").and_then(|x| x.as_str()) {
                        accs.push(n.to_string());
                    }
                }
            }
            out.push(IdlIx {
                name: ix_name,
                accounts: accs,
            });
        }
    }
    Ok(Idl {
        name,
        instructions: out,
    })
}

fn anchor_disc(ix_name: &str) -> [u8; 8] {
    let s = format!("global:{}", ix_name);
    let mut h = Sha256::new();
    h.update(s.as_bytes());
    let d = h.finalize();
    let mut out = [0u8; 8];
    out.copy_from_slice(&d[..8]);
    out
}

//use std::path::{Path, PathBuf};

// fn resolve_idl_path(profiles_dir: &Path, idl_file: &str) -> PathBuf {
//     // profiles_dir = <workspace>/crates/normalizer-codegen/profiles
//     // workspace_root = profiles_dir/../..
//     let workspace_root = profiles_dir
//         .parent()
//         .and_then(|p| p.parent())
//         .unwrap_or_else(|| Path::new("."))
//         .to_path_buf();

//     // Try <workspace>/crates/mevana-idl-registry/idls/<idl_file>
//     let p1 = workspace_root
//         .join("crates")
//         .join("mevana-idl-registry")
//         .join("idls")
//         .join(idl_file);
//     if p1.exists() {
//         return p1;
//     }

//     // Try <workspace>/idls/<idl_file>
//     let p2 = workspace_root.join("idls").join(idl_file);
//     if p2.exists() {
//         return p2;
//     }

//     // Optional: try nested layout dirs you might use
//     let p3 = workspace_root
//         .join("idl-registry")
//         .join(idl_file);
//     if p3.exists() {
//         return p3;
//     }

//     // Nothing matched; return the most likely absolute path (p1) to surface in the error
//     p1
// }

fn resolve_idl_path(profiles_dir: &Path, idl_file: &str) -> PathBuf {
    // profiles_dir = <workspace>/crates/normalizer-codegen/profiles (absolute)
    let workspace_root = profiles_dir
        .parent() // normalizer-codegen
        .and_then(|p| p.parent()) // crates
        .and_then(|p| p.parent()) // workspace root
        .unwrap_or_else(|| Path::new("/"))
        .to_path_buf();

    // 1) crates/mevana-idl-registry/idls/<idl_file>
    let p1 = workspace_root
        .join("crates")
        .join("mevana-idl-registry")
        .join("idls")
        .join(idl_file);
    if p1.exists() {
        return p1;
    }

    // 2) idls/<idl_file> at workspace root
    let p2 = workspace_root.join("idls").join(idl_file);
    if p2.exists() {
        return p2;
    }

    // 3) idl-registry/<idl_file> (optional)
    let p3 = workspace_root.join("idl-registry").join(idl_file);
    if p3.exists() {
        return p3;
    }

    // Nothing found; return p1 so the error shows the most likely path
    p1
}

fn kind_to_enum(s: &str) -> &'static str {
    match s.to_ascii_lowercase().as_str() {
        "clmm" => "SwapKind::Clmm",
        "cpmm" => "SwapKind::Cpmm",
        "aggregator" => "SwapKind::Aggregator",
        _ => "SwapKind::Other",
    }
}

pub fn generate_extractors(
    profiles_dir_in: &std::path::Path,
    _out_dir: &std::path::Path,
) -> Result<String> {
    // Make sure we work with an absolute path
    let profiles_dir = profiles_dir_in
        .canonicalize()
        .unwrap_or_else(|_| profiles_dir_in.to_path_buf());

    // ← THIS was missing
    let mut profiles: Vec<Profile> = Vec::new();

    // Read every YAML in the profiles dir
    for entry in std::fs::read_dir(&profiles_dir)
        .with_context(|| format!("read_dir {}", profiles_dir.display()))?
    {
        let p = entry?.path();
        if p.extension().and_then(|s| s.to_str()) != Some("yaml") {
            continue;
        }
        let txt = std::fs::read_to_string(&p)?;
        let prof: Profile =
            serde_yaml::from_str(&txt).with_context(|| format!("parse profile {}", p.display()))?;
        profiles.push(prof);
    }

    // 2) Load IDLs + precompute discriminator + role index mapping
    struct IxGen {
        ix_name: String,
        disc: [u8; 8],
        // role_key -> list of (alias_name, index_in_accs). We’ll keep the alias_name as the map key in roles.
        positions: Vec<(String, usize)>,
    }
    struct ProgramGen {
        program_id: String,
        dex_name: String,                     // from program_name
        kind_by_ix: BTreeMap<String, String>, // ix_name -> SwapKind::<..> as string
        ixs: Vec<IxGen>,
    }

    let mut progs = Vec::new();

    for prof in profiles {
        let idl_path = resolve_idl_path(&profiles_dir, &prof.idl_file);

        let idl =
            load_idl(&idl_path).with_context(|| format!("load idl {}", idl_path.display()))?;

        // Build a quick lookup: ix_name -> IdlIx
        let mut idl_ix_by_name: BTreeMap<String, &IdlIx> = BTreeMap::new();
        for ix in &idl.instructions {
            idl_ix_by_name.insert(ix.name.clone(), ix);
        }

        let mut ixs: Vec<IxGen> = Vec::new();
        let mut kind_by_ix: BTreeMap<String, String> = BTreeMap::new();

        for sip in &prof.swap_instructions {
            let Some(idl_ix) = idl_ix_by_name.get(&sip.name) else {
                // If missing, just skip; you can make this a hard error if you prefer strictness.
                continue;
            };

            // Build “positions” for all role groups you listed:
            // We record each alias (e.g., "whirlpool", "pool", "whirlpoolOne") that actually exists in the IDL’s account list.
            // At runtime we’ll pull keys[accs[pos]] and insert into the role map under that alias string.
            let mut positions: Vec<(String, usize)> = Vec::new();

            if let Some(list) = &sip.pool_account_roles {
                for alias in list {
                    if let Some(pos) = idl_ix.accounts.iter().position(|a| a == alias) {
                        positions.push((alias.clone(), pos));
                    }
                }
            }
            if let Some(list) = &sip.user_authority_roles {
                for alias in list {
                    if let Some(pos) = idl_ix.accounts.iter().position(|a| a == alias) {
                        positions.push((alias.clone(), pos));
                    }
                }
            }
            if let Some(list) = &sip.token_accounts_roles_in {
                for alias in list {
                    if let Some(pos) = idl_ix.accounts.iter().position(|a| a == alias) {
                        positions.push((alias.clone(), pos));
                    }
                }
            }
            if let Some(list) = &sip.token_accounts_roles_out {
                for alias in list {
                    if let Some(pos) = idl_ix.accounts.iter().position(|a| a == alias) {
                        positions.push((alias.clone(), pos));
                    }
                }
            }
            if let Some(alias) = &sip.clmm_state_account {
                if let Some(pos) = idl_ix.accounts.iter().position(|a| a == alias) {
                    positions.push((alias.clone(), pos));
                }
            }

            ixs.push(IxGen {
                ix_name: sip.name.clone(),
                disc: anchor_disc(&sip.name),
                positions,
            });
            kind_by_ix.insert(sip.name.clone(), kind_to_enum(&sip.kind).to_string());
        }

        progs.push(ProgramGen {
            program_id: prof.program_id,
            dex_name: prof.program_name, // we’ll CamelCase for type name and keep a snake-ish string for dex tag
            kind_by_ix,
            ixs,
        });
    }

    // 3) Emit Rust source
    let mut src = String::new();
    src.push_str(
        r#"
use std::collections::HashMap;
use normalizer_runtime::{
    SwapExtractor, TxContext, NormalizedSwapV1, SwapKind,
    infer_amounts_from_meta, compute_raw_price, QualityFlags,
};
"#,
    );

    // ---- decoder_for switch ----
    src.push_str("pub type DecFn = fn(data: &[u8], account_indices: &[u8], account_keys: &[String]) -> Option<(String, HashMap<String, String>)>;\n");
    src.push_str(
        "pub fn decoder_for(program_id: &str) -> Option<DecFn> {\n    match program_id {\n",
    );
    for pg in &progs {
        src.push_str(&format!(
            "        \"{}\" => Some(decode_{}),\n",
            pg.program_id,
            to_snake(&pg.dex_name)
        ));
    }
    src.push_str("        _ => None,\n    }\n}\n");

    // ---- one decoder per program ----
    for pg in &progs {
        // discriminators
        for ix in &pg.ixs {
            src.push_str(&format!(
                "const DISC_{}_{}: [u8;8] = {:?};\n",
                to_const(&pg.dex_name),
                to_const(&ix.ix_name),
                ix.disc
            ));
        }

        // decoder body
        let has_positions = pg.ixs.iter().any(|ix| !ix.positions.is_empty());
        let accs_param = if has_positions {
            "accs: &[u8]"
        } else {
            "_accs: &[u8]"
        };
        let keys_param = if has_positions {
            "keys: &[String]"
        } else {
            "_keys: &[String]"
        };

        if pg.ixs.is_empty() {
            src.push_str(&format!(
r#"
fn decode_{dex}(_data: &[u8], _accs: &[u8], _keys: &[String]) -> Option<(String, HashMap<String,String>)> {{
    None
}}"#,
                dex = to_snake(&pg.dex_name),
            ));
        } else {
            src.push_str(&format!(
r#"
fn decode_{dex}(data: &[u8], {accs_param}, {keys_param}) -> Option<(String, HashMap<String,String>)> {{
    if data.len() < 8 {{ return None; }}
    let disc: [u8;8] = data[0..8].try_into().ok()?;
    match disc {{"#,
                dex = to_snake(&pg.dex_name),
                accs_param = accs_param,
                keys_param = keys_param,
            ));

            for ix in &pg.ixs {
                let disc_name =
                    format!("DISC_{}_{}", to_const(&pg.dex_name), to_const(&ix.ix_name));
                let mut_modifier = if ix.positions.is_empty() { "" } else { "mut " };
                src.push_str(&format!(
                    "\n        {} => {{\n            let {}roles = HashMap::new();\n",
                    disc_name, mut_modifier
                ));
                for (alias, pos) in &ix.positions {
                    let accessor = if *pos == 0 {
                        "first()"
                    } else {
                        &format!("get({})", pos)
                    };
                    src.push_str(&format!(
                        "            if let Some(ai) = accs.{} {{ if let Some(pk) = keys.get(*ai as usize) {{ roles.insert(\"{}\".to_string(), pk.clone()); }} }}\n",
                        accessor, alias
                    ));
                }
                src.push_str(&format!(
                    "            Some((\"{}\".to_string(), roles))\n        }},\n",
                    ix.ix_name
                ));
            }

            src.push_str("        _ => None,\n    }\n}\n");
        }
    }

    // ---- one SwapExtractor per program ----
    for pg in &progs {
        let ty = format!("{}SwapExtractor", to_camel(&pg.dex_name));
        src.push_str(&format!(
r#"
pub struct {ty};
impl SwapExtractor for {ty} {{
    fn program_id(&self) -> &str {{ "{pid}" }}
    fn try_extract(&self, ctx: &TxContext) -> Option<NormalizedSwapV1> {{
        let ia = infer_amounts_from_meta(ctx.token_balance_changes, None)?;
        // Prefer a single-pool role if present; fall back to any 'whirlpool*' role.
        let pool_id = ctx.decoded_accounts_by_role.as_ref()
            .and_then(|m| m.get("pool")).cloned()
            .or_else(|| ctx.decoded_accounts_by_role.as_ref().and_then(|m| m.get("whirlpool")).cloned())
            .or_else(|| ctx.decoded_accounts_by_role.as_ref().and_then(|m| m.get("whirlpoolOne")).cloned())
            .or_else(|| ctx.decoded_accounts_by_role.as_ref().and_then(|m| m.get("whirlpoolTwo")).cloned())
            .unwrap_or_default();

        Some(NormalizedSwapV1 {{
            slot: ctx.slot,
            block_time_unix: ctx.block_time_unix,
            signature: ctx.signature.clone(),
            ix_index: ctx.ix_index,
            program_id: ctx.program_id.clone(),
            dex: "{dex_tag}".into(),
            pool_id,
            // We could dispatch by decoded_name to pick CLMM/CPMM/… exactly per-ix. For now, prefer CLMM if any ix kind is CLMM.
            swap_kind: {{
                {swap_kind_assignment}
            }},
            token_in_mint: ia.token_in_mint,
            token_out_mint: ia.token_out_mint,
            amount_in: ia.amount_in,
            amount_out: ia.amount_out,
            fee_bps_estimate: None,
            price: compute_raw_price(ia.amount_in, ia.amount_out),
            tick_index: None,
            sqrt_price_x64: None,
            user_authority: ctx.decoded_accounts_by_role.as_ref()
                .and_then(|m| m.get("tokenAuthority")).cloned()
                .or_else(|| ctx.decoded_accounts_by_role.as_ref().and_then(|m| m.get("authority")).cloned())
                .or_else(|| ctx.decoded_accounts_by_role.as_ref().and_then(|m| m.get("owner")).cloned()),
            success: ctx.meta_ok,
            quality: QualityFlags::default(),
        }})
    }}
}}
"#,
            ty = ty,
            pid = pg.program_id,
            dex_tag = to_snake(&pg.dex_name),
            swap_kind_assignment = if pg.kind_by_ix.is_empty() {
                "SwapKind::Other".to_string()
            } else {
                format!("match ctx.decoded_name.as_deref() {{\n                    {}\n                    _ => SwapKind::Other,\n                }}", kind_match(&pg.kind_by_ix))
            },
        ));
    }

    // ---- registry() ----
    src.push_str("pub fn registry() -> Vec<Box<dyn SwapExtractor>> { vec![\n");
    for pg in &progs {
        src.push_str(&format!(
            "    Box::new({}SwapExtractor),\n",
            to_camel(&pg.dex_name)
        ));
    }
    src.push_str("]}\n");

    Ok(src)
}

fn to_camel(s: &str) -> String {
    let mut out = String::new();
    let mut cap = true;
    for ch in s.chars() {
        if !ch.is_alphanumeric() {
            cap = true;
            continue;
        }
        if cap {
            out.push(ch.to_ascii_uppercase());
            cap = false;
        } else {
            out.push(ch);
        }
    }
    out
}
fn to_snake(s: &str) -> String {
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if ch.is_ascii_uppercase() {
            if i != 0 {
                out.push('_');
            }
            out.push(ch.to_ascii_lowercase());
        } else if ch.is_alphanumeric() {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    out
}
fn to_const(s: &str) -> String {
    let mut out = String::new();
    for ch in s.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_uppercase());
        } else {
            out.push('_');
        }
    }
    out
}
fn kind_match(map: &BTreeMap<String, String>) -> String {
    // emits arms like:  "swap" => SwapKind::Clmm, "swapV2" => SwapKind::Clmm, ...
    let mut s = String::new();
    for (ix, kind) in map {
        s.push_str(&format!(
            "Some(\"{}\") => {},\n                    ",
            ix, kind
        ));
    }
    s
}
