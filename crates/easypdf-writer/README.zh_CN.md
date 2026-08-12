# easypdf-writer

> PDF 写入层：创建新 PDF 文档（文本、表格、图片、SVG、形状、自定义字体），支持大文档常量内存溢出模式。

## 角色

`easypdf-writer` 负责 easypdf-rust 工作区中所有的 PDF 输出操作。它基于 `printpdf` 后端构建 PDF，支持文本（14 种内置字体 + 自定义 TTF/TTC）、带样式的表格、PNG/JPEG 图片、SVG 矢量图形和形状图元。提供两种写入后端：`InMemory`（默认，适合小文档）和 `Spill`（页面级临时文件，恒定内存），以及 `Auto` 模式按页面数阈值自动选择。

## 核心能力

- **文本写入**——14 种内置字体（`BuiltInFont`）+ 自定义 TTF/TTC 字体注册（`crates/easypdf-writer/src/writer.rs`）
- **表格写入**——表头、行、单元格样式、边框，通过 `PdfTable` + `TableStyle`（`crates/easypdf-core/src/content.rs`、`crates/easypdf-core/src/style.rs`）
- **图片写入**——从路径或字节嵌入 PNG/JPEG（`crates/easypdf-writer/src/writer.rs`）
- **SVG 写入**——矢量图形嵌入（`crates/easypdf-writer/src/writer.rs`）
- **形状绘制**——直线、矩形、圆形（`crates/easypdf-writer/src/writer.rs`）
- **Handler 生命周期**——`PdfWriteHandler` 钩子：`before_document` / `before_page` / `after_page` / `after_document`（`crates/easypdf-core/src/traits.rs:183`）
- **AcroForm 模板填充**——`PdfTemplateFiller` 填充已有 PDF 表单（`crates/easypdf-writer/src/template.rs`）
- **两种写入后端**——`InMemory`（默认）和 `Spill`（页面级临时文件，恒定内存），支持 `Auto` 阈值选择（`crates/easypdf-writer/src/backend.rs`）

## 依赖

### 内部依赖

| Crate | 用途 |
|-------|------|
| `easypdf-core` | 核心类型（`PdfText`、`PdfTable`、`PdfImage`、`PdfFont`、`PdfWriteHandler`、`AtomicFileOutput`） |

### 外部依赖

| Crate | 版本 | 用途 |
|-------|------|------|
| `printpdf` | 0.12.4 | PDF 创建引擎（features: png, html, svg） |
| `lopdf` | 0.44.0 | PDF 对象模型（后处理与模板填充） |
| `image` | 0.25.9 | 图片解码 |
| `serde` / `serde_json` | 1.x | 模板数据序列化 |
| `chrono` | 0.4.45 | 时间戳 |

## 主要 API

### PdfWriter

```rust
use easypdf_writer::PdfWriter;
use easypdf_core::*;

let mut w = PdfWriter::new("我的文档");
w.add_page(PageSize::A4, Orientation::Portrait)?;
w.write_text("你好，世界！", 100.0, 700.0)?;
w.add_text(&PdfFont::times_roman(12.0), "自动定位文本")?;
w.write_image_from_path("logo.png", 50.0, 50.0, 200.0, 100.0)?;
w.draw_line(50.0, 680.0, 545.0, 680.0, 1.0)?;
w.register_font_from_path("custom.ttf")?;
w.finish("output.pdf")?;
```

### PdfWriterBuilder

```rust
use easypdf_writer::{PdfWriterBuilder, WriteBackend};

let w = PdfWriterBuilder::new("大型报告")
    .metadata(PdfMetadata::new().title("Q4 报告"))
    .backend(WriteBackend::auto(500)) // 500 页时自动切换后端
    .constant_memory(true)
    .build()?;
```

### PdfTemplateFiller

```rust
use easypdf_writer::PdfTemplateFiller;

let filler = PdfTemplateFiller::new("template.pdf")?;
let output = filler.fill(&model)?.save_to("filled.pdf")?;
```

### WriteBackend

```rust
use easypdf_writer::WriteBackend;

// 默认：内存模式（快速，适合小文档）
let backend = WriteBackend::InMemory;

// 溢出模式：页面级临时文件，恒定内存
let backend = WriteBackend::Spill;

// 自动模式：按页面阈值选择
let backend = WriteBackend::auto(500);
```

## License

Apache-2.0

---

**项目主页**：https://github.com/easy-4-rust/easypdf-rust
**crates.io**：https://crates.io/crates/easypdf-writer
**docs.rs**：https://docs.rs/easypdf-writer
