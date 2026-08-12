# easypdf-rust 测试与 CI 系统设计

- **日期**：2026-08-09
- **作者**：ZCode Agent（协同设计）
- **状态**：已实现（v0.1.0 发布）
- **依赖**：`fuzz/fuzz_targets/`、`.github/workflows/`、`Cargo.toml` workspace lints、各 crate tests/

## 1. 目标与范围

为 easypdf-rust 建立**全面的测试与 CI 系统**，覆盖单元测试、集成测试、fuzz 测试、性能基准、代码覆盖率、clippy/rustfmt 静态检查、GitHub Actions CI 矩阵。

**核心需求**：

1. 1522 个测试全绿（单元 + 集成）。
2. 91.61% 行覆盖率。
3. 6 个 cargo-fuzz targets。
4. 0 CVE（cargo audit）。
5. 0 clippy 警告。
6. 0 rustdoc 警告。
7. GitHub Actions CI 矩阵（Linux + macOS × stable + MSRV 1.88）。
8. cargo-deny 配置（License 白名单、bans、sources）。

**非目标**：

- 不实现 Windows CI 矩阵（未来版本）。
- 不实现 property-based testing（proptest，未来版本）。
- 不实现 golden test 框架。
- 不实现性能回归检测（仅手动 benchmark）。

## 2. 总体架构

```
┌──────────────────────────────────────────────────────────────┐
│                    测试与 CI 系统                              │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐  │
│  │  单元测试 (各 crate tests/)                            │  │
│  │  ├── easypdf-core: model / error / crypto / converter  │  │
│  │  ├── easypdf-reader: reader / strategy / streaming     │  │
│  │  ├── easypdf-writer: writer / backend / shape / image  │  │
│  │  ├── easypdf-markdown: profile / pipeline / table      │  │
│  │  ├── easypdf-ocr: glm / hunyuan / baidu / http        │  │
│  │  ├── easypdf-runtime: mcp / resident                   │  │
│  │  └── easypdf-derive: trybuild tests                    │  │
│  └────────────────────────────────────────────────────────┘  │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐  │
│  │  集成测试 (easypdf-test/)                              │  │
│  │  ├── feature_integration.rs   feature 组合测试         │  │
│  │  ├── prelude_test.rs          prelude 导出测试         │  │
│  │  ├── html_tests.rs            HTML→PDF 测试            │  │
│  │  ├── markdown_tests.rs        Markdown 转换测试        │  │
│  │  └── derive_extended.rs       derive 宏扩展测试        │  │
│  └────────────────────────────────────────────────────────┘  │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐  │
│  │  Fuzz 测试 (fuzz/fuzz_targets/)                        │  │
│  │  ├── pdf_parse.rs             PDF 解析 fuzz            │  │
│  │  ├── streaming_scan.rs        流式扫描 fuzz            │  │
│  │  ├── pdf_encrypt_decrypt.rs   加解密 fuzz              │  │
│  │  ├── pdf_sign_verify.rs       签名验证 fuzz            │  │
│  │  ├── markdown_convert.rs      Markdown 转换 fuzz       │  │
│  │  └── ssrf_url.rs              SSRF 防护 fuzz           │  │
│  └────────────────────────────────────────────────────────┘  │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐  │
│  │  性能基准 (crates/easypdf-reader/benches/)             │  │
│  │  └── reader_session.rs        会话复用 benchmark       │  │
│  └────────────────────────────────────────────────────────┘  │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐  │
│  │  GitHub Actions CI                                     │  │
│  │  ├── ci.yml                   主 CI 流水线             │  │
│  │  │   ├── Build + Test (2 × 2 矩阵)                    │  │
│  │  │   ├── Clippy + Rustfmt                             │  │
│  │  │   └── Doctest + Examples                           │  │
│  │  └── security.yml             安全审计                 │  │
│  │      ├── cargo-audit (rustsec)                         │  │
│  │      └── cargo-deny (license/bans/sources)             │  │
│  └────────────────────────────────────────────────────────┘  │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐  │
│  │  静态检查                                              │  │
│  │  ├── clippy (pedantic + nursery)                       │  │
│  │  ├── rustfmt (100% 合规)                               │  │
│  │  ├── rustdoc (0 warnings)                              │  │
│  │  └── cargo-deny (License 白名单)                       │  │
│  └────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────┘
```

