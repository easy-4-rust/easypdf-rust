# easypdf-markdown 双向转换设计

- **日期**：2026-08-09
- **作者**：ZCode Agent（协同设计）
- **状态**：已实现（v0.1.0 发布）
- **依赖**：easypdf-markdown 现有 `markdown_profile.rs`、`processor_pipeline.rs`、`pdf_markdown_processor.rs`、`markdown_renderer.rs`、`table/`、`ocr_policy.rs`

## 1. 目标与范围

为 easypdf-rust 实现**PDF→Markdown 单向转换**（当前版本），通过 ProcessorPipeline 能力协商和优先级排序，支持 GFM/LLM/Plain 三种输出配置文件，覆盖表格检测、链接提取、图片策略、OCR 策略等横切面。

**核心需求**：

1. `ProcessorPipeline` 支持能力协商和优先级排序。
2. `MarkdownProfile` 支持 Gfm / Llm / Plain 三种预设配置。
3. `PdfMarkdownBuilder` 提供链式配置 API（image_policy / table_policy / ocr_policy）。
4. 表格检测支持 heuristic（启发式）和 parser（精确解析）两种模式。
5. 图片策略支持 Skip / Link / Inline 三种模式。
6. OCR 策略支持 Skip / Cloud(GLM/Hunyuan/Baidu/DeepSeek) 四种模式。
7. 渲染后端支持 pdfium（外部）和 text（内置）两种。

**非目标**：

- 不实现 Markdown→PDF 转换（当前版本仅通过 HTML 中转，未来版本原生支持）。
- 不实现 DOCX→Markdown 转换（这是 easydoc-rust 的职责）。
- 不支持自定义 Processor 的动态加载。
- 不支持 PDF 页面布局还原（仅提取语义内容）。

## 2. 总体架构

```
┌──────────────────────────────────────────────────────────────┐
│                     easypdf-markdown                          │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐  │
│  │  PdfMarkdownBuilder                                    │  │
│  │  ├── profile(Gfm / Llm / Plain)                       │  │
│  │  ├── image_policy(Skip / Link / Inline)                │  │
│  │  ├── table_policy(Skip / Detect / Parse)               │  │
│  │  ├── ocr_policy(Skip / Cloud{engine})                  │  │
│  │  └── build() → PdfMarkdownProcessor                    │  │
│  └────────────────────────────────────────────────────────┘  │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐  │
│  │  ProcessorPipeline                                     │  │
│  │  ├── heading_detector    (PRIORITY_SPECIFIC)           │  │
│  │  ├── link_extractor      (PRIORITY_SPECIFIC)           │  │
│  │  ├── table_detector      (PRIORITY_SPECIFIC)           │  │
│  │  ├── reading_order       (PRIORITY_GENERIC)            │  │
│  │  ├── aggregate_capabilities()                          │  │
│  │  └── with_target_level(capabilities)                   │  │
│  └────────────────────────────────────────────────────────┘  │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐  │
│  │  MarkdownRenderer                                      │  │
│  │  ├── render_gfm()      GFM 表格 / 代码块 / 链接       │  │
│  │  ├── render_llm()      LLM 友好格式（简化标记）        │  │
│  │  └── render_plain()    纯文本（无标记）                │  │
│  └────────────────────────────────────────────────────────┘  │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐  │
│  │  table/                                                │  │
│  │  ├── heuristic.rs   启发式表格检测（空格对齐）         │  │
│  │  ├── parser.rs      精确表格解析（PDF 表格对象）        │  │
│  │  └── mod.rs         TablePolicy 枚举                   │  │
│  └────────────────────────────────────────────────────────┘  │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐  │
│  │  processors/                                           │  │
│  │  ├── heading_detector.rs   标题检测                    │  │
│  │  ├── link_extractor.rs     链接提取                    │  │
│  │  └── reading_order.rs      阅读顺序                    │  │
│  └────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────┘
```

## 3. 模块职责划分

### 3.1 `MarkdownProfile` — 输出配置文件

