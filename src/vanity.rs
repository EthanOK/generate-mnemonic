//! Multi-threaded brute force: random mnemonic → derive address → match prefix/suffix.

use alloy::signers::local::coins_bip39::{English, Mnemonic};
use rand::RngExt;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use crate::btc::BtcAddressKind;
use crate::chain::Chain;

/// BIP173 bech32 data charset (SegWit `bc1…` payload).
const BECH32_CHARSET: &str = "qpzry9x8gf2tvdw0s3jn54khce6mua7l";

fn is_base58_char(c: char) -> bool {
    matches!(c,
        '1'..='9' | 'A'..='H' | 'J'..='N' | 'P'..='Z' | 'a'..='k' | 'm'..='z'
    )
}

fn is_bech32_char(c: char) -> bool {
    let c = c.to_ascii_lowercase();
    c == 'b' || c == 'c' || c == '1' || BECH32_CHARSET.contains(c)
}

fn normalize_hex_fragment(s: &str, strict_case: bool, max_len: usize) -> Result<String, String> {
    let t = s.trim();
    let t = t.strip_prefix("0x").unwrap_or(t);
    if t.is_empty() {
        return Ok(String::new());
    }
    if !t.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(format!("non-hex character: {s:?}"));
    }
    if t.len() > max_len {
        return Err(format!(
            "prefix/suffix length cannot exceed {max_len} hex characters (ETH address body)"
        ));
    }
    Ok(if strict_case {
        t.to_string()
    } else {
        t.to_lowercase()
    })
}

fn normalize_base58_fragment(
    s: &str,
    strict_case: bool,
    max_len: usize,
    chain_label: &str,
) -> Result<String, String> {
    let t = s.trim();
    if t.is_empty() {
        return Ok(String::new());
    }
    if !t.chars().all(is_base58_char) {
        return Err(format!(
            "non-Base58 character ({chain_label} addresses exclude 0/O/I/l): {s:?}"
        ));
    }
    if t.len() > max_len {
        return Err(format!(
            "prefix/suffix length cannot exceed {max_len} Base58 characters ({chain_label})"
        ));
    }
    Ok(if strict_case {
        t.to_string()
    } else {
        t.to_lowercase()
    })
}

fn normalize_bech32_fragment(
    kind: BtcAddressKind,
    s: &str,
    strict_case: bool,
    max_len: usize,
) -> Result<String, String> {
    let t = s.trim();
    if t.is_empty() {
        return Ok(String::new());
    }
    crate::btc::validate_vanity_prefix(t, kind)?;
    if !t.chars().all(is_bech32_char) {
        return Err(format!(
            "non-Bech32 character (BTC native SegWit uses bc1 + BIP173 charset): {s:?}"
        ));
    }
    if t.len() > max_len {
        return Err(format!(
            "prefix/suffix length cannot exceed {max_len} characters (BTC bc1 address)"
        ));
    }
    Ok(if strict_case {
        t.to_string()
    } else {
        t.to_lowercase()
    })
}

fn normalize_fragment(chain: Chain, s: &str, strict_case: bool) -> Result<String, String> {
    let max_len = chain.max_fragment_len();
    match chain {
        Chain::Eth => normalize_hex_fragment(s, strict_case, max_len),
        Chain::Sol => normalize_base58_fragment(s, strict_case, max_len, "SOL"),
        Chain::Btc(kind) => normalize_bech32_fragment(kind, s, strict_case, max_len),
    }
}

fn body_matches(body: &str, prefix: &str, suffix: &str) -> bool {
    if !prefix.is_empty() && !body.starts_with(prefix) {
        return false;
    }
    if !suffix.is_empty() && !body.ends_with(suffix) {
        return false;
    }
    true
}

pub struct VanityConfig {
    pub chain: Chain,
    pub word_count: usize,
    /// `false`: case-insensitive match for prefix/suffix and address body (default).
    /// `true`: exact match (ETH EIP-55, SOL Base58, BTC Bech32).
    pub strict_case: bool,
    pub prefix: String,
    pub suffix: String,
    pub threads: usize,
    /// Stop after this many matches; default 1.
    pub match_count: usize,
}

