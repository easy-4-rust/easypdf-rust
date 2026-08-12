# easypdf-writer

> PDF 写入层：创建新 PDF 文档（文本、表格、图片、形状、自定义字体），支持常量内存模式。

## 角色

`easypdf-writer` 负责创建新的 PDF 文档。它基于 `printpdf` 后端构建 PDF，支持文本、表格、图片、SVG、形状等元素的写入，并提供两种写入后端：默认的内存模式和适合大文档的页面级溢出（spill）模式。

## 核心能力

- **文本写入**（内置字体 + 自定义 TTF/TTC 字体）——支持 14 种内置字体和外部字体文件
- **表格写入**——带表头、样式、边框的表格渲染
- **图片写入**——PNG/JPEG 图片嵌入
- **SVG 写入**——矢量图形嵌入
- **形状绘制**——直线、矩形、圆形
- **元数据与书签**——文档属性与目录
- **Handler 生命周期**——`before_document` / `before_page` / `after_page` / `after_document` 钩子
- **两种写入后端**——`InMemory`（默认）和 `Spill`（大文档常量内存）

## 依赖

- `easypdf-core`: 核心类型（`PdfText`、`PdfTable`、`PdfImage`、`PdfFont`、`PdfWriteHandler`）
- `printpdf`: PDF 构建引擎
- `lopdf`: PDF 对象模型（用于后处理）
- `image`: 图片解码
- `chrono`: 时间戳

## 主要 API

### `PdfWriter`
```rust
use easypdf_writer::PdfWriter;
use easypdf_core::*;

let mut w = PdfWriter::new("My Document");
w.add_page(PageSize::A4, Orientation::Portrait)?;
w.write_text(&PdfText::new("Hello").font(PdfFont::helvetica(14.0)), 100.0, 700.0)?;
w.add_text(&PdfFont::times_roman(12.0), "Auto-positioned text")?;
w.finish(&"output.pdf")?;
```

### `PdfWriterBuilder`
```rust
use easypdf_writer::{PdfWriterBuilder, WriteBackend};

let w = PdfWriterBuilder::new("Big Report")
    .metadata(PdfMetadata::new().title("Q4 Report"))
    .backend(WriteBackend::auto(500)) // 自动选择后端
    .constant_memory(true)
    .build()?;
```

### `PdfTemplateFiller`
```rust
use easypdf_writer::PdfTemplateFiller;

let filler = PdfTemplateFiller::new("template.pdf")?;
let output = filler.fill(&model)?.save_to("filled.pdf")?;
```

## License

Apache-2.0

---

**项目主页**：https://github.com/easy-4-rust/easypdf-rust
**crates.io**：https://crates.io/crates/easypdf-writer
