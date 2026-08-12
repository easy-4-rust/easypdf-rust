# 0.1.0 发布检查清单（试运行报告）

生成日期：2026-08-12（元数据修复后更新）

## 摘要

| 项目 | 状态 |
|------|------|
| 版本 | `0.1.0`（workspace 一致） |
| 元数据完整 | **是** -- 所有阻塞问题已修复 |
| 试运行叶子 crate | **通过** -- easypdf-core、easypdf-derive |
| 试运行依赖 crate | 预期发布前失败（见下方说明） |
| 可发布 | **是** -- 按依赖顺序发布（第 4 节） |

**关于依赖 crate 试运行的说明**：依赖 `easypdf-core` 的 crate 在试运行时会失败，报错"no matching package named `easypdf-core` found"，因为它尚未发布到 crates.io。这是预期的发布前状态。原始阻塞错误（"all dependencies must have a version requirement specified"）**已解决**。发布 `easypdf-core` 并等待 crates.io 传播（~45 秒）后，所有依赖 crate 将通过试运行。

---

## 1. 元数据检查（按 crate）

### 公共 workspace 继承字段

所有 8 个 crate 从 `[workspace.package]` 继承：

- `version` = `0.1.0`
- `edition` = `2024`
- `rust-version` = `1.88`
- `license` = `Apache-2.0`
- `repository` = `https://github.com/easy-4-rust/easypdf-rust`
- `keywords` = `["pdf", "ocr", "markdown", "parser", "writer"]`
- `categories` = `["parser-implementations", "encoding"]`

### easypdf-core

| 字段 | 值 | 状态 |
|------|-----|------|
| name | `easypdf-core` | OK |
| version | `0.1.0`（workspace） | OK |
| edition | `2024`（workspace） | OK |
| rust-version | `1.88`（workspace） | OK |
| description | "Core types, traits, enums, converters, and errors for easypdf-rust" | OK |
| license | `Apache-2.0`（workspace） | OK |
| repository | 从 workspace 继承 | OK |
| readme | `README.md`（workspace） | OK |
| keywords | `["pdf", "ocr", "markdown", "parser", "writer"]`（workspace） | OK |
| categories | `["parser-implementations", "encoding"]`（workspace） | OK |
| publish | 未设置（默认 true） | OK |

### easypdf-derive

| 字段 | 值 | 状态 |
|------|-----|------|
| name | `easypdf-derive` | OK |
| version | `0.1.0`（workspace） | OK |
| edition | `2024`（workspace） | OK |
| rust-version | `1.88`（workspace） | OK |
| description | "Proc-macro derive for PdfModel trait in easypdf-rust" | OK |
| license | `Apache-2.0`（workspace） | OK |
| repository | 从 workspace 继承 | OK |
| readme | `README.md`（workspace） | OK |
| keywords | `["pdf", "ocr", "markdown", "parser", "writer"]`（workspace） | OK |
| categories | `["parser-implementations", "encoding"]`（workspace） | OK |
| publish | 未设置（默认 true） | OK |

### easypdf-reader

| 字段 | 值 | 状态 |
|------|-----|------|
| name | `easypdf-reader` | OK |
| version | `0.1.0`（workspace） | OK |
| edition | `2024`（workspace） | OK |
| rust-version | `1.88`（workspace） | OK |
| description | "PDF reading and text extraction for easypdf-rust (lopdf backend)" | OK |
| license | `Apache-2.0`（workspace） | OK |
| repository | 从 workspace 继承 | OK |
| readme | `README.md`（workspace） | OK |
| keywords | `["pdf", "ocr", "markdown", "parser", "writer"]`（workspace） | OK |
| categories | `["parser-implementations", "encoding"]`（workspace） | OK |
| publish | 未设置（默认 true） | OK |
| 内部依赖 | `easypdf-core` 通过 workspace（path + version） | OK |

### easypdf-writer