pub fn search_vanity_mnemonic(
    cfg: VanityConfig,
) -> Result<Vec<(Mnemonic<English>, String)>, String> {
    if cfg.prefix.is_empty() && cfg.suffix.is_empty() {
        return Err("at least one of --prefix or --suffix is required".into());
    }

    let match_count = cfg.match_count.max(1);
    let threads = cfg.threads.max(1);
    let stop = Arc::new(AtomicBool::new(false));
    let found = Arc::new(AtomicUsize::new(0));
    let attempts = Arc::new(AtomicU64::new(0));
    let (tx, rx) = mpsc::sync_channel(match_count);

    let strict_case = cfg.strict_case;
    let word_count = cfg.word_count;
    let chain = cfg.chain;
    let prefix = cfg.prefix;
    let suffix = cfg.suffix;

    let progress = chain.btc_kind().map(|_| {
        let stop = Arc::clone(&stop);
        let attempts = Arc::clone(&attempts);
        thread::spawn(move || {
            let start = Instant::now();
            while !stop.load(Ordering::Relaxed) {
                thread::sleep(Duration::from_secs(5));
                if stop.load(Ordering::Relaxed) {
                    break;
                }
                let n = attempts.load(Ordering::Relaxed);
                let secs = start.elapsed().as_secs_f64().max(0.001);
                eprintln!("… {n} mnemonics tried ({:.1}/s)", n as f64 / secs);
            }
        })
    });

    thread::scope(|s| {
        for _ in 0..threads {
            let stop = Arc::clone(&stop);
            let found = Arc::clone(&found);
            let attempts = Arc::clone(&attempts);
            let tx = tx.clone();
            let prefix = prefix.clone();
            let suffix = suffix.clone();
            s.spawn(move || {
                let btc_ctx = chain
                    .btc_kind()
                    .and_then(|k| crate::btc::DeriveContext::new(k).ok());
                let mut rng = rand::rng();
                while !stop.load(Ordering::Relaxed) {
                    let batch = rng.random_range(64usize..256usize);
                    for _ in 0..batch {
                        let m = match crate::words::random_mnemonic(word_count) {
                            Ok(m) => m,
                            Err(_) => continue,
                        };
                        if btc_ctx.is_some() {
                            attempts.fetch_add(1, Ordering::Relaxed);
                        }
                        let addrs = if let Some(ctx) = &btc_ctx {
                            match ctx.addresses_from_mnemonic(&m) {
                                Ok(a) => a,
                                Err(_) => continue,
                            }
                        } else {
                            match chain.address_from_mnemonic(&m) {
                                Ok(a) => vec![a],
                                Err(_) => continue,
                            }
                        };
                        for addr in addrs {
                            let body = chain.normalize_address_body(&addr, strict_case);
                            if body_matches(&body, &prefix, &suffix) {
                                let idx = found.fetch_add(1, Ordering::SeqCst);
                                if idx < match_count {
                                    let _ = tx.send((m, addr));
                                }
                                if idx + 1 >= match_count {
                                    stop.store(true, Ordering::SeqCst);
                                }
                                if stop.load(Ordering::Relaxed) {
                                    return;
                                }
                                break;
                            }
                        }
                    }
                }
            });
        }
    });

    stop.store(true, Ordering::SeqCst);
    if let Some(handle) = progress {
        let _ = handle.join();
    }

    let mut out = Vec::with_capacity(match_count);
    for _ in 0..match_count {
        out.push(
            rx.recv()
                .map_err(|_| "worker exited before target match count was reached".to_string())?,
        );
    }
    Ok(out)
}

fn set_chain(selected: &mut Option<Chain>, new: Chain) -> Result<(), String> {
    match selected {
        None => {
            *selected = Some(new);
            Ok(())
        }
        Some(existing) if *existing == new => Err("cannot repeat the same chain flag".into()),
        Some(_) => Err(
            "cannot specify multiple chains or conflicting flags (--ETH / --SOL / --BTC)".into(),
        ),
    }
}

fn infer_btc_kind_from_prefix(prefix: &str) -> Option<BtcAddressKind> {
    let p = prefix.trim().to_ascii_lowercase();
    if !p.starts_with("bc1") {
        return None;
    }
    if p.starts_with("bc1p") {
        Some(BtcAddressKind::P2tr)
    } else if p.starts_with("bc1q") {
        Some(BtcAddressKind::P2wpkh)
    } else {
        Some(BtcAddressKind::Both)
    }
}

fn require_btc_bc1_prefix(prefix_raw: Option<&str>) -> Result<(), String> {
    let Some(raw) = prefix_raw else {
        return Ok(());
    };
    let p = raw.trim();
    if p.is_empty() {
        return Ok(());
    }
    if !p.to_ascii_lowercase().starts_with("bc1") {
        return Err("BTC vanity --prefix must start with bc1".into());
    }
    Ok(())
}

