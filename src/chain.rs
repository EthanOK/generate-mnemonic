//! 链 / 派生配置：便于在 CLI 上通过 `--ETH` / `--SOL` 等开关选择网络。

use alloy::signers::local::coins_bip39::{English, Mnemonic};

/// 与 MetaMask 默认首账户、Alloy `MnemonicBuilder` 默认一致。
pub const ETH_DEFAULT_PATH: &str = "m/44'/60'/0'/0/0";

/// 与 Phantom / Solflare 首账户常用路径一致（SLIP-0010 Ed25519）。
pub const SOL_DEFAULT_PATH: &str = "m/44'/501'/0'/0'";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Chain {
    /// 以太坊（BIP44 coin type 60′）
    #[default]
    Eth,
    /// Solana（BIP44 coin type 501′）
    Sol,
}

impl Chain {
    /// BIP32 / SLIP-0010 派生路径（当前为账户 `0` 的首地址）。
    pub const fn derivation_path(self) -> &'static str {
        match self {
            Chain::Eth => ETH_DEFAULT_PATH,
            Chain::Sol => SOL_DEFAULT_PATH,
        }
    }

    /// 日志与输出用短标签。
    pub const fn cli_label(self) -> &'static str {
        match self {
            Chain::Eth => "ETH",
            Chain::Sol => "SOL",
        }
    }

    /// 前缀/后缀片段允许的最大长度（匹配用「主体」）。
    pub const fn max_fragment_len(self) -> usize {
        match self {
            Chain::Eth => 40,
            Chain::Sol => 44,
        }
    }

    /// 将地址规范成供 vanity 匹配的「主体」。
    /// ETH：`strict_case == false` 时去掉 `0x` 并小写；`true` 时保留 EIP-55。
    /// SOL：Base58 主体；`strict_case == false` 时整体小写（默认忽略大小写）。
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
            Chain::Sol => {
                if strict_case {
                    addr.to_string()
                } else {
                    addr.to_lowercase()
                }
            }
        }
    }

    pub fn address_from_mnemonic(
        self,
        m: &Mnemonic<English>,
    ) -> Result<String, String> {
        let path = self.derivation_path();
        match self {
            Chain::Eth => crate::eth::address_from_mnemonic_at_path(m, path)
                .map_err(|e| e.to_string()),
            Chain::Sol => crate::sol::address_from_mnemonic_at_path(m, path)
                .map_err(|e| e.to_string()),
        }
    }
}
