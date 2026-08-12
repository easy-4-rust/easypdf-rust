# v0.1.0-alpha.1 核心抽象与基础能力 实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: rust-workspace, rust-module-layout

**Goal:** 从零构建 easypdf-rust 的核心骨架，实现 PDF 创建、读取、合并、拆分、旋转、重排、Markdown 转换、表单填充、资源限制、事件系统等基础能力，建立 `EasyPdf` 门面 API 和 `#[derive(PdfModel)]` 过程宏，完成首批 69 个测试。

**Architecture:** 单一 Cargo workspace，初期约 22 个 crate（后在 v0.2 合并为 9 个）。核心分层：`easypdf`（门面）、`easypdf-core`（模型/错误/IO）、`easypdf-reader`（lopdf 读取）、`easypdf-writer`（printpdf 写入）、`easypdf-derive`（过程宏）、`easypdf-markdown`（Markdown 转换）。

**Tech Stack:** Rust edition 2024, rust-version 1.88, printpdf 0.8, lopdf 0.34, proc-macro2/quote/syn（derive）, pulldown-cmark（Markdown）。

## Global Constraints

- `#![deny(unsafe_code)]` 在每个 crate 的 `lib.rs` 中声明。
- 所有公共 API 返回 `Result<T, PdfError>`，不 panic。
- PDF 写入使用原子文件输出（temp file + rename）。
- 页面索引采用零基（zero-based），内部转换为 PDF 1-based。
- 不引入外部 C/C++ 依赖，纯 Rust 实现。

---

### Task 1: Workspace 初始化与核心错误类型

> Files:
> - Create: `Cargo.toml`（workspace root）
> - Create: `crates/easypdf-core/Cargo.toml`
> - Create: `crates/easypdf-core/src/lib.rs`
> - Create: `crates/easypdf-core/src/error.rs`
> - Create: `crates/easypdf-core/src/io/ssrf_guard.rs`

**Steps:**
- [x] 创建 Cargo workspace，根 `Cargo.toml` 定义 workspace members
- [x] 创建 `easypdf-core` crate，定义 `PdfError` / `PdfErrorCode` 枚举（含 Io / Parse / InvalidPdf / UnsupportedFeature / Encryption / Signature / SecurityViolation 等变体）
- [x] 定义 `Result<T>` 类型别名
- [x] 创建 `easypdf-core::io` 模块，含 `ssrf_guard`（URL 校验防 SSRF）
- [x] 验证 `cargo check` 通过

---

### Task 2: 模型层 -- PdfDocumentModel / PdfPageModel / PdfBlock

> Files:
> - Create: `crates/easypdf-core/src/model/`（pdf_document_model.rs / pdf_page_model.rs / pdf_block.rs / pdf_block_type.rs）
> - Create: `crates/easypdf-core/src/content.rs`
> - Create: `crates/easypdf-core/src/metadata.rs`

**Steps:**
- [x] 定义 `PdfDocumentModel` -- 文档级元数据、页面列表
- [x] 定义 `PdfPageModel` -- 页面尺寸、内容块列表
- [x] 定义 `PdfBlock` 枚举 -- 初始 5 个变体：Heading / Paragraph / List / Table / Image
- [x] 定义 `PdfText` / `PdfFont` / `PdfColor` / `PdfImage` / `PdfTable` / `PdfTableCell` 内容类型
- [x] 定义 `SourceLocation`（源位置追踪）
- [x] 验证模型编译通过

---

### Task 3: 读取引擎 -- easypdf-reader

> Files:
> - Create: `crates/easypdf-reader/Cargo.toml`
> - Create: `crates/easypdf-reader/src/lib.rs`
> - Create: `crates/easypdf-reader/src/reader/`
> - Create: `crates/easypdf-reader/src/strategy.rs`

**Steps:**
- [x] 基于 lopdf 实现 `PdfReader::open(path)` -- 加载 PDF 文件
- [x] 实现文本提取 `extract_text()` -- 遍历页面内容流，解码文本操作符
- [x] 实现元数据提取 `extract_metadata()` -- 读取 /Info 字典
- [x] 实现 `ReadStrategy` 枚举 -- Full / Lazy / auto（按文件大小自动选择）
- [x] 实现单会话复用（session reuse）-- 约 129x 加速 vs 重复打开
- [x] 实现 `PdfReadListener` 事件回调
- [x] 验证读取测试通过

---

### Task 4: 写入引擎 -- easypdf-writer

> Files:
> - Create: `crates/easypdf-writer/Cargo.toml`
> - Create: `crates/easypdf-writer/src/lib.rs`
> - Create: `crates/easypdf-writer/src/writer.rs`

**Steps:**
- [x] 基于 printpdf 实现 `PdfWriter::new(path)` -- 创建 PDF 文档
- [x] 实现文本写入 `write_text()` -- 14 种内置字体支持
- [x] 实现元数据写入 `set_metadata()` -- Title / Author / Subject / Keywords
- [x] 实现页面管理 `add_page()` / `finish()` -- 多页累积
- [x] 实现 `PdfWriteHandler` trait -- 写入生命周期钩子
- [x] 实现原子文件输出（temp + rename）
- [x] 验证写入测试通过

---

### Task 5: 合并 / 拆分 / 旋转 / 重排

