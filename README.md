# generate-mnemonic

生成 BIP39 英文助记词（**CPU** / `rand`）；支持按 **以太坊** 或 **Solana** 地址前缀/后缀暴力搜索助记词（vanity）。

| 链 | 派生路径 | 实现 |
|----|----------|------|
| ETH（默认） | `m/44'/60'/0'/0/0` | Alloy [`MnemonicBuilder`](https://docs.rs/alloy/latest/alloy/signers/local/struct.MnemonicBuilder.html)，与 MetaMask 首账户一致 |
| SOL | `m/44'/501'/0'/0'` | BIP39 种子 + SLIP-0010 Ed25519（`slip10`），与 Phantom / Solflare 首账户常用路径一致 |

## 环境要求

- Rust 工具链（建议 stable）

## 编译

```bash
cd generate-mnemonic
cargo build --release
```

二进制位于 `target/release/generate-mnemonic`（或 `cargo run --release --` 传参）。

## 命令行用法

安装后可将 `target/release` 加入 `PATH`，或始终通过 `cargo run --release --` 传参。

### 查看帮助

```bash
./target/release/generate-mnemonic --help
# 或
cargo run --release -- --help
```

### 词数（`--words` / `-w`）

在所有子命令前可写全局词数开关（可出现在参数列表任意位置，会先从 `argv` 中剥离再解析子命令）。**默认 12 词**；可选 BIP39 标准：**12、15、18、21、24**。

| 参数 | 简写 | 说明 |
|------|------|------|
| `--words N` | `-w N` | 助记词词数；省略时为 12。多次出现时以后者为准 |

示例：

```bash
cargo run --release -- --words 24
cargo run --release -- vanity --words 24 --prefix f
```

`vanity` 搜索时按该词数随机生成候选助记词（多线程 CPU）。

### 生成随机助记词（仅打印词组）

| 场景 | 命令 |
|------|------|
| 无参：CPU 随机助记词（默认 12 词） | `generate-mnemonic` |
| 指定词数 | `generate-mnemonic --words 24` |

无参时输出一行助记词（空格分词），无地址。

### Vanity：按链匹配地址前缀或后缀

子命令：`vanity`。通过 **`--ETH`** 或 **`--SOL`** 选择链；**省略链开关时默认为 ETH**。`--ETH` 与 `--SOL` 互斥，不可重复指定。

**大小写：**默认**忽略大小写**（前缀、后缀与地址主体均先转小写再比较）。

- **ETH + `--strict` / `--case-sensitive`**：与 **EIP-55** 校验和地址主体逐字匹配；输出亦为 checksummed 形式。
- **SOL + `--strict`**：与 **Base58** 地址逐字匹配（大小写敏感）。

#### 以太坊（`--ETH`，默认）

| 参数 | 简写 | 说明 |
|------|------|------|
| `--ETH` | 无 | 显式选择以太坊（与默认相同） |
| `--strict` | 无 | EIP-55 严格匹配（与 `--case-sensitive` 等价） |
| `--case-sensitive` | 无 | 同 `--strict` |
| `--prefix` | `-p` | 地址主体**十六进制**前缀（可带 `0x`） |
| `--suffix` | `-s` | 地址主体**十六进制**后缀 |
| `--threads` | `-j` | 工作线程数；省略时用 `available_parallelism()`，至少 1 |
| `--count` | `-n` | 匹配条数，**默认 1**，须 ≥ 1 |

`--ETH` 与 `--eth` 等价。前缀、后缀各最长 **40** 个十六进制字符。至少指定 `--prefix` 或 `--suffix` 之一。

```bash
cargo run --release -- vanity --ETH --prefix dead --count 1
cargo run --release -- vanity --prefix dead          # 省略链开关，仍为 ETH
cargo run --release -- vanity --ETH --suffix cafe
cargo run --release -- vanity --ETH -p 0x00 -s ff -j 8
cargo run --release -- vanity --strict --prefix 8EfF   # EIP-55 严格
```

#### Solana（`--SOL`）

| 参数 | 简写 | 说明 |
|------|------|------|
| `--SOL` | 无 | 选择 Solana（Base58 地址） |
| `--strict` | 无 | Base58 严格大小写 |
| `--prefix` | `-p` | 地址 **Base58** 前缀（字符集不含 `0` / `O` / `I` / `l`） |
| `--suffix` | `-s` | 地址 **Base58** 后缀 |
| `--threads` | `-j` | 同 ETH |
| `--count` | `-n` | 同 ETH |

`--SOL` 与 `--sol` 等价。前缀、后缀各最长 **44** 个 Base58 字符。

```bash
cargo run --release -- vanity --SOL --prefix So1 --count 1
cargo run --release -- vanity --SOL --prefix So1 --suffix abc -j 8
cargo run --release -- vanity --SOL --strict --prefix HuS
```

#### 输出

每条匹配输出 `address:` 与 `mnemonic:`；`--count` 大于 1 时带 `--- #k ---` 分段。助记词等同于私钥，请妥善保管。

## 技术说明

- **随机助记词**：BIP39 英文词表，`rand` 线程 RNG；词数由 `--words` 控制。
- **ETH 地址**：`MnemonicBuilder` + `coins_bip39::English`，与 `cast wallet address --mnemonic ... --mnemonic-derivation-path "m/44'/60'/0'/0/0"` 对齐（`abandon … about` 向量单测）。
- **SOL 地址**：`mnemonic.to_seed` → SLIP-0010 `m/44'/501'/0'/0'` → Ed25519 公钥 → Base58，与 `ed25519-hd-key` + `@solana/web3.js` `Keypair.fromSeed` 对齐（同向量单测）。
- **Vanity**：多线程 CPU 随机助记词并派生地址直至匹配。耗时近似：ETH 每多 1 个十六进制字符约 **×16**；SOL 每多 1 个 Base58 字符约 **×58**。

> **路径说明**：Solana CLI 默认 `m/44'/501'` 与浏览器钱包 `m/44'/501'/0'/0'` 不同，导入同一助记词会得到不同地址；本工具 SOL 模式与 Phantom / Solflare 首账户路径一致。

## 在代码中使用 Alloy（ETH 参考）

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

随机钱包、自定义词数、密码、`.index(n)` 等见 [Alloy `MnemonicBuilder` 文档](https://docs.rs/alloy-signer-local/latest/alloy_signer_local/struct.MnemonicBuilder.html)。

## 安全提示

- 助记词泄露即资产风险；勿提交到版本库、聊天或日志。
- Vanity 为暴力搜索，仅用于你理解风险且可控的环境。

## 开发与测试

```bash
cargo test
cargo clippy
```
