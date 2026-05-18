# generate-mnemonic

Multi-threaded CPU tool: generate BIP39 English mnemonics and brute-force vanity mnemonics by **Ethereum / Solana / Bitcoin** address prefix or suffix.

## Supported chains

| Chain | CLI | Derivation path | Address format | Prefix/suffix charset | Cost per extra char (approx.) |
|-------|-----|-----------------|----------------|----------------------|------------------------------|
| ETH (**default**) | `--ETH` | `m/44'/60'/0'/0/0` | `0x` + hex (EIP-55 output) | Hex, max 40 | ×16 |
| SOL | `--SOL` | `m/44'/501'/0'/0'` | Base58 | Base58 (no `0`/`O`/`I`/`l`), max 44 | ×58 |
| BTC | `--BTC` + `bc1…` prefix | BIP84 / BIP86 | Bech32 `bc1q…` / `bc1p…` only | Bech32, max 62 | ×32 |

Stack: `alloy` (ETH), `slip10` + Ed25519 (SOL), `bitcoin` BIP32 (BTC). Mnemonic entropy uses **rand 0.10** + `coins-bip39::new_from_entropy`.

`--ETH` / `--SOL` / `--BTC` are **mutually exclusive**. Omitting a chain flag defaults to `--ETH`. Prefix `bc1…` auto-selects BTC when no chain flag is set. BTC only supports **native SegWit / Taproot** (`bc1…`); `--prefix` must start with `bc1`.

## Requirements

- Rust toolchain (stable recommended)

## Build

```bash
cd generate-mnemonic
cargo build --release
```

Binary: `target/release/generate-mnemonic` (or `cargo run --release -- …`).

## Quick start

```bash
# Random 12-word mnemonic
cargo run --release --

# ETH vanity: address starts with dead
cargo run --release -- vanity --ETH --prefix dead

# SOL vanity
cargo run --release -- vanity --SOL --prefix So1 --count 1

# BTC (bc1 only)
cargo run --release -- vanity --BTC --prefix bc1
cargo run --release -- vanity --BTC --prefix bc1q -j 8
```

## CLI

### Help

```bash
cargo run --release -- --help
# Usage is also printed on invalid args or vanity parse errors
```

### Word count `--words` / `-w`

Global flag; may appear anywhere in `argv` (stripped before subcommand parsing). **Default 12 words**; allowed: **12 / 15 / 18 / 21 / 24**.

```bash
cargo run --release -- --words 24
cargo run --release -- vanity --words 24 --SOL --prefix H
```

### Generate mnemonic only

| Case | Command |
|------|---------|
| Default 12 words | `generate-mnemonic` |
| Custom count | `generate-mnemonic --words 24` |

Prints one line of space-separated words; **does not** derive an address.

### Vanity subcommand

```text
generate-mnemonic [--words N] vanity [--ETH|--SOL|--BTC]
    [--strict|--case-sensitive]
    --prefix <P> [--suffix <S>]
    [--threads N] [--count N]
```

#### Shared options (all chains)

| Flag | Short | Description |
|------|-------|-------------|
| `--strict` | — | Case-sensitive match (ETH: EIP-55; SOL: Base58; BTC: Bech32) |
| `--case-sensitive` | — | Same as `--strict` |
| `--prefix` | `-p` | Address prefix (charset per chain table above) |
| `--suffix` | `-s` | Address suffix |
| `--threads` | `-j` | Worker threads; default `available_parallelism()`, min 1 |
| `--count` | `-n` | Stop after N matches; default **1** |

At least one of `--prefix` or `--suffix` is required. By default matching is **case-insensitive** (address and fragments lowercased).

#### Output format

```
address: <address>
mnemonic: <mnemonic>
```

With `--count` > 1, each match is prefixed with `--- #k ---`.

#### Ethereum `--ETH` (default)

- Prefix/suffix apply to the address **body** (hex); optional `0x` is stripped for matching.
- With `--strict`, fragments must match EIP-55 checksummed form.

