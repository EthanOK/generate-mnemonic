//! Bitcoin SegWit v0 P2WPKH (`bc1q`, BIP84) and Taproot P2TR (`bc1p`, BIP86).

use alloy::signers::local::coins_bip39::{English, Mnemonic};
use bitcoin::bip32::{DerivationPath, Xpriv};
use bitcoin::key::CompressedPublicKey;
use bitcoin::secp256k1::Secp256k1;
use bitcoin::{Address, Network};

/// BIP84 P2WPKH (`bc1q`), BIP86 Taproot (`bc1p`), or both when prefix is generic `bc1…`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BtcAddressKind {
    /// `m/84'/0'/0'/0/0` → addresses start with `bc1q`
    #[default]
    P2wpkh,
    /// `m/86'/0'/0'/0/0` → addresses start with `bc1p`
    P2tr,
    /// Prefix is `bc1…` but not `bc1q` / `bc1p`; try both derivations per mnemonic.
    Both,
}

impl BtcAddressKind {
    pub const fn derivation_path(self) -> &'static str {
        match self {
            Self::P2wpkh => "m/84'/0'/0'/0/0",
            Self::P2tr => "m/86'/0'/0'/0/0",
            Self::Both => "m/84'/0'/0'/0/0", // placeholder; search uses both paths
        }
    }

    pub const fn cli_label(self) -> &'static str {
        match self {
            Self::P2wpkh => "BTC-bc1q",
            Self::P2tr => "BTC-bc1p",
            Self::Both => "BTC-bc1",
        }
    }
}

#[derive(Debug)]
pub enum BtcAddressError {
    Derivation(String),
}

impl std::fmt::Display for BtcAddressError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BtcAddressError::Derivation(e) => write!(f, "BIP32 derivation failed: {e}"),
        }
    }
}

impl std::error::Error for BtcAddressError {}

/// Reused per worker thread during vanity search.
pub struct DeriveContext {
    secp: Secp256k1<bitcoin::secp256k1::All>,
    p2wpkh_path: DerivationPath,
    p2tr_path: DerivationPath,
    kind: BtcAddressKind,
}

impl DeriveContext {
    pub fn new(kind: BtcAddressKind) -> Result<Self, BtcAddressError> {
        let parse = |p: &str| -> Result<DerivationPath, BtcAddressError> {
            p.parse()
                .map_err(|e: bitcoin::bip32::Error| BtcAddressError::Derivation(e.to_string()))
        };
        Ok(Self {
            secp: Secp256k1::new(),
            p2wpkh_path: parse(BtcAddressKind::P2wpkh.derivation_path())?,
            p2tr_path: parse(BtcAddressKind::P2tr.derivation_path())?,
            kind,
        })
    }

    pub fn address_from_mnemonic(&self, m: &Mnemonic<English>) -> Result<String, BtcAddressError> {
        let seed = m
            .to_seed(Some(""))
            .map_err(|e| BtcAddressError::Derivation(e.to_string()))?;
        match self.kind {
            BtcAddressKind::P2wpkh => {
                address_from_seed(&self.secp, &self.p2wpkh_path, BtcAddressKind::P2wpkh, &seed)
            }
            BtcAddressKind::P2tr => {
                address_from_seed(&self.secp, &self.p2tr_path, BtcAddressKind::P2tr, &seed)
            }
            BtcAddressKind::Both => {
                address_from_seed(&self.secp, &self.p2wpkh_path, BtcAddressKind::P2wpkh, &seed)
            }
        }
    }

    /// All addresses to check for a match (`Both` returns P2WPKH then P2TR).
    pub fn addresses_from_mnemonic(
        &self,
        m: &Mnemonic<English>,
    ) -> Result<Vec<String>, BtcAddressError> {
        let seed = m
            .to_seed(Some(""))
            .map_err(|e| BtcAddressError::Derivation(e.to_string()))?;
        match self.kind {
            BtcAddressKind::P2wpkh => Ok(vec![address_from_seed(
                &self.secp,
                &self.p2wpkh_path,
                BtcAddressKind::P2wpkh,
                &seed,
            )?]),
            BtcAddressKind::P2tr => Ok(vec![address_from_seed(
                &self.secp,
                &self.p2tr_path,
                BtcAddressKind::P2tr,
                &seed,
            )?]),
            BtcAddressKind::Both => Ok(vec![
                address_from_seed(&self.secp, &self.p2wpkh_path, BtcAddressKind::P2wpkh, &seed)?,
                address_from_seed(&self.secp, &self.p2tr_path, BtcAddressKind::P2tr, &seed)?,
            ]),
        }
    }
}

