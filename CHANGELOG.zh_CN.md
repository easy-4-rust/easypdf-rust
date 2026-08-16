# 变更日志

本项目所有重要变更均记录于此。

格式基于 [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)，
版本遵循 [语义化版本](https://semver.org/spec/v2.0.0.html)。

## [未发布]

### 新增

- **写入引擎抽象**：`WriteEngineKind` 枚举支持运行时引擎选择
  （`Printpdf` 默认 + `Krilla` 通过 `writer-krilla` feature）。
- **`PdfWriterBuilder::engine()`**：公共 API，构建时选择写入引擎。
- **`writer-krilla` feature**：krilla 0.8.2 后端，支持字体子集化和 CJK
  体积优化。限制：不支持 base14 内置字体（需提供真实字体文件）、不支持 SVG。
- **引擎对等测试**：6 个测试对比 printpdf 与 krilla 输出（页数、文本内容、
  图形、roundtrip、输出体积、字体子集化）。
- **引擎对比基准**：`docs/performance/ENGINE_COMPARISON.md`，含实测数据。
- **CI 扩展**：check/test/clippy 作业增加 `writer-krilla` feature 矩阵。

## [0.1.1] - 2026-08-16

质量与合规补丁版本。公共 API 无变化，所有既有路径保持有效。

### 修复

- **CI 恢复全绿**：全局 `bin/` 与 `*.json` gitignore 规则误伤了 4 个测试工具
  二进制与 6 个 golden 基线文件（均在 Cargo.toml/测试中声明），导致全新检出
  无法构建。已添加反向规则并补齐入库。
- **`pdfium` feature 恢复编译**：修复 pdfium-render 0.8.37 API 漂移
  （u16 页索引、`set_maximum_height` 构建器、非 Send/Sync 的 `Pdfium` 句柄）
  及失效的文档示例。
- **`ocrs` feature clippy 清零**：像素坐标的 pedantic 类型转换警告。
- **resident 端口文件测试竞态**：通过模块级互斥锁消除 Linux CI 并行测试的
  TOCTOU 竞争。
- **432 处 rustfmt 违规**（104 个文件）全量格式化。

### 变更

- **mod.rs 纯净化**：12 个 `mod.rs` 不再定义类型/函数；定义移入按类型命名的
  独立文件并以 `pub use` 重导出（全部公共路径不变）。
- **中文文档**：约 150 个生产文件的所有 pub 类型与 pub 方法补齐中文 doc
  注释（代码示例逐字节保留；测试模块不变）。
- **代码规范合规**：生产代码零 wildcard import、零 todo!/unimplemented!()、
  零超 800 行文件。
- **新增 deny.toml**：与依赖树匹配的显式许可证白名单、附理由的公告豁免；
  `cargo deny check` 四项全过。

### 验证

1535 测试 + 71 doctest（全 feature）；clippy/fmt/rustdoc 零警告；MSRV 1.88；
覆盖率 90.86% 行（排除开发工具 bin）。

## [0.1.0] - 2026-08-12

easypdf-rust 首次公开发布 -- 纯 Rust PDF 库，提供 builder API、OCR、
Markdown 转换、MCP server 和常驻守护进程。

### 新增

- **22 crate 整合为 9 crate**：工作区从约 22 个细粒度 crate 重构为 9 个聚焦
  crate，降低编译时间和依赖图复杂度。
  （映射关系见 [docs/easypdf-rust-Architecture.zh_CN.md](docs/easypdf-rust-Architecture.zh_CN.md)）
- **Streaming ReadStrategy**：`ReadStrategy::Streaming` 执行字节流扫描，不构建
  完整 `Document` 对象。`ReadStrategy::auto` 按文件大小自动选择最优策略
  （Full < 5 MB，Lazy 5-100 MB，Streaming > 100 MB）。
- **CMap / ToUnicode 支持**：`easypdf-reader` 正确处理 CMap 编码字体，修复
  CJK 文本提取乱码问题。
- **WriteBackend 选择**：`easypdf-writer` 支持 `InMemory`（默认）、`Spill`
  （页面级临时文件，恒定内存）和 `Auto`（阈值自动切换）三种后端。
- **PdfWriterBuilder + WriteHandlerChain**：可组合写处理器 pipeline，按优先级
  稳定排序执行。
- **ConverterRegistry**：`easypdf-core::converter_registry` 类型擦除双向
  转换器注册表。
- **ProcessorPipeline 调度器**：capability 协商 + priority 稳定排序，统一
  markdown 处理器链。
- **4 大云 OCR 引擎**（新 `easypdf-ocr` crate）：
  - GLM-OCR（智谱 BigModel，feature-gated `ocr-glm`）
  - HunyuanOCR（腾讯云，TC3-HMAC-SHA256 签名，feature-gated `ocr-hunyuan`）
  - 百度 Qianfan / PP-OCRv6（14 个 API 端点 + OAuth Token 管理，feature-gated `ocr-baidu`）
  - DeepSeek-OCR-2（OpenAI 兼容协议，feature-gated `ocr-deepseek`）
  - 统一 `HttpOcrEngine` trait，reqwest blocking HTTP，base64 图片编码，
    结构化 `OcrHttpError`。
- **常驻守护进程**（新 `easypdf-runtime` crate）：Unix socket + Windows TCP
  fallback（`Transport` trait 抽象），自适应 autosave（EMA 平滑），
  空闲超时看门狗。
- **MCP server**（新 `easypdf-runtime` crate）：7 个工具（`pdf_read_text`、
  `pdf_to_markdown`、`pdf_create_text`、`pdf_merge`、`pdf_split`、
  `pdf_metadata`、`pdf_page_count`），stdio JSON-RPC 协议，供 LLM agent
  集成使用。
- **PdfBlock IR 扩展**：从 5 变体扩展到 14 变体（新增 Code、Formula、
  PageBreak、Footnote、TableCell、BlockQuote、HorizontalRule、Link、Unknown）。
- **easypdf-derive 属性**：8 个新属性 -- `field`、`order`、`skip`、`default`、
  `required`、`format`、`nested`、`font`/`size`。
- **tracing 可观测性**：workspace 级 `tracing` + `tracing-subscriber`
  （`env-filter` + JSON 输出），结构化 span 覆盖
  reader/writer/markdown/IPC。
- **Transport trait**：`easypdf-runtime::transport` 提供 Unix socket（默认）
  和 Windows TCP fallback 的统一接口。
- **ISO 32000 加密**：AES-128（V4/R4）和 AES-256（V5/R6）加密，完整权限控制
  （PRINT、MODIFY、COPY、FILL_FORMS 等）。
- **ISO 32000 数字签名**：PKCS#7/CMS detached SignedData，RSA-PKCS#1v1.5
  + SHA-256（via `ring`），X.509 证书解析（via `x509-parser`）。
- **cargo-fuzz**：6 个 fuzz targets -- `pdf_parse`、`streaming_scan`、
  `pdf_encrypt_decrypt`、`pdf_sign_verify`、`markdown_convert`、`ssrf_url`。

### 安全

- **RUSTSEC-2023-0071 修复（Marvin Attack）**：生产代码路径从 `rsa` 迁移到
  `ring` 0.17.14 constant-time RSA。`rsa` 仅保留为 dev-dependency（用于测试
  证书生成，ring 无 keygen API）。通过 `.cargo/audit.toml` 忽略该 advisory。
- **RUSTSEC-2025-0055 修复**：`tracing-subscriber` 升级到 >=0.3.20，修复
  ANSI 转义序列注入漏洞。
- **解压炸弹 guard 修复**：移除 64 KB 豁免，改为按绝对解压大小检查（不论
  输入大小）。
- **SSRF IPv6 防护**：全覆盖 IPv6 -- loopback、ULA、link-local 和
  IPv4-mapped 地址均纳入拦截范围。
- **API key Debug redact**：含密钥结构体（`GlmConfig`、`BaiduConfig` 等）
  不再在 `Debug` 输出中泄露 `api_key` / `secret_key`。
- **双 hash 签名 bug 修复**：签名前不再额外哈希（符合 CMS spec），修复
  签名验证失败。

### 变更

- **架构聚拢**：22 crate 合并为 9 crate（见映射表）。
- **`EasyPdf::encrypt()` 完整实现**：取代之前的 `UnsupportedFeature` stub，
  支持 `PdfEncryption` builder 模式配置。
- **`EasyPdf::sign()` 完整实现**：取代之前的 `UnsupportedFeature` stub，
  支持 `SignatureInfo` builder 模式配置。
- **`PdfEncryption` 新增字段**：`permissions`（PDF 权限位）+ `algorithm`
  （加密算法选择）+ builder 方法。
- **`SignatureInfo` 新增字段**：X.509 元数据 -- `signer_name`、`issuer`、
  `cert_not_before`、`cert_not_after`。
- **Markdown 处理器链重构**：基于 `ProcessorPipeline` 的 capability
  协商机制。
- **Feature 体系重建**：修复潜在 bug -- `ocr` feature 不再未激活
  `http-base`；`markdown-table`、`render`、`ocr` feature 现在启用
  `easypdf-markdown` 内子模块而非独立 crate；`resident` 和 `mcp` feature
  移至 `easypdf-runtime`。
- **文件大小拆分**：`streaming`、`lib.rs` 等文件控制在 800 行以内，
  合规度从 80% 提升到 95%。
- **Clippy 配置**：workspace 级添加 `similar_names = "allow"`
  （PDF 对象名 `page_dict` / `pages_dict` 导致误报）。

### 修复

- **Writer metadata UTF-16BE 编码持久化**：writer 正确将 UTF-16BE BOM +
  编码写入 PDF metadata，reader 检测 BOM 解码。
- **百度 OCR Digit 路径修正**：`digit` -> `numbers`（修正 API 端点路径）。
- **百度 OCR Structured 路径修正**：`structured` -> `smart_struct`（修正
  API 端点路径）。（注：此为端点路径修正，非逻辑 bug。）
- **parity roundtrip_metadata 测试**：writer metadata 持久化修复后，
  roundtrip 测试全部通过。
- **测试隔离修复**：每个 parity test 使用独立 tempdir，避免并行测试竞争。
- **byte_finder OOB panic**：fuzz 发现的越界 panic 修复。
- **rustdoc warnings**：修复 `easypdf-markdown` 中 broken intra-doc links
  （冗余显式链接目标、未解析模块路径）。

### 文档

- 14 个中英双语文档（`docs/` 目录）。
- 11 个示例 + `crates/easypdf/examples/README.md`。
- `docs/security/AUDIT.md` + `docs/security/AUDIT-IGNORED.md`。
- `docs/performance/BENCHMARK.md`。
- workspace 0 rustdoc warning。
- Roadmap 与实际 v0.2 完成状态同步。

[未发布]: https://github.com/easy-4-rust/easypdf-rust/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/easy-4-rust/easypdf-rust/releases/tag/v0.1.0
