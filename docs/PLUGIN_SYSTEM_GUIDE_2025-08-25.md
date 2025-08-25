# DEX Plugin System Developer Guide
## August 25, 2025 - AI Development Session

### 🎯 **OVERVIEW**

The MEVana Plugin System enables **modular, extensible DEX support** through a standardized interface. Each DEX protocol can be implemented as an independent plugin, providing IDL parsing, instruction building, and protocol-specific optimizations.

### 🔌 **PLUGIN INTERFACE**

```rust
pub trait DexPlugin: Send + Sync {
    /// Get plugin metadata
    fn metadata(&self) -> PluginMetadata;
    
    /// Parse IDL and generate instruction builders
    fn parse_idl(&self, idl_content: &str) -> Result<ParsedIdl>;
    
    /// Build swap instruction for this DEX
    fn build_swap_instruction(&self, params: SwapParams) -> Result<SwapInstruction>;
    
    /// Get supported instruction types
    fn supported_instructions(&self) -> Vec<String>;
    
    /// Validate if a program ID belongs to this DEX
    fn is_program_supported(&self, program_id: &str) -> bool;
    
    /// Get DEX-specific quirks and optimizations
    fn get_quirks(&self) -> Vec<String>;
}
```

### 📋 **PLUGIN METADATA**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    pub name: String,           // "Orca Whirlpool Parser"
    pub version: String,        // "1.0.0"
    pub dex_name: String,       // "Orca"
    pub protocol_type: String,  // "CLMM", "AMM", "CLOB", "DLMM"
    pub supported_program_ids: Vec<String>,
    pub author: String,         // "MEVana"
    pub description: String,
}
```

### 🏗️ **BUILT-IN PLUGINS**

#### **1. Orca Whirlpool Plugin**
```rust
impl DexPlugin for OrcaWhirlpoolPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: "Orca Whirlpool Parser".to_string(),
            dex_name: "Orca".to_string(),
            protocol_type: "CLMM".to_string(),
            supported_program_ids: vec![
                "whirLbMi2vNtMeJ6z18WZb98sPNT1L4GzWk2vuyZgq2U".to_string()
            ],
        }
    }
    
    fn get_quirks(&self) -> Vec<String> {
        vec![
            "q64_math".to_string(),
            "tick_arrays".to_string(), 
            "position_nfts".to_string(),
            "fee_tiers".to_string(),
        ]
    }
}
```

**Orca Quirks:**
- **Q64.64 Math**: Fixed-point arithmetic for price calculations
- **Tick Arrays**: Sparse liquidity distribution across price ranges
- **Position NFTs**: Liquidity positions represented as NFTs
- **Dynamic Fees**: Fee tiers based on pool volatility

#### **2. Raydium Plugin**
```rust
impl DexPlugin for RaydiumPlugin {
    fn get_quirks(&self) -> Vec<String> {
        vec![
            "serum_integration".to_string(),
            "farm_rewards".to_string(),
            "concentrated_liquidity".to_string(),
        ]
    }
}
```

**Raydium Quirks:**
- **Serum Integration**: Built on Serum order book infrastructure
- **Farm Rewards**: Dual token rewards for liquidity providers
- **Multiple Versions**: Both legacy AMM and new CLMM protocols

#### **3. Phoenix Plugin**
```rust
impl DexPlugin for PhoenixPlugin {
    fn get_quirks(&self) -> Vec<String> {
        vec![
            "order_book".to_string(),
            "market_makers".to_string(),
            "limit_orders".to_string(),
        ]
    }
}
```

**Phoenix Quirks:**
- **Central Limit Order Book**: Traditional exchange model
- **Market Makers**: Professional MM incentives
- **Gas Optimization**: Batched order operations

#### **4. Meteora Plugin**
```rust
impl DexPlugin for MeteoraPlugin {
    fn get_quirks(&self) -> Vec<String> {
        vec![
            "dynamic_fees".to_string(),
            "bin_steps".to_string(),
            "auto_rebalancing".to_string(),
        ]
    }
}
```

**Meteora Quirks:**
- **Dynamic Fees**: Variable fees based on market conditions
- **Bin Steps**: Discrete liquidity bins like Trader Joe v2
- **Auto-Rebalancing**: Automated liquidity management

### 🔧 **INSTRUCTION BUILDING**

#### **SwapParams Structure**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapParams {
    pub program_id: String,
    pub pool_address: String,
    pub user_authority: String,
    pub token_account_in: String,
    pub token_account_out: String,
    pub amount_in: u64,
    pub minimum_amount_out: u64,
    pub direction: SwapDirection,
    pub slippage_bps: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SwapDirection {
    AToB,  // Token A to Token B
    BToA,  // Token B to Token A
}
```

#### **SwapInstruction Output**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapInstruction {
    pub program_id: String,
    pub accounts: Vec<AccountMeta>,
    pub data: Vec<u8>,           // Serialized instruction data
    pub compute_units: u32,      // Estimated compute units
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountMeta {
    pub pubkey: String,
    pub is_signer: bool,
    pub is_writable: bool,
}
```

### 📚 **IDL PARSING**

#### **ParsedIdl Structure**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedIdl {
    pub program_id: String,
    pub version: String,
    pub instructions: Vec<IdlInstruction>,
    pub accounts: Vec<IdlAccount>,
    pub types: Vec<IdlType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdlInstruction {
    pub name: String,                    // "swap"
    pub discriminator: [u8; 8],         // Anchor discriminator
    pub accounts: Vec<IdlAccountMeta>,   // Required accounts
    pub args: Vec<IdlField>,            // Instruction arguments
}
```

