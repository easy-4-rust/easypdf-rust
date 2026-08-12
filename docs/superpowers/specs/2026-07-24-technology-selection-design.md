# easypdf-rust 技术选型设计

> **日期**: 2026-07-24
> **作者**: easypdf-rust team
> **状态**: 已采纳
> **依赖**: 无
> **来源**: 原 `docs/technology-selection.md`（已并入 superpowers 体系）

---

## 1. 背景与问题

easypdf-rust 作为纯 Rust PDF 操作库，需要在 Rust 生态中选择各技术域的实现方案。Java 基线为 PDFBox 3.x / iText 7.x / Apache POI，Rust 基线要求 Rust 1.88+、Edition 2024、Resolver 3。

核心定位：**纯 Rust PDF 库**，不依赖 JVM 运行时，通过 trait 注入实现可扩展性。

---

## 2. 候选方案对比

### 2.1 PDF 核心引擎

| 领域 | Java 组件 | Rust 组件 | crate | 版本 | 状态 | 说明 |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| PDF 读取/解析 | PDFBox `PDDocument.load()` | **lopdf** | `lopdf` | 0.44 | 已采用 | 纯 Rust PDF 解析；对象流、交叉引用表、页面树 |
| PDF 创建/写入 | PDFBox `PDDocument.save()` | **printpdf** | `printpdf` | 0.12 | 已采用 | 纯 Rust PDF 生成；文本、图片、矢量、字体 |
| PDF 文本提取 | PDFBox `PDFTextStripper` | **lopdf** + 自研 | `lopdf` | 0.44 | 已采用 | 基于 lopdf 的流内容解析 |
| PDF 元数据 | PDFBox `PDDocumentInformation` | **lopdf** | `lopdf` | 0.44 | 已采用 | `/Info` 字典读取 |
| PDF 表单 | PDFBox `PDAcroForm` | **lopdf** | `lopdf` | 0.44 | 已采用 | AcroForm 字段填充 |
| PDF 合并 | PDFBox `PDFMergerUtility` | **lopdf** 自研合并 | `lopdf` | 0.44 | 已采用 | 对象表合并 + `/Pages` 树构建 |
| PDF 拆分 | PDFBox `Splitter` | **lopdf** 自研拆分 | `lopdf` | 0.44 | 已采用 | 页面提取 + `/Pages` 树重建 |
| PDF 加密 | PDFBox `StandardDecryptionMaterial` | lopdf 加密扩展 | `lopdf` | 0.44 | 已实现 | ISO 32000 AES-128/256 |
| PDF 签名 | PDFBox `PDSignature` | ring + x509-parser | `ring` / `x509-parser` | 0.17 / 0.16 | 已实现 | PKCS#7/CMS detached SignedData |
| PDF/A 合规 | PDFBox `PDFACompliance` | — | — | — | 计划中 | 计划 v0.3 |

### 2.2 序列化与数据格式

| 领域 | Java 组件 | Rust 组件 | crate | 版本 | 状态 |
| :--- | :--- | :--- | :--- | :--- | :--- |
| JSON | Jackson | **serde + serde_json** | `serde` / `serde_json` | 1.x | 已采用 |
| YAML | SnakeYAML | 当前不需要 | — | — | 不迁移 |
| XML | JAXB / DOM4J | `quick-xml` 候选 | `quick-xml` | — | 按需引入 |
| TOML | — | Cargo 原生清单 | — | — | 不迁移 |

### 2.3 错误处理

| 领域 | Java 组件 | Rust 组件 | crate | 版本 | 状态 |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 检查异常 | `throws IOException` | **thiserror** | `thiserror` | 2.x | 已采用 |
| 运行时异常 | `RuntimeException` | **thiserror** | `thiserror` | 2.x | 已采用 |
| 错误传播 | `try-catch` | `?` 操作符 | std | — | 已采用 |

### 2.4 测试

| 领域 | Java 组件 | Rust 组件 | crate | 版本 | 状态 |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 单元测试 | JUnit 5 | **#[test]** | std | — | 已采用 |
| 编译期测试 | — | **trybuild** | `trybuild` | 1.x | 已采用 |
| 覆盖率 | JaCoCo | **cargo-llvm-cov** | `cargo-llvm-cov` | — | 已采用 |
| 属性测试 | — | **proptest** | `proptest` | 1.x | 计划中 |
| Fuzz | — | **cargo-fuzz** | `cargo-fuzz` | — | 已采用（6 targets） |

### 2.5 过程宏与编译期

| 领域 | Java 组件 | Rust 组件 | crate | 版本 | 状态 |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 注解处理 | `@interface` / APT | **syn** | `syn` | 3.x | 已采用 |
| 代码生成 | JavaPoet | **quote** | `quote` | 1.x | 已采用 |
| 宏辅助 | — | **proc-macro2** | `proc-macro2` | 1.x | 已采用 |

### 2.6 文件与 IO

| 领域 | Java 组件 | Rust 组件 | crate | 版本 | 状态 |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 临时文件 | `File.createTempFile()` | **tempfile** | `tempfile` | 3.x | 已采用 |
| 原子写入 | `Files.move(ATOMIC)` | 自研 `AtomicFileOutput` | std | — | 已采用 |

### 2.7 加密与安全

| 领域 | Java 组件 | Rust 组件 | crate | 版本 | 状态 |
| :--- | :--- | :--- | :--- | :--- | :--- |
| AES 加密 | JCE | **ring** | `ring` | 0.17 | 已采用 |
| RSA 签名 | JCE | **ring** (常量时间) | `ring` | 0.17 | 已采用 |
| X.509 证书 | Bouncy Castle | **x509-parser** | `x509-parser` | 0.16 | 已采用 |
| HTTP 客户端 | HttpClient | **reqwest** | `reqwest` | 0.12 | 已采用（OCR 云服务） |

