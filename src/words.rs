//! 助记词词数：默认 12，可配置为 BIP39 允许的 12 / 15 / 18 / 21 / 24。

use alloy::signers::local::coins_bip39::{English, Mnemonic, MnemonicError};

pub const DEFAULT_WORD_COUNT: usize = 12;

/// 词数对应的熵字节长度（不含校验位）。
pub const fn entropy_byte_len(word_count: usize) -> Option<usize> {
    match word_count {
        12 => Some(16),
        15 => Some(20),
        18 => Some(24),
        21 => Some(28),
        24 => Some(32),
        _ => None,
    }
}

pub fn parse_word_count(s: &str) -> Result<usize, String> {
    let n: usize = s
        .parse()
        .map_err(|_| format!("无效的 --words 参数: {s:?}"))?;
    entropy_byte_len(n).ok_or_else(|| {
        format!("--words 仅支持 12、15、18、21、24（BIP39 标准），收到 {n}")
    })?;
    Ok(n)
}

/// 从 `argv` 中剥离 `--words N` / `-w N`，返回剩余参数与最终词数（以后出现的为准）。
pub fn peel_word_flags(args: &[String]) -> Result<(usize, Vec<String>), String> {
    let mut word_count = DEFAULT_WORD_COUNT;
    let mut out = Vec::with_capacity(args.len());
    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "--words" | "-w" => {
                let v = args
                    .get(i + 1)
                    .ok_or_else(|| "--words / -w 需要紧跟词数".to_string())?;
                word_count = parse_word_count(v)?;
                i += 2;
            }
            other => {
                out.push(other.to_string());
                i += 1;
            }
        }
    }
    Ok((word_count, out))
}

pub fn random_mnemonic(word_count: usize) -> Result<Mnemonic<English>, MnemonicError> {
    let mut rng = rand::thread_rng();
    Mnemonic::new_with_count(&mut rng, word_count)
}
