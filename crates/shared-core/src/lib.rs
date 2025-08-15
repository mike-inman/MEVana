use sha2::{Digest, Sha256};

pub fn anchor_disc(name: &str) -> [u8; 8] {
    let mut h = Sha256::new();
    h.update(format!("global:{name}"));
    let x = h.finalize();
    let mut d = [0u8; 8];
    d.copy_from_slice(&x[..8]);
    d
}
