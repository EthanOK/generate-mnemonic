//! Solana 地址：BIP39 种子 + SLIP-0010 Ed25519 派生，与 Phantom / Solflare 常用路径一致。

use alloy::signers::local::coins_bip39::{English, Mnemonic};
use ed25519_dalek::SigningKey;
use slip10::{derive_key_from_path, BIP32Path, Curve};
use std::str::FromStr;

pub use crate::chain::SOL_DEFAULT_PATH;

#[derive(Debug)]
pub enum SolAddressError {
    Derivation(String),
}

impl std::fmt::Display for SolAddressError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SolAddressError::Derivation(e) => write!(f, "SLIP-0010 派生失败: {e}"),
        }
    }
}

impl std::error::Error for SolAddressError {}

pub fn address_from_mnemonic_at_path(
    m: &Mnemonic<English>,
    derivation_path: &str,
) -> Result<String, SolAddressError> {
    let seed = m
        .to_seed(Some(""))
        .map_err(|e| SolAddressError::Derivation(e.to_string()))?;
    let path = BIP32Path::from_str(derivation_path)
        .map_err(|e| SolAddressError::Derivation(e.to_string()))?;
    let derived = derive_key_from_path(&seed, Curve::Ed25519, &path)
        .map_err(|e| SolAddressError::Derivation(e.to_string()))?;
    let signing_key = SigningKey::from_bytes(&derived.key);
    let address = bs58::encode(signing_key.verifying_key().to_bytes()).into_string();
    Ok(address)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy::signers::local::coins_bip39::{English, Mnemonic};

    /// 与 `ed25519-hd-key` + `@solana/web3.js` Keypair.fromSeed 对齐（BIP39 → SLIP-0010）。
    #[test]
    fn abandon_mnemonic_phantom_path_matches_ed25519_hd() {
        let phrase = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let m: Mnemonic<English> = phrase.parse().unwrap();
        let addr = address_from_mnemonic_at_path(&m, SOL_DEFAULT_PATH).unwrap();
        assert_eq!(addr, "HAgk14JpMQLgt6rVgv7cBQFJWFto5Dqxi472uT3DKpqk");
    }
}
