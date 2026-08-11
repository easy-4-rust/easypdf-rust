# Changelog

All notable changes to this project will be documented in this file.
Format based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.2.0] — 2026-08-12

### Architecture: 22-crate consolidation to 9 crates

Workspace 从 ~22 个细粒度 crate 聚拢为 9 个聚焦 crate，降低编译时间和依赖图复杂度。

| 旧 crate | 新位置 | 理由 |
|---|---|---|
| `easypdf-model` | `easypdf-core::model` | 语义 IR 无引擎依赖，归属核心类型 |
| `easypdf-io` | `easypdf-core::io` | guards/limits/atomic output 共享 core 错误类型 |
| `easypdf-layout` | `easypdf-core::layout` | `LayoutSink` trait 是引擎无关基础设施 |
| `easypdf-manipulate` | `easypdf-reader::manipulate` | merge/split/rotate 均先读 lopdf 对象 |
| `easypdf-template` | `easypdf-writer::template` | AcroForm fill 复用 writer 的 lopdf document handle |
| `easypdf-markdown-table` | `easypdf-markdown::table` | 表格检测是 markdown pipeline 的一部分 |
| `easypdf-render` | `easypdf-markdown::render` | 页面渲染供 markdown OCR fallback 路径使用 |
| `easypdf-resident` | `easypdf-runtime::resident` | 守护进程与 MCP server 共享 runtime |
| `easypdf-mcp` | `easypdf-runtime::mcp` | MCP server 与守护进程共享 runtime |

### Added

- **Streaming ReadStrategy**：`ReadStrategy::Streaming` 字节流扫描，不构建 Document 对象，适用于超大文件或内存受限环境。`ReadStrategy::auto` 按文件大小自动选择最优策略（Full / Lazy / Streaming）。
- **CMap / ToUnicode 支持**：`easypdf-reader` 正确处理 CMap 编码字体，修复 CJK 文本提取乱码问题。
- **WriteBackend 选择**：`easypdf-writer` 支持 `InMemory`（默认）、`Spill`（页面级临时文件，恒定内存）、`Auto`（阈值自动切换）三种后端，通过 `PdfWriterBuilder` 配置。
- **PdfWriterBuilder + WriteHandlerChain**：可组合写处理器 pipeline，按优先级稳定排序执行。
- **ConverterRegistry**：`easypdf-core::converter_registry` 类型擦除双向转换器注册表。
- **ProcessorPipeline 调度器**：capability 协商 + priority 稳定排序，统一 markdown 处理器链。
- **easypdf-ocr**（新 crate）：4 大云 OCR 引擎——
  - GLM-OCR（智谱 BigModel，feature-gated `ocr-glm`）
  - HunyuanOCR（腾讯云，TC3-HMAC-SHA256 签名，feature-gated `ocr-hunyuan`）
  - 百度 Qianfan / PP-OCRv6（14 个 API 端点 + OAuth Token 管理，feature-gated `ocr-baidu`）
  - DeepSeek-OCR-2（OpenAI 兼容协议，feature-gated `ocr-deepseek`）
  - 统一 `HttpOcrEngine` trait，reqwest blocking HTTP，base64 图片编码，结构化 `OcrHttpError`。
- **easypdf-runtime — 常驻守护进程**：
  - Unix socket + Windows TCP fallback（`Transport` trait 抽象）
  - 自适应 autosave（EMA 平滑）
  - 空闲超时看门狗
- **easypdf-runtime — MCP server**：
  - 7 个 tools：`pdf_read_text` / `pdf_to_markdown` / `pdf_create_text` / `pdf_merge` / `pdf_split` / `pdf_metadata` / `pdf_page_count`
  - stdio JSON-RPC 协议，供 LLM agent 集成使用