```bash
cargo run --release -- vanity --prefix dead              # same as --ETH
cargo run --release -- vanity --ETH -p 0x00 -s ff -j 8
cargo run --release -- vanity --strict --prefix 9858EfFD23
```

#### Solana `--SOL`

```bash
cargo run --release -- vanity --SOL --prefix So1
cargo run --release -- vanity --SOL --prefix So1 --suffix abc
cargo run --release -- vanity --SOL --strict --prefix HuS
```

#### Bitcoin `--BTC`

Only **`bc1…`** addresses (BIP84 P2WPKH `bc1q…`, BIP86 Taproot `bc1p…`). No Legacy (`1…`) or nested SegWit (`3…`). **`--prefix` must start with `bc1`.**

| Prefix | Search mode | Path |
|--------|-------------|------|
| `bc1…` (not `bc1q` / `bc1p`) | both `bc1q` and `bc1p` per mnemonic | BIP84 + BIP86 |
| `bc1q…` | P2WPKH only | `m/84'/0'/0'/0/0` |
| `bc1p…` | Taproot only | `m/86'/0'/0'/0/0` |

P2WPKH: prefix `bc1q1…` is **impossible** (5th character cannot be `1`).

```bash
cargo run --release -- vanity --BTC --prefix bc1
cargo run --release -- vanity --BTC --prefix bc1q
cargo run --release -- vanity --prefix bc1p   # auto-selects BTC Taproot
```

BTC vanity is slow (PBKDF2 per attempt). Progress logs every 5s.

## Test vectors (`abandon … about`, empty BIP39 passphrase)

Use these to verify derivation matches common wallets (`cargo test` covers them):

| Chain | Path | First address |
|-------|------|---------------|
| ETH | `m/44'/60'/0'/0/0` | `0x9858EfFD232b4033E47D90003D41EC34EcAeda94` (EIP-55) |
| SOL | `m/44'/501'/0'/0'` | `HAgk14JpMQLgt6rVgv7cBQFJWFto5Dqxi472uT3DKpqk` |
| BTC bc1q | `m/84'/0'/0'/0/0` | `bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu` |
| BTC bc1p | `m/86'/0'/0'/0/0` | `bc1p5cyxnuxmeuwuvkwfem96lqzszd02n6xdcjrs20cac6yqjjwudpxqkedrcr` |

## Paths vs wallets

| Case | Notes |
|------|-------|
| ETH | Matches MetaMask / Alloy `MnemonicBuilder` default first account |
| SOL | Matches Phantom / Solflare common first path; **Solana CLI** default `m/44'/501'` yields a different address |
| BTC bc1q | BIP84 P2WPKH — Electrum, BlueWallet, etc. |
| BTC bc1p | BIP86 Taproot — modern wallets (e.g. Sparrow, some hardware) |

BIP39 passphrase is always the empty string.

## Dependencies

| Crate | Role |
|-------|------|
| `alloy` | ETH `MnemonicBuilder` |
| `slip10` + `ed25519-dalek` | SOL SLIP-0010 derivation |
| `bitcoin` | BTC BIP32 + P2WPKH encoding |
| `rand` 0.10 | Mnemonic entropy, vanity batch sizing |
| `coins-bip39` (via alloy) | BIP39 English wordlist |

## Using Alloy for ETH

```rust
use alloy::signers::local::{coins_bip39::English, MnemonicBuilder};

fn example() -> Result<(), alloy::signers::local::LocalSignerError> {
    let signer = MnemonicBuilder::<English>::default()
        .phrase("your twelve or twenty four words here …")
        .derivation_path("m/44'/60'/0'/0/0")?
        .build()?;
    println!("{}", signer.address());
    Ok(())
}
```

See [Alloy `MnemonicBuilder`](https://docs.rs/alloy-signer-local/latest/alloy_signer_local/struct.MnemonicBuilder.html).

## Security

- A mnemonic is equivalent to private keys; never commit it, paste it in chat, or log it.
- Vanity search is brute force; runtime grows sharply with prefix/suffix length. Use only where you understand the risk.

## Development

```bash
cargo test      # ETH / SOL / BTC derivation vectors
cargo clippy
```
