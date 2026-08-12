# easypdf-markdown

> PDF 转 Markdown 层：确定性转换、语义处理器管道、表格检测、OCR 抽象、页面渲染。

## 角色

`easypdf-markdown` 将 PDF 的语义内容确定性地转换为 Markdown 字符串。它采用处理器管道（`ProcessorPipeline`）架构，内置标题检测、链接提取、阅读顺序排列等语义处理器，并集成了表格检测、OCR 文本提取和页面渲染能力。支持通过 `MarkdownProfile` 预设快速配置。

## 核心能力

- **处理器管道**（`ProcessorPipeline`）——按优先级组合多个语义增强处理器
- **Markdown 渲染**（`MarkdownRenderer`）——将 `PdfDocumentModel` 渲染为 Markdown 字符串
- **内置处理器**（`ReadingOrderProcessor`、`HeadingDetectorProcessor`、`LinkExtractorProcessor`）
- **表格检测**（`TableDetectorProcessor`）——启发式表格边界识别
- **OCR 抽象**（`OcrProcessor` + `OcrEngine` trait）——可插拔的 OCR 引擎接口
- **页面渲染**（`PdfRenderer`）——PDF 页面光栅化为图片（支持 pdfium 后端）
- **Profile 预设**（`MarkdownProfile`）——`balanced` / `fast` / `high_quality` 等配置组合

## 依赖

- `easypdf-core`: 核心类型（`PdfDocumentModel`、`PdfPageModel`）
- `easypdf-reader`: PDF 解析与文本提取
- `lopdf`: PDF 底层解析
- `image`: 图片处理

## 主要 API

### `PdfMarkdownBuilder`
```rust
use easypdf_markdown::{PdfMarkdownBuilder, MarkdownProfile};

let md = PdfMarkdownBuilder::new("document.pdf")
    .profile(MarkdownProfile::balanced())
    .build()
    .convert_to_markdown()?;
```

### `ProcessorPipeline`
```rust
use easypdf_markdown::{
    ProcessorPipeline,
    processors::{ReadingOrderProcessor, HeadingDetectorProcessor},
};

let mut pipeline = ProcessorPipeline::new();
pipeline.register(Box::new(ReadingOrderProcessor));
pipeline.register(Box::new(HeadingDetectorProcessor::new()));
```

### `TableDetectorProcessor`
```rust
use easypdf_markdown::{TableDetectorProcessor, TableDetectionConfig};

let detector = TableDetectorProcessor::new(TableDetectionConfig::default());
```

### `OcrProcessor`
```rust
use easypdf_markdown::{OcrProcessor, OcrConfig};

let ocr = OcrProcessor::new(OcrConfig::default());
// 在处理器管道中使用
pipeline.register(Box::new(ocr));
```

## Feature flags

| Feature | 说明 |
|--------|------|
| `pdfium` | 启用 pdfium 渲染后端（需系统安装 libpdfium） |
| `ocrs` | 启用 ocrs 纯 Rust OCR 引擎 |
| `llm` | 启用 LLM Vision OCR 引擎（OpenAI/Gemini/DeepSeek） |

## License

Apache-2.0

---

**项目主页**：https://github.com/easy-4-rust/easypdf-rust
**crates.io**：https://crates.io/crates/easypdf-markdown