## 3. 模块职责划分

### 3.1 单元测试分布

| Crate | 测试文件 | 覆盖范围 |
|---|---|---|
| `easypdf-core` | `model/` tests, `error.rs` tests, `crypto/` tests, `converter_registry.rs` tests | 模型、错误、加密签名、转换器 |
| `easypdf-reader` | `reader/` tests, `strategy.rs` tests, `streaming/` tests, `manipulate.rs` tests | 读取、策略、流式、操作 |
| `easypdf-writer` | `writer.rs` tests, `backend.rs` tests, `shape.rs` tests, `image.rs` tests | 写入、后端、图形、图片 |
| `easypdf-markdown` | `markdown_profile.rs` tests, `processor_pipeline.rs` tests, `table/tests.rs` | Profile、管道、表格 |
| `easypdf-ocr` | `glm/` tests, `hunyuan/` tests, `baidu/` tests, `http/client/tests.rs` | OCR 引擎、HTTP 客户端 |
| `easypdf-runtime` | `mcp/` tests, `resident/` tests | MCP 服务器、Resident Daemon |
| `easypdf-derive` | `tests/trybuild_tests.rs` | derive 宏编译测试 |

### 3.2 集成测试

| 测试文件 | 覆盖范围 |
|---|---|
| `feature_integration.rs` | feature 组合（markdown / ocr / mcp / resident） |
| `prelude_test.rs` | prelude 导出完整性 |
| `html_tests.rs` | HTML→PDF（feature-gated） |
| `markdown_tests.rs` | Markdown 转换端到端 |
| `derive_extended.rs` | derive 宏高级功能 |

### 3.3 Fuzz Targets

| Target | 输入 | 断言 | 发现问题 |
|---|---|---|---|
| `pdf_parse` | 随机字节 | 不 panic | byte_finder OOB panic（已修复） |
| `streaming_scan` | 随机 PDF | 不 panic | — |
| `pdf_encrypt_decrypt` | 随机 PDF + 密码 | roundtrip 正确 | — |
| `pdf_sign_verify` | 随机 PDF + 证书 | roundtrip 正确 | — |
| `markdown_convert` | 随机 Markdown | 不 panic | — |
| `ssrf_url` | 随机 URL | 不 panic、正确拒绝 | — |

**Fuzz 发现的问题**：
- `byte_finder` OOB panic：当 PDF 对象偏移表不完整时，访问越界。已修复为边界检查。

### 3.4 GitHub Actions CI

**ci.yml 矩阵**：

```yaml
strategy:
  matrix:
    os: [ubuntu-latest, macos-latest]
    rust: [stable, 1.88.0]  # MSRV
```

**CI 步骤**：
1. `cargo build --workspace`
2. `cargo test --workspace`
3. `cargo clippy --workspace -- -D warnings`
4. `cargo fmt --check`
5. `cargo test --doc`（doctest）
6. `cargo build --examples`

**环境变量**：
```yaml
env:
  RUSTFLAGS: -D warnings
```

**security.yml**：
- 每周一 + push to main 触发
- `cargo audit`（rustsec advisory database）
- `cargo deny check`（license / bans / sources）

### 3.5 cargo-deny 配置

```toml
# deny.toml
[licenses]
allow = ["MIT", "Apache-2.0", "BSD-2-Clause", "BSD-3-Clause", "ISC", "Unicode-3.0"]

[bans]
multiple-versions = "warn"
wildcards = "allow"

[sources]
unknown-registry = "deny"
unknown-git = "deny"
```

