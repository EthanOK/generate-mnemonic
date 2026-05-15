//! 链 / 派生配置：便于在 CLI 上通过 `--ETH` 等开关扩展其它网络。

/// 与 MetaMask 默认首账户、Alloy `MnemonicBuilder` 默认一致。
pub const ETH_DEFAULT_PATH: &str = "m/44'/60'/0'/0/0";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Chain {
    /// 以太坊（BIP44 coin type 60′）
    #[default]
    Eth,
}

impl Chain {
    /// BIP32 派生路径（当前为账户 `0` 的第一个外链地址）。
    pub const fn derivation_path(self) -> &'static str {
        match self {
            Chain::Eth => ETH_DEFAULT_PATH,
        }
    }

    /// 日志与输出用短标签。
    pub const fn cli_label(self) -> &'static str {
        match self {
            Chain::Eth => "ETH",
        }
    }

    /// 将地址字符串规范成供 vanity 匹配的「主体」片段（去掉 `0x`）。
    /// `strict_case == false` 时转为小写，与默认忽略大小写的前缀/后缀一致；
    /// `strict_case == true` 时保留 EIP-55 校验和大小写（地址需为 checksummed 或等价形式）。
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
        }
    }
}
