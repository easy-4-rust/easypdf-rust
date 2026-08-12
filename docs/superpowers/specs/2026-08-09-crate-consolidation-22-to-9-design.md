# Crate Consolidation 22-to-9 Design

**日期**: 2026-08-09
**作用范围**: 整个 workspace（所有 9 个 crate）
**类型**: 架构重构

---

## 1. 背景与问题

easypdf-rust 最初采用约 22 个细粒度 crate 的架构，每个功能模块（如 `easypdf-table`、`easypdf-image`、`easypdf-font`、`easypdf-encrypt`、`easypdf-sign` 等）都是独立 crate。这种设计在早期提供了清晰的模块边界，但随着功能增长带来了以下问题：

### 1.1 编译时间过长
22 个 crate 意味着 22 次独立的 rustc 调用，即使 Cargo 并行编译，依赖链导致的串行瓶颈仍然显著。

### 1.2 依赖图复杂度
crate 之间的依赖关系错综复杂，尤其是 `easypdf-core` 被几乎所有 crate 依赖，任何 core 的改动都触发全量重编译。

### 1.3 Feature 体系缺陷
- `ocr` feature 静默激活 `http-base`，导致启用 OCR 时意外引入 HTTP 依赖。
- `markdown-table`、`render`、`ocr` 对应的是独立 crate，而非 `easypdf-markdown` 内的子模块。
- `resident` 和 `mcp` 的 feature 归属不清晰。

### 1.4 发布管理负担
22 个 crate 的版本同步、发布顺序、crates.io 管理成本过高。

---

## 2. 设计方案

### 2.1 九 Crate 分层架构

```
Layer 0 (Foundation):
  easypdf-core      -- 模型 / IO / 加密签名 / 事件 / 布局 / 转换器注册 / 处理器链

Layer 1 (Engines):
  easypdf-reader    -- PDF 读取 / 文本提取 / 操作（合并/拆分/旋转/重排）/ streaming
  easypdf-writer    -- PDF 写入 / 后端选择 / 字体 / 图片 / 形状 / 模板
  easypdf-derive    -- #[derive(PdfModel)] 过程宏

Layer 2 (Domain):
  easypdf-markdown  -- Markdown→PDF 转换 / 处理器管道 / OCR 集成接口
  easypdf-ocr       -- 4 云 OCR 引擎（GLM / Hunyuan / Baidu / DeepSeek）
  easypdf-runtime   -- MCP server / resident daemon / transport

Layer 3 (Facade):
  easypdf           -- 门面 API（EasyPdf / builders / crypto_facade / prelude）
  easypdf-test      -- 集成测试 + parity 测试（不发布到 crates.io）
```

### 2.2 合并映射表

| 原 crate（约 22 个） | 目标 crate | 合并理由 |
|---|---|---|
| easypdf-model, easypdf-io, easypdf-error, easypdf-crypto-encrypt, easypdf-crypto-sign, easypdf-event, easypdf-handler-chain, easypdf-layout, easypdf-converter-registry | **easypdf-core** | 基础设施层，被所有上层依赖 |
| easypdf-reader, easypdf-extract, easypdf-manipulate, easypdf-streaming, easypdf-cmap | **easypdf-reader** | 读取相关功能内聚 |
| easypdf-writer, easypdf-backend, easypdf-font, easypdf-image, easypdf-shape, easypdf-template | **easypdf-writer** | 写入相关功能内聚 |
| easypdf-derive | **easypdf-derive** | 独立 proc-macro crate，不合并 |
| easypdf-markdown, easypdf-table-detect, easypdf-ocr-processor | **easypdf-markdown** | Markdown 处理 + OCR 集成 |
| easypdf-ocr-glm, easypdf-ocr-hunyuan, easypdf-ocr-baidu, easypdf-ocr-deepseek | **easypdf-ocr** | 统一 OCR 引擎抽象 |
| easypdf-mcp, easypdf-resident | **easypdf-runtime** | 运行时服务 |
| easypdf | **easypdf** | 门面，不变 |
| easypdf-test | **easypdf-test** | 集成测试，不变 |

### 2.3 Feature 体系重建

```toml
# easypdf/Cargo.toml
[features]
default = []
markdown = ["easypdf-markdown"]
markdown-table = ["easypdf-markdown/table"]
render = ["easypdf-markdown/render"]
ocr = ["easypdf-ocr"]              # 不再静默激活 http-base
ocr-glm = ["easypdf-ocr/glm"]
ocr-hunyuan = ["easypdf-ocr/hunyuan"]
ocr-baidu = ["easypdf-ocr/baidu"]
ocr-deepseek = ["easypdf-ocr/deepseek"]
resident = ["easypdf-runtime/resident"]
mcp = ["easypdf-runtime/mcp"]
html = ["printpdf/html"]
```

关键修复：
- `ocr` feature 不再隐式激活 HTTP 基础设施，HTTP 依赖由各 OCR 引擎的 feature gate 控制。
- `markdown-table` / `render` / `ocr` 启用 `easypdf-markdown` 内的子模块，而非引用独立 crate。

### 2.4 子模块隔离策略

每个合并后的 crate 内部使用 `pub(crate)` 可见性隔离实现细节：

- `easypdf-core::crypto::encrypt` -- 加密实现，只通过 `pub use` 导出公共 API
- `easypdf-reader::streaming` -- streaming 实现，只通过 `ReadStrategy::Streaming` 暴露
- `easypdf-writer::backend` -- WriteBackend 实现，只通过 `WriteBackend` 枚举暴露
- `easypdf-ocr::http` -- HTTP 客户端实现，只通过 `HttpOcrEngine` trait 暴露

---

## 3. 测试改动范围

- 所有 crate 内的 `use` 语句需要更新路径（如 `use easypdf_model::PdfBlock` → `use easypdf_core::model::PdfBlock`）
- 所有 `Cargo.toml` 的依赖声明需要更新
- 集成测试（`easypdf-test/`）需要更新外部依赖引用
- parity 测试（roundtrip 测试）需要验证合并后行为不变

---

## 4. 不在范围内（YAGNI）

- 不引入 workspace-level feature aggregation（每个 crate 独立管理 feature）
- 不创建 `easypdf-common` 共享工具 crate（`easypdf-core` 已承担此角色）
- 不拆分 `easypdf-core` 为更细的 crate（如 `easypdf-model` + `easypdf-crypto`）
- 不引入 `build.rs` 自动生成映射表

---

## 5. 风险与缓解

| 风险 | 影响 | 缓解 |
|---|---|---|
| 合并后循环依赖 | 编译失败 | 严格分层，Layer N 只依赖 Layer 0..N-1 |
| feature 组合爆炸 | 测试覆盖不足 | CI 矩阵测试关键 feature 组合 |
| 公共 API 路径变更 | 用户代码破坏 | 提供 prelude 重导出，保持类型名不变 |
| 单 crate 编译时间过长 | 开发体验下降 | 使用 `pub(crate)` 减少增量编译范围 |

---

## 6. 实施顺序

1. 设计 9-crate 分层架构和合并映射表
2. 合并 easypdf-core（最大的合并，涉及最多原 crate）
3. 合并 easypdf-reader 和 easypdf-writer
4. 合并 easypdf-markdown
5. 创建 easypdf-ocr 和 easypdf-runtime
6. 更新 easypdf 门面
7. 重建 feature 体系
8. 更新所有测试
9. 验证 `cargo test` 全 workspace 通过
10. 更新文档