### 3.6 Workspace Lints

```toml
# Cargo.toml
[workspace.lints.clippy]
pedantic = { level = "warn", priority = -1 }
similar_names = { level = "allow", priority = 0 }  # PDF 对象名误报

[workspace.lints.rust]
unsafe_code = "forbid"  # 实际为 deny（各 crate lib.rs 声明）
```

**注意**：roadmap 标注 `#![forbid(unsafe_code)]`，但实际代码使用 `#![deny(unsafe_code)]`。`deny` 可被 `#[allow(unsafe_code)]` 局部覆盖，`forbid` 不可。

## 4. 关键数据流

### 4.1 CI 流水线

```
Push / PR
    │
    ▼
ci.yml 触发
    │
    ├── Job 1: Build + Test (2 × 2 矩阵)
    │   ├── ubuntu-latest + stable
    │   ├── ubuntu-latest + 1.88.0
    │   ├── macos-latest + stable
    │   └── macos-latest + 1.88.0
    │
    ├── Job 2: Clippy + Rustfmt
    │   ├── cargo clippy --workspace -- -D warnings
    │   └── cargo fmt --check
    │
    └── Job 3: Doctest + Examples
        ├── cargo test --doc
        └── cargo build --examples
```

### 4.2 安全审计流水线

```
每周一 + push to main
    │
    ▼
security.yml 触发
    │
    ├── cargo audit (rustsec)
    │   └── 检查已知 CVE
    │
    └── cargo deny check
        ├── license: 白名单校验
        ├── bans: 重复依赖警告
        └── sources: 未知来源拒绝
```

### 4.3 Fuzz 测试流程

```
cargo fuzz run pdf_parse
    │
    ▼
libfuzzer 生成随机输入
    │
    ▼
pdf_parse(input)
    │
    ├── 正常返回 → 继续
    ├── panic → 报告 bug
    └── OOM → 报告 bug
    │
    ▼
修复 → 回归测试
```

## 5. 技术决策与权衡

| # | 决策 | 理由 | 权衡 |
|---|---|---|---|
| 1 | 1522 测试（非 2000+） | 覆盖率 91.61% 已足够 | 边界场景可能遗漏 |
| 2 | 6 fuzz targets | 覆盖关键路径 | 未覆盖所有模块 |
| 3 | 2 × 2 CI 矩阵（无 Windows） | macOS 和 Linux 已覆盖主流 | Windows 未测试 |
| 4 | `deny` 而非 `forbid` unsafe | 灵活性（可局部 allow） | 安全性略低 |
| 5 | cargo-deny 而非 cargo-vet | 更全面的审计 | 配置复杂度 |
| 6 | similar_names = "allow" | 避免 PDF 对象名误报 | 可能遗漏真正的问题 |

## 6. 测试与验收

### 6.1 质量指标

| 指标 | 值 |
|---|---|
| 测试通过 | 1522 |
| 行覆盖率 | 91.61% |
| Cargo audit CVE | 0 |
| Clippy 警告 | 0 |
| Rustdoc 警告 | 0 |
| Fuzz targets | 6 |
| CI 矩阵 | 2 × 2 = 4 jobs |

### 6.2 已知局限

- 无 Windows CI 矩阵（未来版本）。
- 无 proptest 属性测试（未来版本）。
- 无 golden test 框架。
- 无性能回归检测。
- `deny` 而非 `forbid` unsafe code。
- `similar_names = "allow"` 可能遗漏真正问题。

## 7. 引用

- CI 配置：`.github/workflows/ci.yml`、`.github/workflows/security.yml`
- Cargo-deny 配置：`deny.toml`
- Fuzz targets：`fuzz/fuzz_targets/`
- 性能基准：`crates/easypdf-reader/benches/reader_session.rs`
- Roadmap：`docs/superpowers/version-plan.md`（质量指标汇总）
- 安全审计：`docs/security/AUDIT.md`