| 配置 | 特点 | 适用场景 |
|---|---|---|
| `Gfm` | GFM 表格、代码块、链接、图片引用 | GitHub / GitLab 文档 |
| `Llm` | 简化标记、结构化输出、无图片 | LLM 上下文输入 |
| `Plain` | 纯文本、无标记、保留段落结构 | 纯文本搜索 / 索引 |

**每个 Profile 内置默认 ProcessorPipeline 配置**：

```rust
impl MarkdownProfile {
    pub fn gfm() -> MarkdownProfileBuilder {
        // heading_detector + link_extractor + table_detector + reading_order
    }
    pub fn llm() -> MarkdownProfileBuilder {
        // heading_detector + reading_order（无图片、无链接）
    }
    pub fn plain() -> MarkdownProfileBuilder {
        // reading_order only
    }
}
```

### 3.2 `ProcessorPipeline` — 能力协商管道

| 组件 | 优先级 | 职责 |
|---|---|---|
| `heading_detector` | PRIORITY_SPECIFIC | 检测标题（H1-H6） |
| `link_extractor` | PRIORITY_SPECIFIC | 提取超链接 |
| `table_detector` | PRIORITY_SPECIFIC | 检测表格（启发式 + 精确） |
| `reading_order` | PRIORITY_GENERIC | 确定阅读顺序 |

**能力协商**：

```
1. 每个 Processor 声明自己的 ProcessorCapability
2. Pipeline 调用 aggregate_capabilities() 合并
3. 用 with_target_level(capabilities) 过滤不需要的 Processor
4. 按优先级排序执行
```

**关键设计**：
- `fail_fast(true)` 时遇到错误立即停止
- `fail_fast(false)` 时跳过错误继续处理
- Processor 可以动态添加（`add_processor()`）

### 3.3 `MarkdownRenderer` — 渲染器

| 方法 | 输出格式 |
|---|---|
| `render_gfm(blocks)` | GFM 表格（`| col1 | col2 |`）、代码块（` ``` `）、链接（`[text](url)`） |
| `render_llm(blocks)` | 简化标记、结构化输出、无图片 |
| `render_plain(blocks)` | 纯文本、无标记、保留段落结构 |

### 3.4 `table/` — 表格检测

| 模式 | 算法 | 精度 |
|---|---|---|
| `heuristic` | 空格对齐检测（列宽一致性） | 中等，速度快 |
| `parser` | PDF 表格对象精确解析 | 高，速度慢 |

**TablePolicy 枚举**：
```rust
pub enum TablePolicy {
    Skip,      // 跳过表格
    Detect,    // 启发式检测
    Parse,     // 精确解析
}
```

### 3.5 `ocr_policy.rs` — OCR 策略

| 策略 | 行为 |
|---|---|
| `Skip` | 跳过图片 OCR |
| `Cloud { engine }` | 调用云 OCR 引擎（GLM / Hunyuan / Baidu / DeepSeek） |

**OCR 集成流程**：
1. 提取 PDF 中的图片
2. 根据 OCR 策略选择引擎
3. 调用 `easypdf-ocr` 的 `HttpOcrEngine`
4. 将 OCR 结果插入 Markdown 输出

### 3.6 `image_policy.rs` — 图片策略

| 策略 | 行为 |
|---|---|
| `Skip` | 跳过图片 |
| `Link` | 输出图片引用链接（`![alt](path)`） |
| `Inline` | 内联图片数据（base64） |

### 3.7 `markdown_warning.rs` — 警告

| 警告类型 | 触发条件 |
|---|---|
| `UnsupportedImageFormat` | 图片格式不支持 |
| `OcrFailed` | OCR 调用失败 |
| `TableDetectionFailed` | 表格检测失败 |
| `PartialExtraction` | 部分内容提取失败 |

## 4. 关键数据流

### 4.1 PDF→Markdown 完整流程

```
input.pdf
    │
    ▼
PdfReader::extract_text() / extract_metadata()
    │
    ▼
PdfDocumentModel { pages: [PdfPageModel { blocks: [PdfBlock] }] }
    │
    ▼
ProcessorPipeline::execute()
    ├── heading_detector: PdfBlock::Heading → Markdown heading
    ├── link_extractor: PdfBlock::Link → Markdown link
    ├── table_detector: 空格对齐 / PDF 表格对象 → Markdown table
    └── reading_order: 确定块的阅读顺序
    │
    ▼
