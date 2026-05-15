//! 多线程暴力搜索：随机助记词 → Alloy 派生地址 → 匹配前缀/后缀。

use alloy::signers::local::coins_bip39::{English, Mnemonic};
use rand::Rng;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

use crate::chain::Chain;
use crate::eth::address_from_mnemonic_at_path;

fn normalize_hex_fragment(s: &str, strict_case: bool) -> Result<String, String> {
    let t = s.trim();
    let t = t.strip_prefix("0x").unwrap_or(t);
    if t.is_empty() {
        return Ok(String::new());
    }
    if !t.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(format!("非十六进制字符: {s:?}"));
    }
    if t.len() > 40 {
        return Err("前缀/后缀长度不能超过 40 个十六进制字符（地址主体）".into());
    }
    Ok(if strict_case {
        t.to_string()
    } else {
        t.to_lowercase()
    })
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
    /// `false`：前缀/后缀与地址主体均忽略大小写（默认）。
    /// `true`：与 EIP-55 checksummed 地址主体逐字匹配（大小写敏感）。
    pub strict_case: bool,
    pub prefix: String,
    pub suffix: String,
    pub threads: usize,
}

pub fn search_vanity_mnemonic(cfg: VanityConfig) -> Result<(Mnemonic<English>, String), String> {
    if cfg.prefix.is_empty() && cfg.suffix.is_empty() {
        return Err("至少需要指定 --prefix 或 --suffix 之一".into());
    }

    let threads = cfg.threads.max(1);
    let stop = Arc::new(AtomicBool::new(false));
    let (tx, rx) = mpsc::sync_channel(1);

    let strict_case = cfg.strict_case;
    let word_count = cfg.word_count;
    let chain = cfg.chain;
    let path = chain.derivation_path();
    let prefix = cfg.prefix;
    let suffix = cfg.suffix;

    thread::scope(|s| {
        for _ in 0..threads {
            let stop = Arc::clone(&stop);
            let tx = tx.clone();
            let prefix = prefix.clone();
            let suffix = suffix.clone();
            s.spawn(move || {
                let mut rng = rand::thread_rng();
                while !stop.load(Ordering::Relaxed) {
                    let batch = rng.gen_range(64usize..256usize);
                    for _ in 0..batch {
                        let m = match crate::words::random_mnemonic(word_count) {
                            Ok(m) => m,
                            Err(_) => continue,
                        };
                        let Ok(addr) = address_from_mnemonic_at_path(&m, path) else {
                            continue;
                        };
                        let body = chain.normalize_address_body(&addr, strict_case);
                        if body_matches(&body, &prefix, &suffix) {
                            if stop
                                .compare_exchange(false, true, Ordering::SeqCst, Ordering::Relaxed)
                                .is_ok()
                            {
                                let _ = tx.send((m, addr));
                            }
                            return;
                        }
                    }
                }
            });
        }
    });

    rx.recv()
        .map_err(|_| "搜索线程意外结束（未找到匹配）".to_string())
}

pub fn parse_vanity_cli(args: &[String], word_count: usize) -> Result<VanityConfig, String> {
    let strict_case = args
        .iter()
        .any(|a| a == "--strict" || a == "--case-sensitive");

    let mut chain: Option<Chain> = None;
    let mut prefix: Option<String> = None;
    let mut suffix: Option<String> = None;
    let mut threads: Option<usize> = None;

    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "--strict" | "--case-sensitive" => {
                i += 1;
            }
            "--ETH" | "--eth" => {
                if chain.replace(Chain::Eth).is_some() {
                    return Err("不能重复指定链（--ETH）".into());
                }
                i += 1;
            }
            "--prefix" | "-p" => {
                let v = args
                    .get(i + 1)
                    .ok_or_else(|| "--prefix 需要参数".to_string())?;
                prefix = Some(normalize_hex_fragment(v, strict_case)?);
                i += 2;
            }
            "--suffix" | "-s" => {
                let v = args
                    .get(i + 1)
                    .ok_or_else(|| "--suffix 需要参数".to_string())?;
                suffix = Some(normalize_hex_fragment(v, strict_case)?);
                i += 2;
            }
            "--threads" | "-j" => {
                let v = args
                    .get(i + 1)
                    .ok_or_else(|| "--threads 需要参数".to_string())?;
                let n: usize = v
                    .parse()
                    .map_err(|_| format!("无效线程数: {v}"))?;
                threads = Some(n);
                i += 2;
            }
            other => {
                return Err(format!("未知参数: {other}"));
            }
        }
    }

    let threads = threads.unwrap_or_else(|| {
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4)
    });

    Ok(VanityConfig {
        chain: chain.unwrap_or_default(),
        word_count,
        strict_case,
        prefix: prefix.unwrap_or_default(),
        suffix: suffix.unwrap_or_default(),
        threads,
    })
}