| 字段 | 值 | 状态 |
|------|-----|------|
| name | `easypdf-writer` | OK |
| version | `0.1.0`（workspace） | OK |
| edition | `2024`（workspace） | OK |
| rust-version | `1.88`（workspace） | OK |
| description | "PDF creation and writing for easypdf-rust (printpdf backend)" | OK |
| license | `Apache-2.0`（workspace） | OK |
| repository | 从 workspace 继承 | OK |
| readme | `README.md`（workspace） | OK |
| keywords | `["pdf", "ocr", "markdown", "parser", "writer"]`（workspace） | OK |
| categories | `["parser-implementations", "encoding"]`（workspace） | OK |
| publish | 未设置（默认 true） | OK |
| 内部依赖 | `easypdf-core` 通过 workspace（path + version） | OK |

### easypdf-markdown

| 字段 | 值 | 状态 |
|------|-----|------|
| name | `easypdf-markdown` | OK |
| version | `0.1.0`（workspace） | OK |
| edition | `2024`（workspace） | OK |
| rust-version | `1.88`（workspace） | OK |
| description | "Deterministic PDF to Markdown conversion for easypdf-rust (includes render, table detection, and OCR)" | OK |
| license | `Apache-2.0`（workspace） | OK |
| repository | 从 workspace 继承 | OK |
| readme | `README.md`（workspace） | OK |
| keywords | `["pdf", "ocr", "markdown", "parser", "writer"]`（workspace） | OK |
| categories | `["parser-implementations", "encoding"]`（workspace） | OK |
| publish | 未设置（默认 true） | OK |
| 内部依赖 | `easypdf-core`、`easypdf-reader` 通过 workspace（path + version） | OK |

### easypdf-ocr

| 字段 | 值 | 状态 |
|------|-----|------|
| name | `easypdf-ocr` | OK |
| version | `0.1.0`（workspace） | OK |
| edition | `2024`（workspace） | OK |
| rust-version | `1.88`（workspace） | OK |
| description | "Cloud OCR engine collection (GLM / HunyuanOCR / Baidu) for easypdf" | OK |
| license | `Apache-2.0`（workspace） | OK |
| repository | 从 workspace 继承 | OK |
| readme | `README.md`（workspace） -- **crate 级 README 已创建** | OK |
| keywords | `["pdf", "ocr", "markdown", "parser", "writer"]`（workspace） | OK |
| categories | `["parser-implementations", "encoding"]`（workspace） | OK |
| publish | 未设置（默认 true） | OK |
| 内部依赖 | `easypdf-core`、`easypdf-markdown` 通过 workspace（path + version） | OK |

### easypdf-runtime

| 字段 | 值 | 状态 |
|------|-----|------|
| name | `easypdf-runtime` | OK |
| version | `0.1.0`（workspace） | OK |
| edition | `2024`（workspace） | OK |
| rust-version | `1.88`（workspace） | OK |
| description | "Runtime layer for easypdf: MCP server + resident daemon" | OK |
| license | `Apache-2.0`（workspace） | OK |
| repository | 从 workspace 继承 | OK |
| readme | `README.md`（workspace） -- **crate 级 README 已创建** | OK |
| keywords | `["pdf", "ocr", "markdown", "parser", "writer"]`（workspace） | OK |
| categories | `["parser-implementations", "encoding"]`（workspace） | OK |
| publish | 未设置（默认 true） | OK |
| 内部依赖 | `easypdf-core`、`easypdf-reader`、`easypdf-writer`、`easypdf-markdown` 通过 workspace（path + version） | OK |

### easypdf（外观）

| 字段 | 值 | 状态 |
|------|-----|------|
| name | `easypdf` | OK |
| version | `0.1.0`（workspace） | OK |
| edition | `2024`（workspace） | OK |
| rust-version | `1.88`（workspace） | OK |
| description | "Easy PDF manipulation library for Rust -- builder API for creating, reading, manipulating, and filling PDFs" | OK |
| license | `Apache-2.0`（workspace） | OK |
| repository | 从 workspace 继承 | OK |
| readme | `README.md`（workspace） | OK |
| keywords | `["pdf", "ocr", "markdown", "parser", "writer"]`（workspace） | OK |
| categories | `["parser-implementations", "encoding"]`（workspace） | OK |
| publish | 未设置（默认 true） | OK |
| 内部依赖 | `easypdf-core`、`easypdf-derive`、`easypdf-reader`、`easypdf-writer` 通过 workspace（必选）；`easypdf-markdown`、`easypdf-ocr`、`easypdf-runtime` 通过 workspace（可选） | OK |