fn address_from_seed(
    secp: &Secp256k1<bitcoin::secp256k1::All>,
    path: &DerivationPath,
    kind: BtcAddressKind,
    seed: &[u8],
) -> Result<String, BtcAddressError> {
    let master = Xpriv::new_master(Network::Bitcoin, seed)
        .map_err(|e| BtcAddressError::Derivation(e.to_string()))?;
    let child = master
        .derive_priv(secp, path)
        .map_err(|e| BtcAddressError::Derivation(e.to_string()))?;
    let addr = match kind {
        BtcAddressKind::P2wpkh => {
            let pk = CompressedPublicKey::from_private_key(secp, &child.to_priv())
                .map_err(|e| BtcAddressError::Derivation(e.to_string()))?;
            Address::p2wpkh(&pk, Network::Bitcoin)
        }
        BtcAddressKind::P2tr => {
            let keypair = child.to_keypair(secp);
            let (internal_key, _) = keypair.x_only_public_key();
            Address::p2tr(secp, internal_key, None, Network::Bitcoin)
        }
        BtcAddressKind::Both => {
            return Err(BtcAddressError::Derivation(
                "Both mode is vanity-only".into(),
            ));
        }
    };
    Ok(addr.to_string())
}

pub fn address_from_mnemonic(
    m: &Mnemonic<English>,
    kind: BtcAddressKind,
) -> Result<String, BtcAddressError> {
    DeriveContext::new(kind)?.address_from_mnemonic(m)
}

/// BTC vanity fragments must start with `bc1` (native SegWit / Taproot only).
pub fn validate_vanity_prefix(prefix: &str, kind: BtcAddressKind) -> Result<(), String> {
    let p = prefix.trim().to_ascii_lowercase();
    if !p.is_empty() && !p.starts_with("bc1") {
        return Err("BTC prefix must start with bc1 (native SegWit / Taproot only)".into());
    }
    if kind == BtcAddressKind::P2wpkh
        && p.len() >= 5
        && p.starts_with("bc1q")
        && p.chars().nth(4) == Some('1')
    {
        return Err(
            "prefix \"bc1q1…\" is impossible for BIP84 P2WPKH: the 5th character cannot be '1'. \
             Try bc1q0, bc1q2, bc1qa, or use a bc1p… prefix for Taproot"
                .into(),
        );
    }
    if p.starts_with("bc1p") && kind == BtcAddressKind::P2wpkh {
        return Err(
            "prefix starts with bc1p but BIP84 P2WPKH (bc1q) mode is selected; use a bc1p… prefix for Taproot"
                .into(),
        );
    }
    if p.starts_with("bc1q") && kind == BtcAddressKind::P2tr {
        return Err(
            "prefix starts with bc1q but BIP86 Taproot (bc1p) mode is selected; use a bc1q… prefix for P2WPKH"
                .into(),
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy::signers::local::coins_bip39::{English, Mnemonic};

    #[test]
    fn abandon_mnemonic_bip84_first_address() {
        let phrase = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let m: Mnemonic<English> = phrase.parse().unwrap();
        let addr = address_from_mnemonic(&m, BtcAddressKind::P2wpkh).unwrap();
        assert_eq!(addr, "bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu");
        assert!(addr.starts_with("bc1q"));
    }

    #[test]
    fn abandon_mnemonic_bip86_first_address() {
        let phrase = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let m: Mnemonic<English> = phrase.parse().unwrap();
        let addr = address_from_mnemonic(&m, BtcAddressKind::P2tr).unwrap();
        assert_eq!(
            addr,
            "bc1p5cyxnuxmeuwuvkwfem96lqzszd02n6xdcjrs20cac6yqjjwudpxqkedrcr"
        );
        assert!(addr.starts_with("bc1p"));
    }
}