---

## 3. 决策

### 选中 crate 与理由

| crate | 版本 | 用途 | 选择理由 |
| :--- | :--- | :--- | :--- |
| `lopdf` | 0.44 | PDF 读取/解析/操作/表单 | 纯 Rust，对象流支持，活跃维护 |
| `printpdf` | 0.12 | PDF 创建/写入 | 纯 Rust，文本/图片/矢量/字体支持 |
| `ring` | 0.17 | AES 加密 + RSA 签名 | 常量时间实现，修复 Marvin Attack (RUSTSEC-2023-0071) |
| `x509-parser` | 0.16 | X.509 证书解析 | DER/PEM 解析，与 ring 配合 |
| `reqwest` | 0.12 | HTTP 客户端（OCR 云服务） | 成熟，支持 blocking 模式 |
| `thiserror` | 2.x | 错误处理 | derive 宏，减少样板代码 |
| `serde` / `serde_json` | 1.x | 序列化 | Rust 生态标准 |
| `tempfile` | 3.x | 临时文件 | 原子输出支持 |
| `tracing` | 0.1 | 日志与可观测 | 结构化 span，可选 feature |
| `pulldown-cmark` | — | Markdown → HTML | Markdown→PDF 转换链路 |

### 多引擎策略

| 引擎 | 职责 | crate | 可替换性 |
| :--- | :--- | :--- | :--- |
| lopdf | 读取、解析、操作、表单 | `lopdf` | 通过 `easypdf-reader` 封装隔离 |
| printpdf | 创建、写入 | `printpdf` | 通过 `easypdf-writer` 封装隔离 |
| easypdf-model | 引擎无关 IR | 自研 | 不依赖任何引擎 |
| easypdf-io | 资源限制 + 原子输出 | 自研 | 不依赖任何引擎 |

引擎替换只需修改对应 domain crate 的内部实现，不影响 facade 和用户 API。

---

## 4. 不在范围内（YAGNI）

| Java 生态 | 理由 |
| :--- | :--- |
| PDFBox 的 `PDFRenderer` | 页面渲染为图片；不在 easypdf-rust 范围 |
| Apache POI | Word/Excel 格式；由 easydoc-rust / easyexcel-rs 负责 |
| Spring Boot Starter | Rust 无 Spring；使用 axum 扩展 crate |
| SLF4J + Logback 绑定 | 保持最小依赖；可选 feature 引入 tracing |
| JVM 内存管理 | Rust 所有权系统替代 GC |
| 反射 / 动态代理 | Rust 编译期零成本抽象替代 |

---

## 5. 风险与缓解

| 风险 | 可能性 | 影响 | 缓解措施 |
| :--- | :---: | :---: | :--- |
| printpdf API 大版本变更 | 中 | 高 | 锁版本，关注 changelog，提前适配 |
| lopdf API 大版本变更 | 中 | 中 | 同上 |
| Chromium 依赖（HTML→PDF） | 高 | 中 | 提供 feature gate，默认不启用 |
| 字体子集化复杂度（TTF） | 中 | 低 | 嵌入完整字体，后续子集化 |
| 加密实现安全性 | 高 | 高 | 使用 ring 常量时间实现，安全审计 |
| PDF 规范兼容性 | 中 | 中 | 用真实 PDF reader 验证输出 |

---

## 6. 实施顺序与状态

| 阶段 | 内容 | 状态 |
| :--- | :--- | :--- |
| v0.1.0-alpha.1 | 核心抽象：EasyPdf 门面 / derive 宏 / 读写/合并/拆分 | 已完成 |
| v0.1.0-alpha.2 | 丰富内容：表格/图片/字体/形状/页眉/多页/水印/加密/签名 | 已完成 |
| v0.1.0 | 架构聚拢：22→9 合并 / Streaming / CMap / OCR / MCP / 发布 | 已完成 |
| v0.2.0 | Rich Content 剩余 + OCR E2E + SVG | 规划中 |
| v0.3.0 | PDF/A 合规 + 转换器 | 规划中 |
| v1.0.0 | Semver 保证 + Windows + 属性测试 | 计划中 |

---

## 7. API 与性能设计

easypdf 采用"静态门面 + 专用 Builder + 终止操作"的三级 API：`EasyPdf::read(...)`、`EasyPdf::create(...)`、`EasyPdf::to_markdown(...)` 负责发现能力；Builder 只保存一次任务配置；`do_read`、`do_write`、`do_convert` 或 `save` 明确触发 I/O。

性能原则：一次任务只解析一次 PDF；页范围尽早下推；按页释放中间数据；字节输入避免临时文件；输出使用缓冲与原子替换；资源上限在昂贵分配前检查。

---

## 8. MarkItDown 吸纳边界

`easypdf-markdown` 是 PDF→Markdown 的唯一实现边界。已吸纳：转换结果对象、内存与文件双入口、可声明能力的处理器链、结构化警告。不吸纳：Word/Excel/PPT/HTML 总路由器、LLM 客户端、云 OCR SDK。

---

## 9. 版本记录

| 版本 | 日期 | 变更说明 |
| :--- | :--- | :--- |
| V1.1.0 | 2026-08-10 | 校正 Cargo 依赖事实；增加 MarkItDown 与 OfficeCLI 吸纳边界 |
| V1.0.0 | 2026-08-10 | 初始版本；16 个技术域 |