### 🚀 **PLUGIN DEVELOPMENT WORKFLOW**

#### **1. Create Plugin Structure**
```rust
pub struct MyDexPlugin;

impl MyDexPlugin {
    pub fn new() -> Self {
        Self
    }
}

impl DexPlugin for MyDexPlugin {
    // Implement required methods...
}
```

#### **2. Register Plugin**
```rust
// In plugin manager initialization
let my_plugin = Arc::new(MyDexPlugin::new());
plugin_manager.register_plugin(my_plugin)?;
```

#### **3. Plugin Discovery**
```rust
// Automatic plugin loading from directory
let plugin_manager = PluginManager::new(
    "plugins".into(),  // plugin directory
    true              // auto-load enabled
);
plugin_manager.initialize().await?;
```

### 🔍 **PLUGIN SELECTION LOGIC**

```rust
// Get plugin for specific program ID
if let Some(plugin) = plugin_manager.get_plugin_for_program(program_id) {
    let metadata = plugin.metadata();
    println!("Using {} plugin for {}", metadata.dex_name, program_id);
    
    // Build instruction
    let instruction = plugin.build_swap_instruction(swap_params)?;
}

// Get plugin by DEX name
if let Some(plugin) = plugin_manager.get_plugin_by_dex("Orca") {
    let supported_instructions = plugin.supported_instructions();
}
```

### 📊 **PLUGIN PERFORMANCE METRICS**

```rust
// Plugin statistics
let stats = plugin_manager.get_stats();
println!("Total plugins: {}", stats.total_plugins);
println!("Plugins by protocol: {:?}", stats.plugins_by_protocol);

// Example output:
// Total plugins: 4
// Plugins by protocol: {"CLMM": 2, "AMM": 1, "CLOB": 1}
```

### 🔒 **SECURITY CONSIDERATIONS**

#### **Plugin Isolation**
- Each plugin runs in the same process but with isolated state
- No shared mutable data between plugins
- Plugin failures don't affect core registry operation

#### **Validation**
- Program ID validation before instruction building
- Account meta verification for security
- Slippage and amount bounds checking

#### **Sandboxing (Future)**
- WASM-based plugin execution for untrusted code
- Resource limits (memory, compute time)
- Capability-based security model

### 🔮 **FUTURE PLUGIN FEATURES**

#### **Advanced Code Generation**
```rust
// Generate complete Rust modules from IDL
pub fn generate_rust_module(&self, idl: &ParsedIdl) -> Result<String> {
    // Generate:
    // - Account structs with Borsh serialization
    // - Instruction builders with type safety
    // - PDA derivation functions
    // - Error types and handling
}
```

#### **Multi-Version Support**
```rust
pub struct VersionedPlugin {
    pub versions: HashMap<String, Box<dyn DexPlugin>>,
}

impl VersionedPlugin {
    pub fn get_version(&self, version: &str) -> Option<&dyn DexPlugin> {
        self.versions.get(version).map(|p| p.as_ref())
    }
}
```

#### **Hot Reloading**
```rust
// Watch plugin directory for changes
pub async fn watch_plugins(&mut self) -> Result<()> {
    let mut watcher = notify::watcher(Duration::from_secs(1))?;
    watcher.watch(&self.plugin_dir, RecursiveMode::Recursive)?;
    
    while let Ok(event) = watcher.recv().await {
        if let DebouncedEvent::Write(path) = event {
            self.reload_plugin(&path).await?;
        }
    }
}
```

### 📖 **EXAMPLE: ADDING A NEW DEX**

```rust
// 1. Create plugin struct
pub struct NewDexPlugin;

// 2. Implement trait
impl DexPlugin for NewDexPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: "NewDex Parser".to_string(),
            version: "1.0.0".to_string(),
            dex_name: "NewDex".to_string(),
            protocol_type: "AMM".to_string(),
            supported_program_ids: vec!["NewDexProgramId...".to_string()],
            author: "Developer".to_string(),
            description: "Parser for NewDex AMM".to_string(),
        }
    }
    
    fn build_swap_instruction(&self, params: SwapParams) -> Result<SwapInstruction> {
        // Build NewDex-specific swap instruction
        Ok(SwapInstruction {
            program_id: params.program_id,
            accounts: self.derive_swap_accounts(&params)?,
            data: self.serialize_swap_data(&params)?,
            compute_units: 100_000,
        })
    }
    
    fn is_program_supported(&self, program_id: &str) -> bool {
        program_id == "NewDexProgramId..."
    }
    
    // ... implement other methods
}

// 3. Register in plugin manager
let new_dex_plugin = Arc::new(NewDexPlugin);
plugin_manager.register_plugin(new_dex_plugin)?;
```

---

**Status**: ✅ **PLUGIN SYSTEM OPERATIONAL**  
**Extensibility**: ✅ **4 Built-in DEX plugins**  
**Performance**: ✅ **Zero hot-path impact**  
**Future-Ready**: ✅ **External plugin loading framework**  

*Ready for MEVana's production arbitrage trading*