### easypdf-test（跳过 -- 不发布）

| 字段 | 值 | 状态 |
|------|-----|------|
| publish | `false` | OK（已跳过） |

---

## 2. 试运行结果

### easypdf-core -- 通过

```
Packaged 45 files, 341.1KiB (79.5KiB compressed)
Uploading easypdf-core v0.1.0
warning: aborting upload due to dry run
```

### easypdf-derive -- 通过

```
Packaged 13 files, 77.7KiB (18.5KiB compressed)
Uploading easypdf-derive v0.1.0
warning: aborting upload due to dry run
```

### easypdf-reader -- 预期发布前失败

```
error: no matching package named `easypdf-core` found
```

**原因**：`easypdf-core` 尚未在 crates.io 上发布。发布并传播后将通过。

### easypdf-writer -- 预期发布前失败

同 easypdf-reader。

### easypdf-markdown -- 预期发布前失败

同 easypdf-reader。

### easypdf-ocr -- 预期发布前失败

同 easypdf-reader。

### easypdf-runtime -- 预期发布前失败

同 easypdf-reader。

### easypdf -- 预期发布前失败

同 easypdf-reader。

---

## 3. 发现的问题

### 阻塞（发布前必须修复）

#### B1：路径依赖缺少 `version` 字段 -- 已修复

所有内部路径依赖现在使用 `workspace = true`，从 workspace 根的 `[workspace.dependencies]` 解析为 `{ path = "...", version = "0.1.0" }`。原始的 manifest 验证错误（"all dependencies must have a version requirement specified"）已解决。

**修复方案**：在根 `Cargo.toml` 的 `[workspace.dependencies]` 中添加了所有 8 个内部 crate 的 `path` + `version`。将所有 6 个子 crate 改为使用 `xxx.workspace = true`。

#### B2：`easypdf-ocr` 缺少 `readme` -- 已修复

添加了 `readme.workspace = true` 并创建了 `crates/easypdf-ocr/README.md`。

#### B3：`easypdf-runtime` 缺少 `readme` -- 已修复

添加了 `readme.workspace = true` 并创建了 `crates/easypdf-runtime/README.md`。

### 建议（非阻塞）

#### R1：任何 crate 都没有 `keywords` -- 已修复

在 `[workspace.package]` 中添加了 `keywords = ["pdf", "ocr", "markdown", "parser", "writer"]`。所有 8 个 crate 通过 `keywords.workspace = true` 继承。

#### R2：任何 crate 都没有 `categories` -- 已修复

在 `[workspace.package]` 中添加了 `categories = ["parser-implementations", "encoding"]`。所有 8 个 crate 通过 `categories.workspace = true` 继承。

#### R3：`easypdf-derive` 的 `easypdf-core` dev-dependency 仅有路径

```toml
[dev-dependencies]
easypdf-core = { path = "../easypdf-core" }
```

**有意保留为仅路径**。Dev-dependency 被排除在发布 crate 之外，因此不阻塞 `cargo publish`。对 dev-deps 使用 `workspace = true` 会导致 Cargo 在打包时尝试 crates.io 解析，这对未发布的内部 crate 会失败。

#### R4：`easypdf-runtime` 有 `[[bin]]` 带 `required-features`

该 crate 定义了二进制 `easypdf-mcp`，门控于 `features = ["mcp"]`。这对 crates.io 没问题，但请注意二进制只能通过 `cargo install easypdf-runtime --features mcp` 安装。

---

## 4. 发布命令序列（正式发布）

在所有阻塞问题修复后运行。每个 `cargo publish` 必须在下一个之前完成（crates.io 索引需要 ~30-45 秒传播）。

```bash
# 设置 crates.io token（如果尚未配置）
# cargo login <your-token>

cd /Users/wandl/workspaces/workspace-github-easy-4-rust/easypdf-rust

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

# 第 6 层：外观（依赖所有）
cargo publish -p easypdf
```

