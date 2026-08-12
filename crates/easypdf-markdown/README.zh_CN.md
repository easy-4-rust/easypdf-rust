# easypdf-markdown

> PDF 转 Markdown 层：确定性转换管道、语义处理器链、表格检测、OCR 抽象、页面渲染。

## 角色

`easypdf-markdown` 将 PDF 内容确定性地转换为 Markdown 字符串。它采用 `ProcessorPipeline` 架构，语义处理器（阅读顺序、标题检测、链接提取、表格检测、OCR fallback）按优先级依次运行，在渲染前丰富文档模型。管道流程：`PdfInput -> PdfReader -> PdfDocumentModel -> ProcessorPipeline -> MarkdownRenderer -> String`。

## 核心能力

- **处理器管道**（`ProcessorPipeline`）——按优先级组合多个语义增强处理器（`crates/easypdf-markdown/src/processor_pipeline.rs`）
- **Markdown 渲染**（`MarkdownRenderer`）——将 `PdfDocumentModel` 渲染为 Markdown 字符串（`crates/easypdf-markdown/src/markdown_renderer.rs`）
- **内置处理器**——`ReadingOrderProcessor`（阅读顺序）、`HeadingDetectorProcessor`（标题检测）、`LinkExtractorProcessor`（链接提取）（`crates/easypdf-markdown/src/processors/`）
- **表格检测**（`TableDetectorProcessor`）——启发式表格边界识别，可配置 `TableDetectionConfig`（`crates/easypdf-markdown/src/table/`）
- **OCR 抽象**（`OcrProcessor` + `OcrEngine` trait）——可插拔的 OCR 引擎接口，用于 fallback 文本提取（`crates/easypdf-markdown/src/ocr/`）
- **页面渲染**（`PdfRenderer` trait）——将 PDF 页面光栅化为图片；默认 text 后端，可选 PDFium 后端（`crates/easypdf-markdown/src/render/`）
- **Profile 预设**（`MarkdownProfile`）——`balanced` / `fast` / `high_quality` 配置预设，通过 `MarkdownProfileBuilder` 自定义（`crates/easypdf-markdown/src/markdown_profile.rs`）
- **导出构建器**——`PdfMarkdownBuilder`（内存转换）和 `PdfMarkdownExportBuilder`（文件导出），附 `MarkdownExportReport`（`crates/easypdf-markdown/src/pdf_markdown_builder.rs`）

## 依赖

### 内部依赖

| Crate | 用途 |
|-------|------|
| `easypdf-core` | 核心类型（`PdfDocumentModel`、`PdfPageModel`、`PdfBlock`） |
| `easypdf-reader` | PDF 解析与文本提取 |

### 外部依赖

| Crate | 版本 | 用途 |
|-------|------|------|
| `lopdf` | 0.44.0 | PDF 对象模型 |
| `image` | 0.25.9 | 图片处理 |
| `serde` / `serde_json` | 1.x | 序列化 |

## 主要 API

### PdfMarkdownBuilder（内存转换）

```rust
use easypdf_markdown::{PdfMarkdownBuilder, MarkdownProfile};

let result = PdfMarkdownBuilder::new("document.pdf")
    .profile(MarkdownProfile::balanced())
    .build()
    .convert_to_markdown()?;

println!("{}", result.markdown);
```

### PdfMarkdownExportBuilder（文件导出）

```rust
use easypdf_markdown::PdfMarkdownExportBuilder;

let report = PdfMarkdownExportBuilder::new("document.pdf")
    .output_path("output.md")
    .build()
    .export()?;
```

### ProcessorPipeline

```rust
use easypdf_markdown::{
    ProcessorPipeline,
    processors::{ReadingOrderProcessor, HeadingDetectorProcessor, LinkExtractorProcessor},
};

let mut pipeline = ProcessorPipeline::new();
pipeline.register(Box::new(ReadingOrderProcessor));
pipeline.register(Box::new(HeadingDetectorProcessor::new()));
pipeline.register(Box::new(LinkExtractorProcessor::new()));
```

### TableDetectorProcessor

```rust
use easypdf_markdown::{TableDetectorProcessor, TableDetectionConfig};

let detector = TableDetectorProcessor::new(TableDetectionConfig::default());
pipeline.register(Box::new(detector));
```

### OcrProcessor

```rust
use easypdf_markdown::{OcrProcessor, OcrConfig, OcrTrigger};

let config = OcrConfig::default().trigger(OcrTrigger::LowConfidence);
let ocr = OcrProcessor::new(config);
pipeline.register(Box::new(ocr));
```

### 页面渲染

```rust
use easypdf_markdown::render::{render_page_to_png, RenderConfig};

let image = render_page_to_png("document.pdf", 0, &RenderConfig::default())?;
```

## Feature Flags

| Feature | 依赖 | 说明 |
|---------|------|------|
| `pdfium` | `pdfium-render: 0.8` | 高质量 PDFium 渲染后端（需系统安装 `libpdfium`） |
| `ocrs` | `ocrs: 0.9` | 纯 Rust OCR 引擎 |
| `llm` | `rig-core: 0.8`、`base64`、`tokio` | LLM Vision OCR（OpenAI / Gemini / DeepSeek） |

## License

Apache-2.0

---

**项目主页**：https://github.com/easy-4-rust/easypdf-rust
**crates.io**：https://crates.io/crates/easypdf-markdown
**docs.rs**：https://docs.rs/easypdf-markdown
