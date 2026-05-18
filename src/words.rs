//! Mnemonic word count: default 12; BIP39 allows 12 / 15 / 18 / 21 / 24.

use alloy::signers::local::coins_bip39::{English, Entropy, Mnemonic, MnemonicError};
use rand::{Rng, RngExt};

pub const DEFAULT_WORD_COUNT: usize = 12;

/// Entropy byte length for a given word count (checksum bits excluded).
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
        .map_err(|_| format!("invalid --words value: {s:?}"))?;
    entropy_byte_len(n)
        .ok_or_else(|| format!("--words must be 12, 15, 18, 21, or 24 (BIP39); got {n}"))?;
    Ok(n)
}

/// Strip `--words N` / `-w N` from `argv`; later flags win.
pub fn peel_word_flags(args: &[String]) -> Result<(usize, Vec<String>), String> {
    let mut word_count = DEFAULT_WORD_COUNT;
    let mut out = Vec::with_capacity(args.len());
    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "--words" | "-w" => {
                let v = args
                    .get(i + 1)
                    .ok_or_else(|| "--words / -w requires a word count".to_string())?;
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

fn entropy_from_rng(rng: &mut impl Rng, nbytes: usize) -> Result<Entropy, MnemonicError> {
    match nbytes {
        16 => {
            let mut buf = [0u8; 16];
            rng.fill(&mut buf);
            Ok(Entropy::Sixteen(buf))
        }
        20 => {
            let mut buf = [0u8; 20];
            rng.fill(&mut buf);
            Ok(Entropy::Twenty(buf))
        }
        24 => {
            let mut buf = [0u8; 24];
            rng.fill(&mut buf);
            Ok(Entropy::TwentyFour(buf))
        }
        28 => {
            let mut buf = [0u8; 28];
            rng.fill(&mut buf);
            Ok(Entropy::TwentyEight(buf))
        }
        32 => {
            let mut buf = [0u8; 32];
            rng.fill(&mut buf);
            Ok(Entropy::ThirtyTwo(buf))
        }
        n => Err(MnemonicError::InvalidEntropyLength(n)),
    }
}

pub fn random_mnemonic(word_count: usize) -> Result<Mnemonic<English>, MnemonicError> {
    let nbytes = entropy_byte_len(word_count).ok_or(MnemonicError::InvalidWordCount(word_count))?;
    let mut rng = rand::rng();
    let entropy = entropy_from_rng(&mut rng, nbytes)?;
    Ok(Mnemonic::new_from_entropy(entropy))
}