- **PdfBlock IR 扩展**：从 5 变体扩展到 14 变体（新增 Code / Formula / PageBreak / Footnote / TableCell / BlockQuote / HorizontalRule / Link / Unknown）。
- **easypdf-derive 扩展属性**：支持 `field` / `order` / `skip` / `default` / `required` / `format` / `nested` 属性。
- **tracing 可观测性集成**：workspace 级 `tracing` + `tracing-subscriber`（`env-filter` + JSON 输出），结构化 span 覆盖 reader session / writer operation / markdown pipeline / IPC 审计。
- **Transport trait 抽象**：`easypdf-runtime::transport` 提供 Unix socket（默认）和 Windows TCP fallback 的统一接口。
- **PDF spec 加密对齐**：ISO 32000-1 section 7.6 标准安全处理器，包装 lopdf 加密能力。
- **PDF spec 签名对齐**：ISO 32000-1 section 12.8 PKCS#7/CMS detached SignedData + X.509 证书解析。
- **crypto audit 配置**：`.cargo/audit.toml` + `workspace.metadata` 声明加密依赖审计策略。
- **cargo-fuzz**：6 个 fuzz targets——`pdf_parse` / `streaming_scan` / `pdf_encrypt_decrypt` / `pdf_sign_verify` / `markdown_convert` / `ssrf_url`。
- **Markdown OCR 集成**：`easypdf-markdown::ocr` 提供 `OcrProcessor`（trait-based `OcrEngine` 抽象 + `MockOcrEngine` 测试桩）。
- **Markdown 表格检测**：`easypdf-markdown::table` 启发式表格检测（pipe / tab / whitespace 模式），集成到处理器 pipeline。
- **Markdown 页面渲染**：`easypdf-markdown::render` PDF 页面光栅化，供 OCR fallback 路径使用。
- **easypdf-test**（新 crate）：专用集成测试 harness，含 golden files、样例 PDF、跨 crate 场景测试。

### Security

- **`RUSTSEC-2023-0071` 修复**：rsa 0.9.10 Marvin Attack 从生产路径完全消除，迁移到 ring 0.17.14 constant-time RSA。`rsa` crate 仅保留为 dev-dependency（用于测试证书生成，ring 无 keygen API）。通过 `.cargo/audit.toml` ignore 该 advisory（生产路径已用 ring）。
- **`RUSTSEC-2025-0055` 修复**：tracing-subscriber 升级到 >=0.3.20，修复 ANSI escape 序列注入漏洞。
- **解压炸弹 guard**：移除 64KB 豁免漏洞，改为按绝对解压大小检查（不论输入大小）。
- **SSRF guard 增强**：新增 IPv6 全覆盖——loopback / ULA / link-local / IPv4-mapped 地址均纳入拦截范围。
- **API key Debug redact**：`GlmConfig` / `BaiduConfig` 等含密钥结构体不再在 `Debug` 输出中泄露 `api_key` / `secret_key`。
- **Guards 模块**：`easypdf-core::io::guards` 提供文件路径、页面范围、资源边界的输入校验。
- **Repair 工具**：`easypdf-core::io::repair` 有界递归 + 校验的安全 PDF 对象修复。
- **AtomicFileOutput**：从独立 `easypdf-io` crate 移入 `easypdf-core::io::atomic_file_output`，所有保存操作使用临时文件 + 原子重命名。
- **SSRF guard**：`easypdf-core::io::ssrf_guard` 校验出站 URL 白名单，防止 OCR HTTP 调用中的 SSRF。

### Changed

- **架构聚拢**：22 crate 合并为 9 crate（详见上方 Architecture 表格）。
- **Facade `EasyPdf::encrypt()` 完整实现**：取代之前的 `UnsupportedFeature` stub，支持 `PdfEncryption` builder 模式配置。
- **Facade `EasyPdf::sign()` 完整实现**：取代之前的 `UnsupportedFeature` stub，支持 `SignatureInfo` builder 模式配置。
- **`PdfEncryption` 新增字段**：`permissions`（PDF 权限位）+ `algorithm`（加密算法选择）+ builder 方法。
- **`SignatureInfo` 新增 X.509 元数据**：`signer_name` / `issuer` / `cert_not_before` / `cert_not_after`。
- **Markdown 处理器链重构**：基于 `ProcessorPipeline` 的 capability 协商机制。
- **Feature 体系重建**：修正潜伏 bug——`ocr` feature 不再未激活 `http-base`；`markdown-table`、`render`、`ocr` feature 现在启用 `easypdf-markdown` 内子模块而非独立 crate；`resident` 和 `mcp` feature 移至 `easypdf-runtime`。
- **文件大小拆分**：`streaming` / `lib.rs` 等文件均控制在 800 行以内，规范合规度从 80% 提升到 95%。
- **Clippy 配置**：workspace 级添加 `similar_names = "allow"`（`page_dict` / `pages_dict` PDF 对象名导致误报）。

### Fixed