### 发布顺序说明

- `easypdf-core` 必须最先发布（所有其他 crate 依赖它）。
- `easypdf-derive`、`easypdf-reader`、`easypdf-writer` 可以在 core 之后以任意顺序发布。
- `easypdf-markdown` 需要 core + reader 在 crates.io 上可用。
- `easypdf-ocr` 需要 core + markdown。
- `easypdf-runtime` 需要 core + reader + writer + markdown。
- `easypdf`（外观）需要以上所有。
- `easypdf-test` 设为 `publish = false` -- 跳过。

---

## 5. 发布后

- [ ] 创建 git tag `v0.1.0`
- [ ] 推送 tag：`git push origin v0.1.0`
- [ ] 创建 GitHub Release，附上变更说明
- [ ] 验证每个 crate 出现在 crates.io 上：
  - https://crates.io/crates/easypdf-core
  - https://crates.io/crates/easypdf-derive
  - https://crates.io/crates/easypdf-reader
  - https://crates.io/crates/easypdf-writer
  - https://crates.io/crates/easypdf-markdown
  - https://crates.io/crates/easypdf-ocr
  - https://crates.io/crates/easypdf-runtime
  - https://crates.io/crates/easypdf
- [ ] 验证 docs.rs 构建：https://docs.rs/easypdf
- [ ] 公告（博客、社交媒体、Discord、r/rust）

---

## 6. 已应用的变更

### 根 `Cargo.toml`

1. 添加到 `[workspace.package]`：`keywords`、`categories`
2. 在 `[workspace.dependencies]` 中添加了 8 个内部 crate 的 `path` + `version`
3. 对 `easypdf-runtime` workspace dep 设置 `default-features = false`（外观控制 features）

### 子 crate `Cargo.toml` 文件（6 个文件）

| 文件 | 变更 |
|------|------|
| `crates/easypdf-reader/Cargo.toml` | `easypdf-core` 改为 `workspace = true`；添加 `keywords.workspace = true`、`categories.workspace = true` |
| `crates/easypdf-writer/Cargo.toml` | `easypdf-core` 改为 `workspace = true`；添加 `keywords.workspace = true`、`categories.workspace = true` |
| `crates/easypdf-markdown/Cargo.toml` | `easypdf-core`、`easypdf-reader` 改为 `workspace = true`；添加 `keywords.workspace = true`、`categories.workspace = true` |
| `crates/easypdf-ocr/Cargo.toml` | `easypdf-core`、`easypdf-markdown` 改为 `workspace = true`；添加 `readme.workspace = true`、`keywords.workspace = true`、`categories.workspace = true` |
| `crates/easypdf-runtime/Cargo.toml` | `easypdf-core`、`easypdf-reader`、`easypdf-writer`、`easypdf-markdown` 改为 `workspace = true`；添加 `readme.workspace = true`、`keywords.workspace = true`、`categories.workspace = true` |
| `crates/easypdf/Cargo.toml` | 所有 7 个内部依赖改为 `workspace = true`（3 个可选）；添加 `keywords.workspace = true`、`categories.workspace = true` |

### 同时更新（非阻塞）

| 文件 | 变更 |
|------|------|
| `crates/easypdf-core/Cargo.toml` | 添加 `keywords.workspace = true`、`categories.workspace = true` |
| `crates/easypdf-derive/Cargo.toml` | 添加 `keywords.workspace = true`、`categories.workspace = true` |

### 新文件

| 文件 | 用途 |
|------|------|
| `crates/easypdf-ocr/README.md` | crate 级 README，用于 crates.io |
| `crates/easypdf-runtime/README.md` | crate 级 README，用于 crates.io |

### 验证结果

| 检查 | 结果 |
|------|------|
| `cargo check --workspace` | 通过 |
| `cargo test --workspace` | 通过（全部测试通过，0 失败） |
| `cargo clippy --workspace --all-targets -D warnings` | 通过（0 警告） |
| `cargo publish --dry-run`（叶子 crate） | 通过（easypdf-core、easypdf-derive） |
| `cargo publish --dry-run`（依赖 crate） | 预期发布前失败（依赖未在 crates.io 上） |
