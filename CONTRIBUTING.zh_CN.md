# 贡献指南

感谢你对 easypdf-rust 的关注！本文档提供贡献指南和操作说明。

## 行为准则

本项目遵循行为准则。参与即表示你同意维护尊重和包容的环境。
参见 [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md)。

## 快速开始

### 前置条件

- **Rust 工具链**：MSRV 1.88，Edition 2024
- **Git**：版本控制

### Fork 并克隆

```bash
# 在 GitHub 上 Fork 仓库，然后：
git clone https://github.com/<your-username>/easypdf-rust.git
cd easypdf-rust
git remote add upstream https://github.com/easy-4-rust/easypdf-rust.git
```

### 构建和验证

```bash
# 构建工作区
cargo build

# 运行所有测试
cargo test --workspace

# 代码检查（要求零 warning）
cargo clippy --workspace --all-targets -- -D warnings

# 验证文档构建无 warning
cargo doc --workspace --no-deps

# 运行 fuzz targets（需要 nightly）
cargo +nightly fuzz run pdf_parse
cargo +nightly fuzz run streaming_scan
cargo +nightly fuzz run pdf_encrypt_decrypt
cargo +nightly fuzz run pdf_sign_verify
cargo +nightly fuzz run markdown_convert
cargo +nightly fuzz run ssrf_url
```

以上所有检查必须在提交 pull request 前通过。

## 开发工作流

### 分支策略

- **`dev`** 是主要开发分支。从 `dev` 创建功能分支。
- **`main`** 是稳定发布分支。不要直接向 `main` 提交。
- 功能分支命名：`feat/<简短描述>`、`fix/<简短描述>`、
  `docs/<简短描述>` 等。

### Commit 消息规范

遵循 [Conventional Commits](https://www.conventionalcommits.org/)：

```
feat: add streaming read strategy for large PDFs
fix: correct UTF-16BE metadata encoding in writer
docs: update architecture diagram for 9-crate layout
test: add fuzz target for PDF signature verification
refactor: consolidate model types into easypdf-core
chore: update lopdf dependency to 0.44.0
```

相关时加 scope 前缀：`feat(reader):`、`fix(writer):`、`docs(ocr):`。

### 代码风格

- **零 unsafe**：workspace 全局强制 `#![deny(unsafe_code)]`。
- **Clippy pedantic**：`Cargo.toml` 中设置 `clippy::all = "warn"`
  和 `clippy::pedantic = "warn"`。
- **格式化**：提交前运行 `cargo fmt --all`。
- **文件大小**：单个文件控制在 800 行以内。

### 测试要求

项目维护 1522+ 测试，覆盖率 91.61%。所有变更必须：

1. 新功能包含对应测试。
2. 不降低现有覆盖率。
3. `cargo test --workspace` 零失败通过。

## Pull Request 流程

1. **从 `dev` 创建功能分支**。
2. **按上述指南进行修改**。
3. **运行所有质量门禁**（见下方）。
4. **更新 CHANGELOG.md**（如有用户可见变更）。
5. **提交 pull request** 到 `dev` 分支。
6. **至少 1 位 reviewer** 审核后方可合并。

### 质量门禁（CI 必须全部通过）

```bash
# 格式检查
cargo fmt --all -- --check

# 代码检查
cargo clippy --workspace --all-targets -- -D warnings

# 构建（无默认 features）
cargo check -p easypdf --no-default-features

# 构建（全 features）
cargo check -p easypdf --all-features

# 测试
cargo test --workspace

# 文档
cargo doc --workspace --no-deps

# 安全审计
cargo audit
cargo deny check
```

## 项目结构

工作区包含 9 个 crate（从原始 22 个整合而来）：

| Crate | 路径 | 角色 |
|-------|------|------|
| `easypdf` | `crates/easypdf/` | 门面 crate -- `EasyPdf` 入口和 builder API |
| `easypdf-core` | `crates/easypdf-core/` | 核心类型、trait、错误、加密、模型、IO、布局 |
| `easypdf-derive` | `crates/easypdf-derive/` | `#[derive(PdfModel)]` proc-macro |
| `easypdf-reader` | `crates/easypdf-reader/` | PDF 读取、文本提取、合并/拆分/旋转（lopdf 后端） |
| `easypdf-writer` | `crates/easypdf-writer/` | PDF 创建与写入（printpdf 后端） |
| `easypdf-markdown` | `crates/easypdf-markdown/` | PDF->Markdown 转换管道（含表格检测、渲染、OCR） |
| `easypdf-ocr` | `crates/easypdf-ocr/` | 云端 OCR 引擎集合（GLM / HunyuanOCR / 百度） |
| `easypdf-runtime` | `crates/easypdf-runtime/` | 运行时层：MCP server + 常驻守护进程 |
| `easypdf-test` | `easypdf-test/` | 集成测试与 golden samples（不发布） |

详细架构参见
[docs/easypdf-rust-Architecture.zh_CN.md](docs/easypdf-rust-Architecture.zh_CN.md)
和 [docs/PROJECT_FACTS.md](docs/PROJECT_FACTS.md)。

## 测试

### 单元测试

单元测试位于各 crate 内的 `#[cfg(test)] mod tests` 块中。

```bash
# 运行特定 crate 的测试
cargo test -p easypdf-core
cargo test -p easypdf-reader
```

### 集成测试

跨 crate 集成测试位于 `easypdf-test/tests/`，golden PDF 样本在
`easypdf-test/golden/` 和 `easypdf-test/samples/`。

### Fuzz 测试

Fuzz targets 位于 `fuzz/`，需要 nightly 工具链：

```bash
cargo +nightly fuzz run <target>
```

可用 targets：`pdf_parse`、`streaming_scan`、`pdf_encrypt_decrypt`、
`pdf_sign_verify`、`markdown_convert`、`ssrf_url`。

## 发布流程

发布按以下顺序进行（另见
[docs/RELEASE_CHECKLIST.md](docs/RELEASE_CHECKLIST.md)）：

1. **修改版本号**：根 `Cargo.toml` 中的 `workspace.package.version`。
2. **更新 CHANGELOG.md**：添加新版本段落。
3. **dry-run 验证**：
   ```bash
   cargo publish -p easypdf-core --dry-run
   cargo publish -p easypdf-derive --dry-run
   ```
4. **按依赖顺序发布**（每步等待约 45 秒让 crates.io 传播）：
   ```bash
   # 第 1 层：无内部依赖
   cargo publish -p easypdf-core && sleep 45

   # 第 2 层：依赖 core
   cargo publish -p easypdf-derive && sleep 45
   cargo publish -p easypdf-reader && sleep 45
   cargo publish -p easypdf-writer && sleep 45

   # 第 3 层：依赖 core + reader
   cargo publish -p easypdf-markdown && sleep 45

   # 第 4 层：依赖 markdown
   cargo publish -p easypdf-ocr && sleep 45

   # 第 5 层：依赖 reader + writer + markdown
   cargo publish -p easypdf-runtime && sleep 45

   # 第 6 层：门面 crate（依赖以上全部）
   cargo publish -p easypdf
   ```
5. **打 tag 并发布**：
   ```bash
   git tag v<version>
   git push origin v<version>
   ```
6. **创建 GitHub Release**，附上 changelog 内容。
7. **验证** crates.io 列表和 docs.rs 构建。

## 有问题？

- 在 [GitHub Issues](https://github.com/easy-4-rust/easypdf-rust/issues)
  提交 bug、功能请求或问题。
- 查阅现有 [文档](docs/) 了解架构和使用指南。
