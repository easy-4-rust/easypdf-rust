# easypdf-core 数据模型与扩展 Trait 设计

- **日期**：2026-08-09
- **作者**：ZCode Agent（协同设计）
- **状态**：已实现（v0.1.0 发布）
- **依赖**：easypdf-core 现有 `model/`、`traits.rs`、`error.rs`、`content.rs`、`converter_registry.rs`、`handler_chain.rs`、`event.rs`

## 1. 目标与范围

为 easypdf-rust 定义**唯一语义模型**与**扩展 trait 体系**，使所有下游 crate（reader、writer、markdown、ocr、runtime）统一消费 `easypdf-core` 提供的类型与接口，不再各自建立平行数据结构。

**核心需求**：

1. `PdfDocumentModel` + `PdfPageModel` + `PdfBlock` 作为读/写/渲染的唯一中间表示（IR）。
2. 所有 `PdfBlock` 变体实现 `Clone + Debug + PartialEq`，支持序列化（serde 可选）。
3. `PdfModel` derive 宏自动为 struct 生成 PDF 模型映射。
4. `ConverterRegistry` 支持自定义值转换，做运行时分发。
5. `PdfReadListener` / `PdfWriteHandler` 为读/写提供生命周期钩子。
6. `WriteHandlerChain` 提供优先级排序的稳定执行管道。
7. 错误模型 `PdfError` 覆盖 IO、解析、加密、签名、安全违规等全部场景。

**非目标**：

- 不提供运行时 schema 校验（如 JSON Schema 验证 `PdfDocumentModel`）。
- 不引入泛型参数到 `PdfDocumentModel`（保持零泛型、易序列化）。
- 不在 core 层引入任何 PDF 解析或 ZIP 操作（这些由 reader/writer 承担）。
- 不支持 PDF 写入的运行时引擎切换（当前仅 printpdf）。

## 2. 总体架构

```
┌─────────────────────────────────────────────────────────────┐
│                     easypdf (facade)                        │
│  EasyPdf::create() / read() / manipulate() / encrypt()      │
└──────────────────────┬──────────────────────────────────────┘
                       │ 消费
        ┌──────────────┼──────────────┬──────────────┐
        ▼              ▼              ▼              ▼
   easypdf-reader  easypdf-writer  easypdf-markdown  easypdf-runtime
        │              │              │              │
        └──────────────┴──────────────┴──────────────┘
                       │ 依赖
                       ▼
              ┌─────────────────┐
              │  easypdf-core   │
              │                 │
              │ PdfDocumentModel│◄── 唯一 IR
              │ PdfPageModel    │
              │ PdfBlock (14)   │◄── 语义块枚举
              │ ConverterRegistry◄── 值转换 trait
              │ WriteHandlerChain◄── 写入钩子
              │ PdfReadListener │◄── 读取钩子
              │ PdfError        │◄── 统一错误
              │ content / style │◄── 基础类型
              └─────────────────┘
                       ▲
                       │ 仅类型依赖
              ┌─────────────────┐
              │ easypdf-derive  │
              │#[derive(PdfModel)]
              └─────────────────┘
```

## 3. 模块职责划分

### 3.1 `model/` — 语义模型

| 类型 | 职责 | 当前状态 |
|---|---|---|
| `PdfDocumentModel` | 顶层容器：元数据 + 页面列表 | `[已实现]` |
| `PdfPageModel` | 页面级：尺寸 + 内容块列表 | `[已实现]` |
| `PdfBlock` | 枚举：14 个变体（Heading/Paragraph/Table/List/Image/Code/Formula/PageBreak/Footnote/TableCell/BlockQuote/HorizontalRule/Link/Unknown） | `[已实现]` |
| `PdfBlockType` | PdfBlock 的类型标识枚举 | `[已实现]` |
| `PdfText` / `PdfFont` / `PdfColor` | 文本/字体/颜色基础类型 | `[已实现]` |
| `PdfImage` / `PdfTable` / `PdfTableCell` | 图片/表格/单元格类型 | `[已实现]` |
| `SourceLocation` | 源位置追踪 | `[已实现]` |

**PdfBlock 变体清单（14 个）**：

```
Heading / Paragraph / Table / List / Image          -- Phase 1 初始 5 个
Code / Formula / PageBreak / Footnote               -- Phase 2 扩展
TableCell / BlockQuote / HorizontalRule / Link      -- Phase 2 扩展
Unknown                                             -- 兜底变体
```

### 3.2 `traits.rs` — 扩展 Trait

| Trait | 签名概要 | 职责 |
|---|---|---|
| `PdfReadListener` | `invoke(data, ctx)`, `on_complete(ctx)`, `on_error(err, ctx)`, `has_next(ctx)` | 读取生命周期钩子 |
| `PdfWriteHandler` | `before_document/after_document`, `before_page/after_page`, `before_text/after_text`, `before_image/after_image` + `order()` | 写入生命周期钩子 |
| `LayoutSink` | `write_text()`, `write_image()`, `new_page()` | 后端无关布局抽象 |
| `PdfModel` | derive 宏生成的 trait | struct 到 PDF 模型映射 |

### 3.3 `converter_registry.rs` / `handler_chain.rs` / `event.rs` — 基础设施