pub fn parse_vanity_cli(args: &[String], word_count: usize) -> Result<VanityConfig, String> {
    let strict_case = args
        .iter()
        .any(|a| a == "--strict" || a == "--case-sensitive");

    let mut chain: Option<Chain> = None;
    let mut prefix_raw: Option<String> = None;
    let mut suffix_raw: Option<String> = None;
    let mut threads: Option<usize> = None;
    let mut match_count: Option<usize> = None;

    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "--strict" | "--case-sensitive" => {
                i += 1;
            }
            "--ETH" | "--eth" => {
                set_chain(&mut chain, Chain::Eth)?;
                i += 1;
            }
            "--SOL" | "--sol" => {
                set_chain(&mut chain, Chain::Sol)?;
                i += 1;
            }
            "--BTC" | "--btc" => {
                set_chain(&mut chain, Chain::Btc(BtcAddressKind::P2wpkh))?;
                i += 1;
            }
            "--prefix" | "-p" => {
                let v = args
                    .get(i + 1)
                    .ok_or_else(|| "--prefix requires a value".to_string())?;
                prefix_raw = Some(v.clone());
                i += 2;
            }
            "--suffix" | "-s" => {
                let v = args
                    .get(i + 1)
                    .ok_or_else(|| "--suffix requires a value".to_string())?;
                suffix_raw = Some(v.clone());
                i += 2;
            }
            "--threads" | "-j" => {
                let v = args
                    .get(i + 1)
                    .ok_or_else(|| "--threads requires a value".to_string())?;
                let n: usize = v
                    .parse()
                    .map_err(|_| format!("invalid thread count: {v}"))?;
                threads = Some(n);
                i += 2;
            }
            "--count" | "-n" => {
                let v = args
                    .get(i + 1)
                    .ok_or_else(|| "--count requires a value".to_string())?;
                let n: usize = v.parse().map_err(|_| format!("invalid match count: {v}"))?;
                if n < 1 {
                    return Err("--count / -n must be a positive integer >= 1".into());
                }
                match_count = Some(n);
                i += 2;
            }
            other => {
                return Err(format!("unknown argument: {other}"));
            }
        }
    }

    let threads = threads.unwrap_or_else(|| {
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4)
    });

    let mut chain = chain.unwrap_or_default();
    if let Some(raw) = &prefix_raw {
        if let Some(inferred) = infer_btc_kind_from_prefix(raw) {
            match chain {
                Chain::Eth | Chain::Sol | Chain::Btc(_) => chain = Chain::Btc(inferred),
            }
        }
    }

    if matches!(chain, Chain::Btc(_)) {
        require_btc_bc1_prefix(prefix_raw.as_deref())?;
    }

    let prefix = prefix_raw
        .map(|s| normalize_fragment(chain, &s, strict_case))
        .transpose()?
        .unwrap_or_default();
    let suffix = suffix_raw
        .map(|s| normalize_fragment(chain, &s, strict_case))
        .transpose()?
        .unwrap_or_default();

    Ok(VanityConfig {
        chain,
        word_count,
        strict_case,
        prefix,
        suffix,
        threads,
        match_count: match_count.unwrap_or(1),
    })
}

#[cfg(test)]
mod parse_tests {
    use super::*;

    fn cli_args(s: &str) -> Vec<String> {
        s.split_whitespace().map(String::from).collect()
    }

    #[test]
    fn btc_flag_infers_kind_from_bc1p_prefix() {
        let cfg = parse_vanity_cli(&cli_args("--BTC --prefix bc1p"), 12).unwrap();
        assert_eq!(cfg.chain, Chain::Btc(BtcAddressKind::P2tr));
    }

    #[test]
    fn btc_flag_infers_kind_from_bc1q_prefix() {
        let cfg = parse_vanity_cli(&cli_args("--BTC --prefix bc1q"), 12).unwrap();
        assert_eq!(cfg.chain, Chain::Btc(BtcAddressKind::P2wpkh));
    }

    #[test]
    fn prefix_bc1p_auto_selects_btc_taproot_without_btc_flag() {
        let cfg = parse_vanity_cli(&cli_args("--prefix bc1p"), 12).unwrap();
        assert_eq!(cfg.chain, Chain::Btc(BtcAddressKind::P2tr));
    }

    #[test]
    fn prefix_bc1_searches_both_kinds() {
        let cfg = parse_vanity_cli(&cli_args("--BTC --prefix bc1"), 12).unwrap();
        assert_eq!(cfg.chain, Chain::Btc(BtcAddressKind::Both));
    }

    #[test]
    fn btc_prefix_must_start_with_bc1() {
        assert!(parse_vanity_cli(&cli_args("--BTC --prefix dead"), 12).is_err());
    }
}