> Files:
> - Create: `crates/easypdf-reader/src/manipulate.rs`

**Steps:**
- [x] 实现 `merge_files(paths, output)` -- 合并多个 PDF，修正 /Pages 树
- [x] 实现 `split(path)` -- 拆分为单页 PDF
- [x] 实现 `rotate_page(page, degrees)` -- 每页独立旋转 0/90/180/270
- [x] 实现 `reorder_pages(order)` -- 按指定顺序重排页面
- [x] 验证操作测试通过

---

### Task 6: Markdown 转换 -- easypdf-markdown

> Files:
> - Create: `crates/easypdf-markdown/Cargo.toml`
> - Create: `crates/easypdf-markdown/src/lib.rs`
> - Create: `crates/easypdf-markdown/src/markdown_profile.rs`
> - Create: `crates/easypdf-markdown/src/pdf_markdown_processor.rs`

**Steps:**
- [x] 定义 `MarkdownProfile` 枚举 -- Gfm / Llm / Plain
- [x] 实现 Markdown 解析（pulldown-cmark）→ `PdfBlock` IR 转换
- [x] 实现 GFM 表格解析
- [x] 实现图片策略（Skip / Link / Inline）
- [x] 验证 Markdown 转换测试通过

---

### Task 7: 表单填充

> Files:
> - Create: `crates/easypdf/src/pdf_fill_builder.rs`

**Steps:**
- [x] 实现 AcroForm 字段检测（读取 /AcroForm 字典）
- [x] 实现 `fill_form(template, data)` -- 按字段名填充值
- [x] 实现 `PdfFormBuilder` -- Builder 模式表单填充
- [x] 验证表单填充测试通过

---

### Task 8: 过程宏 -- easypdf-derive

> Files:
> - Create: `crates/easypdf-derive/Cargo.toml`
> - Create: `crates/easypdf-derive/src/lib.rs`
> - Create: `crates/easypdf-derive/src/implementation.rs`

**Steps:**
- [x] 实现 `#[derive(PdfModel)]` -- struct 到 PDF 模型映射
- [x] 支持属性：`#[pdf(field)]` / `#[pdf(order)]` / `#[pdf(skip)]` / `#[pdf(default)]` / `#[pdf(required)]` / `#[pdf(format)]` / `#[pdf(nested)]` / `#[pdf(font)]` / `#[pdf(size)]`
- [x] 验证 derive 宏测试通过

---

### Task 9: 门面 API -- easypdf

> Files:
> - Create: `crates/easypdf/Cargo.toml`
> - Create: `crates/easypdf/src/lib.rs`
> - Create: `crates/easypdf/src/builders.rs`
> - Create: `crates/easypdf/src/prelude.rs`

**Steps:**
- [x] 实现 `EasyPdf::create(path)` -- 创建 PDF Builder
- [x] 实现 `EasyPdf::read(path)` -- 读取 PDF
- [x] 实现 `EasyPdf::manipulate(path)` -- 操作已有 PDF
- [x] 实现 `PdfCreateBuilder` -- 链式 API（add_text / add_image / do_write）
- [x] 实现 `PdfSplitBuilder` / `PdfManipulateBuilder`
- [x] 实现 prelude 模块（常用类型重导出）

---

### Task 10: 资源限制与安全

> Files:
> - Modify: `crates/easypdf-reader/src/reader/`

**Steps:**
- [x] 实现文件大小限制（解压后检查）
- [x] 实现页面数量限制
- [x] 实现文本长度限制
- [x] 实现解压炸弹防护（decompression bomb guard）
- [x] 验证资源限制测试通过

---

### Task 11: 文档与测试

> Files:
> - Create: `README.md` + `README_zh.md`
> - Create: `docs/easypdf-rust-Architecture.md`
> - Create: `docs/usage-guide.md`

**Steps:**
- [x] 编写 README（双语）
- [x] 编写架构文档
- [x] 编写使用指南
- [x] 达到 69 个测试通过
- [x] 提交 Phase 1 complete

---

## Acceptance / Verification

```bash
cargo test --workspace                    # 69 tests pass
cargo clippy --workspace -- -D warnings   # 0 warnings
cargo fmt --check                         # 100% compliant
```

## 关键发现（代码核对）

- `#![deny(unsafe_code)]` 而非 `#![forbid(unsafe_code)]`，所有 crate 均为 deny。
- Phase 1 时 workspace 约 22 个 crate，合并为 9 个发生在 v0.2 架构聚拢阶段。
- `PdfBlock` 初始 5 个变体（Heading / Paragraph / List / Table / Image），后扩展至 14 个。
- session reuse 实测 129x 加速（vs 重复打开同一文件）。
- 原子文件输出使用 tempfile + persist（rename）。

## 依赖关系

```
Task 1 (Workspace + Error)
    │
    ├── Task 2 (Model)
    │       │
    │       ├── Task 3 (Reader)
    │       │       │
    │       │       ├── Task 5 (Manipulate)
    │       │       └── Task 10 (Resource Limits)
    │       │
    │       ├── Task 4 (Writer)
    │       │
    │       ├── Task 6 (Markdown)
    │       │
    │       └── Task 7 (Form Fill)
    │
    ├── Task 8 (Derive)
    │
    └── Task 9 (Facade)
            │
            └── Task 11 (Docs + Tests)
```
