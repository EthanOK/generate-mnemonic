use alloy::signers::local::coins_bip39::{English, Mnemonic};

mod btc;
mod chain;
mod eth;
mod sol;
mod vanity;
mod words;

fn random_mnemonic(word_count: usize) -> Mnemonic<English> {
    words::random_mnemonic(word_count).expect("word_count validated by peel_word_flags")
}

fn print_usage() {
    eprintln!(
        "\
Usage:
  generate-mnemonic [--words N] [-w N]
      Random BIP39 mnemonic on CPU; N defaults to 12, or 15 / 18 / 21 / 24
  generate-mnemonic [--words N] vanity [--ETH|--SOL|--BTC] [--strict] --prefix <P> [--suffix <S>] [--threads N] [--count N]
      BTC: native SegWit/Taproot only (bc1…); prefix must start with bc1
           bc1q… → P2WPKH, bc1p… → Taproot, bc1… → both
      Examples:
          generate-mnemonic vanity --BTC --prefix bc1
          generate-mnemonic vanity --BTC --prefix bc1q
          generate-mnemonic vanity --ETH --prefix dead

ETH: Alloy; SOL: SLIP-0010; BTC: BIP32 via bitcoin crate.
",
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
            "Searching ({} threads, {} words, target {} match(es)), {}, path {}, case: {}…",
            cfg.threads,
            cfg.word_count,
            cfg.match_count,
            cfg.chain.cli_label(),
            cfg.chain.derivation_path(),
            if cfg.strict_case { "strict" } else { "ignore" }
        );
        let charset_hint = match cfg.chain {
            chain::Chain::Eth => "~16× per extra hex char (ETH)",
            chain::Chain::Sol => "~58× per extra Base58 char (SOL)",
            chain::Chain::Btc(_) => "~32× per extra Bech32 char (BTC); slow (PBKDF2 per try)",
        };
        eprintln!("Hint: {charset_hint}; mnemonics are secret keys — do not leak.");
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

    eprintln!("Unknown argument: {}", args[0]);
    print_usage();
    std::process::exit(1);
}