MarkdownRenderer::render_gfm() / render_llm() / render_plain()
    │
    ▼
MarkdownConversionResult {
    markdown: String,
    warnings: Vec<MarkdownWarning>,
    metadata: PdfMetadata,
}
```

### 4.2 表格检测流程

```
PdfPageModel { blocks: [...] }
    │
    ▼
table_detector::detect(blocks)
    │
    ├── heuristic: 扫描文本块，检测空格对齐模式
    │   └── 列宽一致性 > 阈值 → 识别为表格
    │
    └── parser: 检测 PdfBlock::Table 变体
        └── 直接提取表格结构
    │
    ▼
Markdown table: | col1 | col2 |\n|---|---|\n| val1 | val2 |
```

### 4.3 OCR 集成流程

```
PdfBlock::Image { data, format }
    │
    ▼
OcrPolicy::Cloud { engine: "glm" }
    │
    ▼
easypdf_ocr::HttpOcrEngine::recognize(image)
    │
    ▼
OcrResult { text, confidence, regions }
    │
    ▼
Markdown 输出: [OCR text: "识别结果"]
```

## 5. 技术决策与权衡

| # | 决策 | 理由 | 权衡 |
|---|---|---|---|
| 1 | 仅支持 PDF→Markdown（不支持反向） | PDF 是只读格式，反向转换信息损失大 | 用户无法从 Markdown 重建 PDF |
| 2 | ProcessorPipeline 用优先级排序 | 确保处理顺序确定性 | 需要手动管理优先级值 |
| 3 | 表格检测用 heuristic + parser 双模式 | 兼顾速度和精度 | heuristic 有误报风险 |
| 4 | OCR 用 feature gate 控制 | 避免引入不需要的 HTTP 依赖 | 用户需要手动启用 feature |
| 5 | 三种 Profile 预设 | 降低用户配置成本 | 无法满足所有场景 |
| 6 | 警告而非错误 | 尽最大努力提取 | 用户可能忽略重要警告 |

## 6. 测试与验收

### 6.1 现有测试覆盖

| 测试 | 断言点 | 文件 |
|---|---|---|
| `test_gfm_profile` | GFM 输出格式正确 | `markdown_profile.rs` tests |
| `test_llm_profile` | LLM 输出格式正确 | `markdown_profile.rs` tests |
| `test_plain_profile` | Plain 输出格式正确 | `markdown_profile.rs` tests |
| `test_heading_detector` | 标题检测正确 | `processors/` tests |
| `test_link_extractor` | 链接提取正确 | `processors/` tests |
| `test_table_heuristic` | 启发式表格检测 | `table/tests.rs` |
| `test_table_parser` | 精确表格解析 | `table/tests.rs` |
| `test_pipeline_capability` | 能力协商正确 | `processor_pipeline.rs` tests |
| `test_pipeline_fail_fast` | fail_fast 行为正确 | `processor_pipeline.rs` tests |
| `test_ocr_skip` | OCR 跳过策略 | `ocr_policy.rs` tests |
| `test_image_skip` | 图片跳过策略 | `image_policy.rs` tests |
| `test_markdown_warning` | 警告生成正确 | `markdown_warning.rs` tests |

### 6.2 已知局限

- Markdown→PDF 仅通过 HTML 中转，不支持原生 Markdown 语法。
- 表格检测 heuristic 有误报风险（非表格的空格对齐文本）。
- OCR 端到端集成未完全接通（引擎就绪，但 Markdown 转换中的图片 OCR 集成待完成）。
- 不支持 PDF 页面布局还原（列、页脚等）。
- 不支持自定义 Processor 的动态加载。

## 7. 引用

- 架构文档：`docs/easypdf-rust-Architecture.md` 第 7 节「easypdf-markdown 转换」
- 使用指南：`docs/usage-guide.md` 第 8 节「Markdown 转换」
- Roadmap：`docs/roadmap.md` 0.1 Foundation（基础转换）、0.3 Rich Content（增强）
- 源码：`crates/easypdf-markdown/src/`（markdown_profile.rs / processor_pipeline.rs / table/ / processors/）
