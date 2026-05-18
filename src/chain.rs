//! Chain / derivation config for CLI flags `--ETH`, `--SOL`, `--BTC`.

use alloy::signers::local::coins_bip39::{English, Mnemonic};

use crate::btc::BtcAddressKind;

/// MetaMask first account / Alloy `MnemonicBuilder` default.
pub const ETH_DEFAULT_PATH: &str = "m/44'/60'/0'/0/0";

/// Phantom / Solflare common first account (SLIP-0010 Ed25519).
pub const SOL_DEFAULT_PATH: &str = "m/44'/501'/0'/0'";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Chain {
    #[default]
    Eth,
    Sol,
    Btc(BtcAddressKind),
}

impl Chain {
    pub const fn derivation_path(self) -> &'static str {
        match self {
            Chain::Eth => ETH_DEFAULT_PATH,
            Chain::Sol => SOL_DEFAULT_PATH,
            Chain::Btc(kind) => kind.derivation_path(),
        }
    }

    pub fn cli_label(self) -> &'static str {
        match self {
            Chain::Eth => "ETH",
            Chain::Sol => "SOL",
            Chain::Btc(kind) => kind.cli_label(),
        }
    }

    pub const fn max_fragment_len(self) -> usize {
        match self {
            Chain::Eth => 40,
            Chain::Sol => 44,
            Chain::Btc(_) => 62,
        }
    }

    pub fn normalize_address_body(self, addr: &str, strict_case: bool) -> String {
        match self {
            Chain::Eth => {
                let b = addr.strip_prefix("0x").unwrap_or(addr);
                if strict_case {
                    b.to_string()
                } else {
                    b.to_lowercase()
                }
            }
            Chain::Sol | Chain::Btc(_) => {
                if strict_case {
                    addr.to_string()
                } else {
                    addr.to_lowercase()
                }
            }
        }
    }

    pub fn address_from_mnemonic(self, m: &Mnemonic<English>) -> Result<String, String> {
        match self {
            Chain::Eth => {
                let path = self.derivation_path();
                crate::eth::address_from_mnemonic_at_path(m, path).map_err(|e| e.to_string())
            }
            Chain::Sol => {
                let path = self.derivation_path();
                crate::sol::address_from_mnemonic_at_path(m, path).map_err(|e| e.to_string())
            }
            Chain::Btc(kind) => {
                if kind == BtcAddressKind::Both {
                    return Err("BTC Both mode is only for vanity search with a bc1… prefix".into());
                }
                crate::btc::address_from_mnemonic(m, kind).map_err(|e| e.to_string())
            }
        }
    }

    pub const fn btc_kind(self) -> Option<BtcAddressKind> {
        match self {
            Chain::Btc(k) => Some(k),
            _ => None,
        }
    }
}
