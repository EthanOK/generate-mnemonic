use alloy::signers::local::coins_bip39::{English, Mnemonic};

mod chain;
mod eth;
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
  generate-mnemonic [--words N] vanity [--ETH] [--strict|--case-sensitive] --prefix <hex> [--suffix <hex>] [--threads N]
      暴力搜索：链默认 ETH；默认忽略大小写，加 --strict 则与 EIP-55 地址逐字匹配
      路径 {}
      例: generate-mnemonic --words 24
          generate-mnemonic vanity --ETH --words 24 --prefix dead
          generate-mnemonic vanity --strict --prefix 9858EfFD23

Alloy: alloy::signers::local::MnemonicBuilder 与 coins_bip39::English（见 Alloy 文档）。
",
        eth::ETH_DEFAULT_PATH,
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
            "开始搜索（{} 线程，{} 词），链 {}，路径 {}，大小写: {}（Alloy MnemonicBuilder）……",
            cfg.threads,
            cfg.word_count,
            cfg.chain.cli_label(),
            cfg.chain.derivation_path(),
            if cfg.strict_case { "严格" } else { "忽略" }
        );
        eprintln!("提示: 前缀/后缀每多 1 个十六进制字符，期望耗时约增 16 倍；助记词等同私钥，请勿泄露。");
        match vanity::search_vanity_mnemonic(cfg) {
            Ok((m, addr)) => {
                println!("address: {addr}");
                println!("mnemonic: {}", m.to_phrase());
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
