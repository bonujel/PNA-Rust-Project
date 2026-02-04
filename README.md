# PingCAP Talent Plan - Project 1: 构建内存键值存储

这是 [PingCAP Talent Plan](https://github.com/pingcap/talent-plan) 的第一个项目，实现了一个简单的内存键值存储系统。

## 项目概述

本项目实现了一个基本的键值存储库 `KvStore` 和一个命令行工具 `kvs`，支持以下操作：
- `set <key> <value>` - 设置键值对
- `get <key>` - 获取指定键的值
- `rm <key>` - 删除指定键

## 项目结构

```
project-1/
├── Cargo.toml          # 项目配置文件
├── src/
│   ├── lib.rs          # KvStore 库实现
│   └── bin/
│       └── kvs.rs      # CLI 工具实现
└── tests/
    └── tests.rs        # 集成测试
```

## 技术栈

- **Rust 2021 Edition**
- **clap v4.5** - 命令行参数解析（使用 derive 特性）
- **HashMap** - 内存存储实现

## 构建项目

```bash
# 构建项目
cargo build

# 构建 release 版本
cargo build --release
```

## 运行测试

```bash
# 运行所有测试
cargo test

# 运行集成测试
cargo test --test tests

# 运行文档测试
cargo test --doc
```

## 使用方法

```bash
# 查看版本
cargo run -- -V

# 设置键值对（当前返回 "unimplemented"）
cargo run -- set key1 value1

# 获取值（当前返回 "unimplemented"）
cargo run -- get key1

# 删除键（当前返回 "unimplemented"）
cargo run -- rm key1
```

## 实现状态

### ✅ 已完成
- [x] KvStore 库实现（使用 HashMap）
- [x] CLI 参数解析（使用 clap v4 derive）
- [x] 完整的 API 文档
- [x] 所有测试通过（13 个集成测试 + 5 个文档测试）
- [x] Clippy 检查通过
- [x] Rustfmt 格式检查通过

### 📝 当前行为
根据 Talent Plan 要求，所有 CLI 命令当前都返回 "unimplemented" 并以非零状态码退出。这是预期行为，后续项目将实现完整功能。

## 代码质量

项目遵循 Rust 最佳实践：
- 使用 `#![deny(missing_docs)]` 强制文档完整性
- 所有公共 API 都有详细文档和示例
- 通过 clippy 严格检查（`-D warnings`）
- 代码格式符合 rustfmt 标准

## 学习要点

### 1. Clap v4 Derive API
使用 derive 宏简化 CLI 参数解析：
```rust
#[derive(Parser)]
#[command(version = env!("CARGO_PKG_VERSION"))]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}
```

### 2. 文档测试
Rust 支持在文档注释中编写可执行测试：
```rust
/// # Examples
/// ```
/// use kvs::KvStore;
/// let mut store = KvStore::new();
/// store.set("key".to_owned(), "value".to_owned());
/// ```
```

### 3. 集成测试
使用 `assert_cmd` 和 `predicates` 进行 CLI 测试：
```rust
Command::cargo_bin("kvs")
    .unwrap()
    .args(&["get", "key1"])
    .assert()
    .failure()
    .stderr(contains("unimplemented"));
```

## 参考资料

- [PingCAP Talent Plan](https://github.com/pingcap/talent-plan)
- [Clap Documentation](https://docs.rs/clap/)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)

## 许可证

MIT
