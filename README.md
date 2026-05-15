# generate-mnemonic

生成 BIP39 英文助记词（**CPU** / `rand`）；支持按 **以太坊地址前缀/后缀** 暴力搜索助记词。地址派生通过 **Alloy** 的 [`MnemonicBuilder`](https://docs.rs/alloy/latest/alloy/signers/local/struct.MnemonicBuilder.html)，默认路径与 MetaMask 首账户一致：`m/44'/60'/0'/0/0`。

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

子命令：`vanity`。当前仅实现 **以太坊（`--ETH`）**，默认与省略链开关时均为 ETH，派生路径 **`m/44'/60'/0'/0/0`**，与 MetaMask 首账户、`MnemonicBuilder` 默认一致。后续可在同一位置增加 `--BTC` 等开关及对应派生与地址格式。

**大小写：**默认**忽略大小写**（前缀、后缀与地址主体均先转小写再比较）。加 **`--strict`** 或 **`--case-sensitive`** 后改为 **EIP-55 严格匹配**：地址以 Alloy `Display`（校验和）形式参与比较，你输入的前缀/后缀需与该校验和主体的大小写一致。

**参数：**

| 参数 | 简写 | 说明 |
|------|------|------|
| `--ETH` | 无 | 显式选择以太坊（与默认行为相同；便于脚本自描述，并为将来多链互斥预留位置） |
| `--strict` | 无 | 大小写严格匹配（与 `--case-sensitive` 等价） |
| `--case-sensitive` | 无 | 同 `--strict` |
| `--prefix` | `-p` | 地址主体十六进制前缀（可带 `0x`）。默认忽略大小写；`--strict` 下与 EIP-55 一致 |
| `--suffix` | `-s` | 地址主体十六进制后缀（规则同上） |
| `--threads` | `-j` | 工作线程数；省略时使用 `std::thread::available_parallelism()`，至少为 1 |

`--ETH` 与 `--eth` 等价。不能重复写两次 `--ETH`。

前缀、后缀各自最长 **40** 个十六进制字符（对应 20 字节地址主体）。至少指定其一。

**示例：**

```bash
# 显式指定 ETH（推荐写在脚本里，后续加其它链时意图清晰）
cargo run --release -- vanity --ETH --prefix dead

# 省略链开关时仍为 ETH
cargo run --release -- vanity --prefix dead

# 后缀 cafe
cargo run --release -- vanity --ETH --suffix cafe

# 前缀 + 后缀，8 线程
cargo run --release -- vanity --ETH -p 0x00 -s ff -j 8

# 严格大小写（需与 EIP-55 校验和前缀一致；地址输出亦为 checksummed）
cargo run --release -- vanity --strict --prefix 8EfF
```

**输出：** 两行——`address:` 与 `mnemonic:`。请妥善保管助记词，等同于私钥。

## 技术说明

- **随机助记词**：BIP39 英文词表，`rand` 线程 RNG；词数由 `--words` 控制。
- **地址计算**：`alloy::signers::local::MnemonicBuilder` + `coins_bip39::English`，与 `cast wallet address --mnemonic ... --mnemonic-derivation-path "m/44'/60'/0'/0/0"` 一类工具对齐（单元测试用标准 `abandon … about` 向量校验）。
- **Vanity**：多线程 CPU 反复随机助记词并派生地址，直到匹配；耗时与前后缀长度近似按每字符 **16 倍** 增长。默认大小写不敏感；`--strict` 下使用 EIP-55 checksummed 地址与字面前缀/后缀匹配。

## 在代码中使用 Alloy（参考）

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
