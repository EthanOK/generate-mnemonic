use alloy::signers::local::coins_bip39::{English, Mnemonic};

mod chain;
mod eth;
mod sol;
mod vanity;
mod words;

fn random_mnemonic(word_count: usize) -> Mnemonic<English> {
    words::random_mnemonic(word_count).expect("word_count 已由 peel 校验")
}

fn print_usage() {
    eprintln!(
        "\
用法:
  generate-mnemonic [--words N] [-w N]
      CPU 随机 BIP39 助记词；词数 N 默认 12，可选 15 / 18 / 21 / 24
      地址派生（本工具随机模式仅输出词组）：Alloy 默认路径 {}
  generate-mnemonic [--words N] vanity [--ETH|--SOL] [--strict|--case-sensitive] --prefix <P> [--suffix <S>] [--threads N] [--count N]
      暴力搜索：链默认 ETH（--ETH）；--SOL 为 Base58 地址，路径 {}
      ETH 前缀/后缀为十六进制；SOL 为 Base58（不含 0/O/I/l）。默认忽略大小写，--strict 逐字匹配
      路径 {}
      例: generate-mnemonic vanity --ETH --prefix dead
          generate-mnemonic vanity --SOL --prefix So1 --suffix abc
          generate-mnemonic vanity --strict --prefix 9858EfFD23

ETH 派生：Alloy MnemonicBuilder；SOL：BIP39 + SLIP-0010 Ed25519。
",
        eth::ETH_DEFAULT_PATH,
        sol::SOL_DEFAULT_PATH,
        eth::ETH_DEFAULT_PATH
    );
}

fn main() {
    let raw: Vec<String> = std::env::args().skip(1).collect();
    let (word_count, args) = match words::peel_word_flags(&raw) {
        Ok(x) => x,
        Err(e) => {
            eprintln!("{e}");
            print_usage();
            std::process::exit(1);
        }
    };

    if args.is_empty() {
        let m = random_mnemonic(word_count);
        println!("{}", m.to_phrase());
        return;
    }

    if args[0] == "-h" || args[0] == "--help" {
        print_usage();
        return;
    }

    if args[0] == "vanity" {
        let cfg = match vanity::parse_vanity_cli(&args[1..], word_count) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("{e}");
                print_usage();
                std::process::exit(1);
            }
        };
        eprintln!(
            "开始搜索（{} 线程，{} 词，目标 {} 条匹配），链 {}，路径 {}，大小写: {}……",
            cfg.threads,
            cfg.word_count,
            cfg.match_count,
            cfg.chain.cli_label(),
            cfg.chain.derivation_path(),
            if cfg.strict_case { "严格" } else { "忽略" }
        );
        let charset_hint = match cfg.chain {
            chain::Chain::Eth => "ETH 每多 1 个十六进制字符约 16 倍",
            chain::Chain::Sol => "SOL 每多 1 个 Base58 字符约 58 倍",
        };
        eprintln!(
            "提示: {charset_hint}；助记词等同私钥，请勿泄露。"
        );
        match vanity::search_vanity_mnemonic(cfg) {
            Ok(matches) => {
                for (i, (m, addr)) in matches.iter().enumerate() {
                    if matches.len() > 1 {
                        println!("--- #{} ---", i + 1);
                    }
                    println!("address: {addr}");
                    println!("mnemonic: {}", m.to_phrase());
                }
            }
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(1);
            }
        }
        return;
    }

    eprintln!("未知参数: {}", args[0]);
    print_usage();
    std::process::exit(1);
}
