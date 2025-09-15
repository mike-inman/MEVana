# IDL Registry System Documentation
Last updated: 2025-09-15

## Overview

The `mevana-idl-registry` crate provides an automated build-time system for fetching and maintaining Solana DEX IDL files in the shared workspace `idls/` directory.

## How It Works

### 1. Automatic IDL Fetching
The system attempts to fetch IDLs using multiple fallback strategies:

1. **On-chain IDL** - Uses `anchor idl fetch <program_id>`
2. **Anchor Repository Build** - Clones GitHub repos and runs `anchor build`
3. **Fallback Git Repository** - Downloads pre-built IDLs from Git repos

### 2. Integration with normalizer-service

The IDL registry is integrated into `normalizer-service/build.rs`:

```rust
fn main() {
    // Ensure IDLs are present before compiling this crate.
    mevana_idl_registry::sync_idls();
    
    // ... rest of build process
}
```

This ensures IDLs are automatically fetched/updated before the normalizer builds.

### 3. Configuration Options

#### Environment Variables
- `IDL_REGISTRY_DIR` - Override output directory (defaults to `<workspace>/idls`)
- `MEVANA_IDL_CONFIG` - Path to custom JSON config file
- `ANCHOR_PROVIDER_URL` - Solana RPC endpoint for on-chain fetching

#### Custom Configuration File
Create a JSON file with DEX definitions:

```json
[
  {
    "name": "saber_stable_swap",
    "program_id": "SSwpkEEGUMDSSx7AMXKN85Ec4NUP6Nc8LbFnqJzXKzr",
    "anchor_repo": "https://github.com/saber-hq/stable-swap",
    "expected_idl_names": ["stable_swap.json", "saber_stable_swap.json"]
  }
]
```

## Current Status

### ✅ Successfully Auto-Fetched IDLs
- `whirlpool_idl.json` - Orca Whirlpools (on-chain)
- `openbook.json` - OpenBook v2 (on-chain)  
- `goosefx.json` - GooseFX SSL (on-chain)

### ❌ Failed Auto-Fetch (Manual IDLs Used)
- `jupiter_idl.json` - No on-chain IDL available
- `phoenix.json` - Repository build issues (we have manual copy)
- `meteora_damm.json` - Repository build issues (we have manual copy)
- `raydium_amm.json` - Repository build issues (we have manual copy)
- `raydium_clmm.json` - Repository build issues (we have manual copy)
- `pump.json` - No on-chain IDL (we have manual copy)
- `pump_amm.json` - No on-chain IDL (we have manual copy)

### 🔍 Missing/Deprecated DEXs
- `saber_stable_swap` - Uses old `build-bpf` command
- `lifinity_v2` - Repository not found
- `invariant` - Repository access issues
- `serum_dex` - Legacy, superseded by OpenBook

## Usage

### Manual IDL Sync
```bash
# Use default configuration
cargo run -p mevana-idl-registry --bin idl-sync

# Use custom configuration
env MEVANA_IDL_CONFIG=custom-config.json cargo run -p mevana-idl-registry --bin idl-sync
```

### Automatic Build Integration
IDLs are automatically synced when building `normalizer-service`:

```bash
cargo build -p normalizer-service
```

## File Locations

- **Source Code**: `crates/mevana-idl-registry/`
- **IDL Output**: `idls/` (workspace root)
- **Configuration**: `idl-config.json` (workspace root)
- **Build Integration**: `crates/normalizer-service/build.rs`

## Benefits

1. **Automated Maintenance** - IDLs stay up-to-date with upstream changes
2. **Build Integration** - Seamless integration with existing build process  
3. **Fallback Strategies** - Multiple methods ensure maximum success rate
4. **Configurable** - Easy to add new DEXs or customize behavior
5. **CI/CD Ready** - Can be integrated into automated build pipelines

## CI/CD Integration

Add to your CI pipeline:

```yaml
- name: Build IDL Registry
  run: cargo build -p mevana-idl-registry

- name: Sync IDLs  
  run: cargo run -p mevana-idl-registry --bin idl-sync --quiet
```

## Current IDL Coverage

**Total DEX Plugins**: 10  
**Auto-Fetched IDLs**: 3  
**Manual IDLs**: 7  
**Coverage**: 100% (all required IDLs available)

The system successfully maintains our IDL collection and can automatically fetch updates for supported DEXs while gracefully falling back to manual IDLs for others.