| 模块 | 内容 |
|---|---|
| `converter_registry.rs` | `ConverterRegistry`（HashMap<TypeId, Box<dyn Any>>）+ `with_defaults()` |
| `handler_chain.rs` | `WriteHandlerChain`（优先级排序的稳定执行管道） |
| `event.rs` | `PdfEvent` 枚举（PageStart / PageEnd / TextWritten / ImageWritten / Error） |
| `error.rs` | `PdfError`（Io / Parse / InvalidPdf / UnsupportedFeature / Encryption / Signature / SecurityViolation） |
| `content.rs` | `PdfText` / `PdfFont` / `FontFamily` / `PdfImage` / `PdfTable` / `PdfTableCell` |
| `style.rs` | `PdfColor` / `FontStyle` / `TextAlignment` |
| `metadata.rs` | `PdfMetadata`（title / author / subject / keywords / creator / producer） |
| `logging.rs` | `init_compact()` / `init_json()` tracing 初始化 |
| `enums.rs` | `Rotation` / `PageFormat` / `ImageFormat` 等枚举 |
| `page_range.rs` | `PageRange` 解析（"1-3,5" 模式） |
| `page_index.rs` | 零基到一基转换 |
| `page_number.rs` | 页码工具 |

## 4. 关键数据流

### 4.1 PDF 创建：EasyPdf → printpdf

```
EasyPdf::create(path)
    │
    ▼
PdfCreateBuilder::add_text() / add_table() / add_image()
    │                           构建 PdfBlock IR
    ▼
PdfWriter::write_blocks(blocks)
    │
    ▼
printpdf::PdfDocument → 原子文件输出
```

### 4.2 PDF 读取：lopdf → PdfDocumentModel

```
input.pdf
    │
    ▼
PdfReader::open(path)         → lopdf::Document
    │
    ▼
ReadStrategy::auto(size)      → Full / Lazy / Streaming
    │
    ▼
extract_text() / extract_metadata()  → PdfDocumentModel
    │
    ▼
PdfReadListener 回调           → 事件通知
```

### 4.3 Markdown 转换管道

```
input.pdf
    │
    ▼
PdfReader → PdfDocumentModel
    │
    ▼
ProcessorPipeline (heading_detector / link_extractor / table_detector / reading_order)
    │
    ▼
MarkdownRenderer → String
```

## 5. 技术决策与权衡

| # | 决策 | 理由 | 权衡 |
|---|---|---|---|
| 1 | `PdfBlock` 用 enum 而非 trait object | 模式匹配简洁、无堆分配、序列化友好 | 新增变体需修改 enum，破坏兼容性 |
| 2 | `ConverterRegistry` 用 TypeId 做 key | 运行时零成本分发 | 无法在编译期检查注册完整性 |
| 3 | `PdfError` 用 thiserror 派生 | 减少样板代码 | 变体不可跨 crate 精细化捕获 |
| 4 | `PdfDocumentModel` 不带泛型参数 | 简化序列化、传递、存储 | 无法在类型层面约束 block 类型 |
| 5 | Writer 仍用自建 `Paragraph`/`Run`/`Table`（非 core model） | 历史原因，printpdf API 直接映射 | 两套类型并存增加认知负担 |
| 6 | `WriteHandlerChain` 用 `order()` 排序 | 确保钩子执行顺序确定性 | 需要手动管理 order 值冲突 |

### 5.1 Writer 统一到 core model 的路径

当前 Writer 有自建的 `Paragraph`、`Run`、`Table`、`DocImage` 类型，与 `PdfBlock` / `PdfText` 平行。未来计划：

1. 让 `PdfWriter` 内部直接消费 `PdfBlock`。
2. 移除 `easypdf-writer/src/lib.rs` 中的重复类型定义。
3. 使 `EasyPdf::read()` → 修改 → `EasyPdf::write()` 的闭环共用同一套类型。

## 6. 测试与验收

### 6.1 现有测试覆盖

| 测试 | 断言点 | 文件 |
|---|---|---|
| `test_pdf_block_variants` | 14 个变体创建和匹配 | `easypdf-core` tests |
| `test_converter_registry` | `ConverterRegistry` 注册与分发 | `easypdf-core` tests |
| `test_write_handler_chain` | 链式执行 + 优先级排序 | `easypdf-core` tests |
| `test_pdf_read_listener` | 回调触发顺序和参数 | `easypdf-reader` tests |
| `test_pdf_model_derive` | derive 宏生成代码正确性 | `easypdf-derive` tests |
| `test_error_variants` | 所有错误变体构造和匹配 | `easypdf-core` tests |

### 6.2 待补充测试

- `PdfBlock::Formula` 的创建与渲染（未来版本）。
- `PdfReadListener` 的 `has_next()` 提前终止。
- `WriteHandlerChain` 各钩子的调用顺序与参数正确性。
- `ConverterRegistry` 在并发场景下的行为。

## 7. 引用

- 架构文档：`docs/easypdf-rust-Architecture.md` 第 4 节「easypdf-core 模型设计」
- 使用指南：`docs/usage-guide.md` 第 3 节「语义模型」、第 7 节「高级特性」
- Roadmap：`docs/superpowers/version-plan.md` 0.1 Foundation、0.2 Architecture Consolidation
- 源码：`crates/easypdf-core/src/model/`、`crates/easypdf-core/src/traits.rs`
