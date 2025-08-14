pub const NS_POOL: &str = "dex:pool";
pub const NS_PAIR_LATEST: &str = "dex:pair:latest_pool";
pub const NS_PAIR_SYM_LATEST: &str = "dex:pair_symbol:latest_pool";
pub const NS_MINT: &str = "dex:mint";

pub fn key_pool(pool: &str) -> String {
    format!("{NS_POOL}:{pool}")
}
pub fn key_pair_latest(pair: &str) -> String {
    format!("{NS_PAIR_LATEST}:{pair}")
}
pub fn key_pair_sym_latest(pair_sym: &str) -> String {
    format!("{NS_PAIR_SYM_LATEST}:{pair_sym}")
}
pub fn key_mint(mint: &str) -> String {
    format!("{NS_MINT}:{mint}")
}