- **Writer metadata UTF-16BE 编码持久化**：writer 正确将 UTF-16BE BOM + 编码写入 PDF metadata，reader 检测 BOM 解码。
- **百度 OCR Digit path 修正**：`digit` -> `numbers`，修正 API 端点路径。
- **百度 OCR Structured path 修正**：`structured` -> `smart_struct`，修正 API 端点路径。
- **parity roundtrip_metadata 测试通过**：writer metadata 持久化修复后，roundtrip 测试全部通过。
- **测试隔离修复**：每个 parity test 使用独立 tempdir，避免并行测试竞争。
- **byte_finder OOB panic**：fuzz 发现的越界 panic 修复。
- **双 hash 签名 bug**：签名前不再额外哈希（签名符合 CMS spec），修复签名验证失败。
- **rustdoc warnings**：修复 `easypdf-markdown` 中 broken intra-doc links（冗余显式链接目标、未解析模块路径）。

### Documentation

- CHANGELOG 0.2.0（本变更）
- 11 个 examples + `crates/easypdf/examples/README.md`
- `docs/printpdf-evaluation.md`（依赖评估报告）
- `docs/security/AUDIT.md` + `docs/security/AUDIT-IGNORED.md`
- `docs/performance/BENCHMARK.md`（性能基准报告）
- 0 rustdoc warning
- public-api 快照（5 个 crate，`api-snapshots/`）
- GitHub Actions CI 强化：OS x Rust 矩阵测试 + `RUSTFLAGS=-D warnings` + examples build + bench build + security.yml（cargo-audit + cargo-deny）

### Security Notes

- `rsa` crate 仅保留为 dev-dependency（用于测试证书生成，ring 无 keygen API）。
- 通过 `.cargo/audit.toml` ignore `RUSTSEC-2023-0071`（生产路径已用 ring constant-time 实现）。
- printpdf 4 个弃用传递依赖（bincode / rustls-pemfile / rustybuzz / ttf-parser）：跟踪上游，长期考虑 lopdf 替代。
- lru 0.16.4 unsound：计划通过 `[patch.crates-io]` 覆盖到 >=0.18.2。

## [0.1.0] — 2026-08-09

### Added
- 11-crate workspace: `easypdf`, `easypdf-core`, `easypdf-model`, `easypdf-io`, `easypdf-derive`, `easypdf-layout`, `easypdf-reader`, `easypdf-writer`, `easypdf-manipulate`, `easypdf-template`, `easypdf-markdown`
- Static factory `EasyPdf` with fluent builder API
- `#[derive(PdfModel)]` proc-macro for compile-time struct-to-PDF mapping
- PDF creation: text, built-in fonts (14 standard), metadata, pages (A4/Letter/Custom)
- PDF reading: text extraction, metadata extraction, single-parse session reuse (~129x speedup)
- PDF → Markdown: GFM/LLM/Plain profiles, zero-based page range, export report, structured warnings
- Merge: multiple PDFs into one with valid `/Pages` tree
- Split: PDF into individual pages with valid `/Pages` tree
- Rotate: per-page or all-page rotation (0°/90°/180°/270°)
- Reorder: arbitrary page reordering
- Template fill: AcroForm field replacement
- Writer lifecycle hooks: `PdfWriteHandler` with before/after document/page callbacks
- Event-driven read listeners: `PdfReadListener` trait
- Engine-neutral semantic IR: `PdfDocumentModel`, `PdfPageModel`, `PdfBlock` in `easypdf-model`
- Backend-neutral layout: `LayoutSink` trait in `easypdf-layout`, `FlowLayout`
- Atomic output: temp file + atomic rename for all save operations
- Resource limits: max file size (100 MB), max pages (10,000), max text length (10 MB)
- Type system: `PageSize`, `Orientation`, `Rotation`, `TextAlignment`, `PdfColor`, `PdfFont`, `BuiltInFont`
- Error handling: 7-variant `PdfError` enum with `thiserror`
- Bilingual README (EN/ZH), architecture design documents, usage guide
- Reader session benchmark: `cargo bench -p easypdf-reader --bench reader_session`
- 136 tests passing across all crates

### Known Limitations
- Encryption returns `UnsupportedFeature` (planned v0.4)
- Digital signatures return `UnsupportedFeature` (planned v0.5)
- Table detection, image extraction, OCR emit structured warnings in Markdown pipeline
- Custom TTF/OTF fonts: `register_font_from_path` exists but not fully integrated into all builders
