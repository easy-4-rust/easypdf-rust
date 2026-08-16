# easypdf-rust Usage Guide 使用指南

> 完整的 API 使用指南，覆盖创建、读取、操作、模板填充四大场景。  
> Comprehensive usage guide covering all four major scenarios: Create, Read, Manipulate, Fill.

---

## Table of Contents

1. [Installation 安装](#1-installation-安装)
2. [Quick Start 快速开始](#2-quick-start-快速开始)
3. [Creating PDFs 创建 PDF](#3-creating-pdfs-创建-pdf)
   - [3.11 选择写入引擎 / Choosing a Write Engine](#311-选择写入引擎--choosing-a-write-engine)
4. [Reading PDFs 读取 PDF](#4-reading-pdfs-读取-pdf)
5. [Manipulating PDFs 操作 PDF](#5-manipulating-pdfs-操作-pdf)
6. [Template & Form Filling 模板与表单填充](#6-template--form-filling-模板与表单填充)
7. [Encryption & Signing 加密与签名](#7-encryption--signing-加密与签名)
8. [HTML & Markdown Conversion 转换](#8-html--markdown-conversion-转换)
9. [Layout Engine 布局引擎](#9-layout-engine-布局引擎)
10. [Derive Macro 派生宏](#10-derive-macro-派生宏)
11. [Advanced Topics 高级主题](#11-advanced-topics-高级主题)
12. [API Reference API 参考](#12-api-reference-api-参考)

---

## 1. Installation 安装

Add to your `Cargo.toml`:

```toml
[dependencies]
easypdf = "0.1"

# Optional: enable HTML/Markdown conversion (requires Chromium)
# easypdf = { version = "0.1", features = ["html"] }
```

Import in your code:

```rust
use easypdf::prelude::*;
```

---

## 2. Quick Start 快速开始

### 2.1 创建 PDF 一行式

```rust
EasyPdf::create("hello.pdf")
    .title("My Document")
    .page(PageSize::A4)
    .add_text("Hello, World!")
        .font(PdfFont::helvetica(16.0).bold())
        .position(100.0, 700.0)
    .do_write()?;
```

### 2.2 读取 PDF

```rust
let text = EasyPdf::read("input.pdf").extract_text()?;
println!("{text}");
```

### 2.3 合并 PDF

```rust
EasyPdf::merge(&["a.pdf", "b.pdf"], "merged.pdf")?;
```

### 2.4 拆分 PDF

```rust
let files = EasyPdf::split("big.pdf")
    .every_n_pages(1)
    .save_to_dir("./pages/")?;
// → ./pages/page_001.pdf, ./pages/page_002.pdf, ...
```

### 2.5 填充表单

```rust
EasyPdf::fill_form("template.pdf", &data)
    .field("name", "Alice")
    .save("filled.pdf")?;
```

---

## 3. Creating PDFs 创建 PDF

### 3.1 简单创建

`EasyPdf::create()` 返回 `PdfCreateBuilder`，支持链式调用：

```rust
use easypdf::prelude::*;

EasyPdf::create("output.pdf")
    .title("Annual Report")            // 文档标题
    .page(PageSize::A4)                // 页面尺寸
    .metadata(PdfMetadata::new()       // 元数据
        .author("Alice")
        .subject("Finance"))
    .add_text("Q4 Results")            // 添加文本
        .font(PdfFont::helvetica(18.0).bold())
        .position(72.0, 750.0)
    .do_write()?;
```

### 3.2 Auto-positioned Text (hutool OfdWriter pattern) 

使用 `add_text` 自动定位，无需手动指定坐标：

```rust
let mut writer = EasyPdf::create("auto-layout.pdf").build()?;
writer.add_page(PageSize::A4, Orientation::Portrait)?;

// 自动堆叠文本，每行自动向下移动
writer
    .add_text(&PdfFont::helvetica(18.0).bold(), "Chapter 1")?
    .add_text(&PdfFont::times_roman(12.0), "This is the first paragraph.")?
    .add_text(&PdfFont::times_roman(12.0), "This is another line.")?
    .add_text_colored(&PdfFont::helvetica(14.0), &PdfColor::blue(), "Blue heading!")?;

// 添加图片（也自动推进光标）
writer.add_image_from_path("logo.png", 200.0, 100.0)?;

writer.finish("auto-layout.pdf")?;
```

### 3.3 Stream-based Output (hutool OfdWriter pattern)

```rust
let mut buf = Vec::new();
let mut writer = PdfWriter::new_from_writer(buf);
writer.add_page(PageSize::A4, Orientation::Portrait)?;
writer.add_text(&PdfFont::helvetica(12.0), "Hello stream!")?;
writer.flush()?; // writes PDF bytes to buffer
// Now `buf` contains the PDF data
```

### 3.4 Multi-page Writer

使用 `PdfWriter` 进行多页写入：

```rust
let mut writer = EasyPdf::create("multi-page.pdf")
    .metadata(PdfMetadata::new().title("Report"))
    .build()?;

// 第 1 页
writer.add_page(PageSize::A4, Orientation::Portrait)?;
writer.write_text(
    &PdfText::new("Chapter 1").font(PdfFont::helvetica(18.0).bold()),
    72.0, 750.0,
)?;
writer.write_text(
    &PdfText::new("Once upon a time...").font(PdfFont::times_roman(12.0)),
    72.0, 700.0,
)?;

// 第 2 页
writer.add_page(PageSize::A4, Orientation::Portrait)?;
writer.write_text(
    &PdfText::new("Chapter 2").font(PdfFont::helvetica(18.0).bold()),
    72.0, 750.0,
)?;

writer.finish("multi-page.pdf")?;
```

### 3.3 字体与样式

14 种内置字体（无需嵌入）：

```rust
// Serif fonts
PdfFont::times_roman(14.0)           // 常规
PdfFont::times_roman(14.0).bold()    // 粗体
PdfFont::times_roman(14.0).italic()  // 斜体

// Sans-serif fonts
PdfFont::helvetica(12.0)
PdfFont::helvetica(12.0).bold()
PdfFont::helvetica(12.0).bold().italic()

// Monospace fonts
PdfFont::courier(10.0)
PdfFont::courier(10.0).bold()

// Symbol fonts
PdfFont {
    family: FontFamily::BuiltIn(BuiltInFont::Symbol),
    size: 14.0,
    style: FontStyle { bold: false, italic: false },
}
```

自定义 TTF/OTC 字体（需要系统字体文件）：

```rust
let mut writer = EasyPdf::create("output.pdf").build()?;
writer.register_font_from_path("/System/Library/Fonts/Helvetica.ttc")?;
writer.write_text_with_custom_font("Hello", "/System/Library/Fonts/Helvetica.ttc", 14.0, 72.0, 700.0)?;
```

### 3.4 颜色

```rust
use easypdf::PdfColor;

// 预定义颜色
PdfColor::black()     // RGB(0, 0, 0)
PdfColor::white()     // RGB(1, 1, 1)
PdfColor::red()       // RGB(1, 0, 0)
PdfColor::green()     // RGB(0, 1, 0)
PdfColor::blue()      // RGB(0, 0, 1)
PdfColor::gray()      // 50% gray

// 自定义颜色
PdfColor::rgb_u8(128, 64, 255)    // RGB from 0-255
PdfColor::Rgb(0.5, 0.25, 1.0)    // RGB from 0.0-1.0
PdfColor::Cmyk(0.1, 0.2, 0.0, 0.0) // CMYK
PdfColor::Gray(0.75)               // Grayscale

// 带颜色的文本
let text = PdfText::new("Colored!")
    .font(PdfFont::helvetica(14.0))
    .color(PdfColor::rgb_u8(255, 0, 128));
writer.write_text(&text, 72.0, 700.0)?;
```

### 3.5 图片

```rust
// 从文件创建图片
let img = PdfImage::from_path("logo.png")?;

// 从字节创建
let img = PdfImage::from_bytes(bytes);

// 写入页面（自动检测格式）
writer.add_page(PageSize::A4, Orientation::Portrait)?;
writer.write_image(&img, 100.0, 500.0, 200.0, 100.0)?; // (x, y, w, h) in points

// 使用自然尺寸（w=0, h=0 → 72 DPI）
writer.write_image(&img, 50.0, 600.0, 0.0, 0.0)?;
```

### 3.6 矢量图形

```rust
writer.add_page(PageSize::A4, Orientation::Portrait)?;

// 直线
writer.draw_line(0.0, 0.0, 595.0, 0.0, 1.0);  // (x1,y1,x2,y2,line_width)

// 矩形边框
writer.draw_rect_stroke(50.0, 600.0, 200.0, 100.0, 1.5);
// (x, y, width, height, line_width)

// 圆形（4 段贝塞尔近似，误差 < 0.027%）
writer.draw_circle(300.0, 400.0, 100.0, 1.0);
// (center_x, center_y, radius, line_width)
```

### 3.7 SVG

```rust
let svg_data = r#"
<svg xmlns="http://www.w3.org/2000/svg" width="200" height="200">
  <rect width="200" height="200" fill="#ff0000"/>
  <circle cx="100" cy="100" r="80" fill="#0000ff"/>
</svg>"#;

writer.add_page(PageSize::A4, Orientation::Portrait)?;
writer.write_svg(svg_data, 100.0, 500.0, 200.0, 200.0)?;
// (svg_string, x, y, width, height)
```

### 3.8 表格

```rust
let table = PdfTable::new(vec!["Name".into(), "Age".into(), "City".into()])
    .row(vec!["Alice".into(), "30".into(), "NYC".into()])
    .row(vec!["Bob".into(), "25".into(), "SF".into()])
    .row(vec!["Charlie".into(), "35".into(), "LA".into()]);

writer.add_page(PageSize::A4, Orientation::Portrait)?;
write_table(
    &mut writer,
    &table,
    50.0,          // x position
    700.0,         // y position (top)
    &[150.0, 80.0, 150.0],  // column widths
    25.0,          // row height
    &PdfFont::helvetica(10.0),
)?;
```

### 3.9 页眉与页脚

```rust
let handler = PageNumberHandler::new()
    .font(PdfFont::helvetica(10.0))
    .offset_y(30.0);

let writer = EasyPdf::create("numbered.pdf")
    .register_handler(Box::new(handler))
    .build()?;
```

### 3.10 自定义写入钩子

实现 `PdfWriteHandler` trait：

```rust
struct MyHandler;
impl PdfWriteHandler for MyHandler {
    fn before_page(&mut self, page: usize) -> easypdf::Result<()> {
        println!("Starting page {page}");
        Ok(())
    }
    fn after_document(&mut self) -> easypdf::Result<()> {
        println!("Document complete!");
        Ok(())
    }
}

let writer = EasyPdf::create("out.pdf")
    .register_handler(Box::new(MyHandler))
    .build()?;
```

### 3.11 选择写入引擎 / Choosing a Write Engine

easypdf-rust 支持两个写入引擎，通过 `WriteEngineKind` 在构建时选择：
easypdf-rust supports two write engines, selectable at build time via `WriteEngineKind`:

| 引擎 / Engine | Feature | 特点 / Characteristics |
|---|---|---|
| `Printpdf`（默认） | 内置 | Base14 字体、SVG、成熟稳定 |
| `Krilla` | `writer-krilla` | 字体子集化、CJK 体积优化 |

**使用默认引擎（printpdf）：**

```rust
use easypdf::prelude::*;

// 默认引擎，无需指定
let writer = PdfWriterBuilder::new("Default Engine")
    .build()?;
```

**显式选择引擎：**

```rust
use easypdf::prelude::*;

// 使用 printpdf 引擎
let writer = PdfWriterBuilder::new("Printpdf Engine")
    .engine(WriteEngineKind::Printpdf)
    .build()?;

// 使用 krilla 引擎（需要 writer-krilla feature）
let writer = PdfWriterBuilder::new("Krilla Engine")
    .engine(WriteEngineKind::Krilla)
    .build()?;
```

**krilla 引擎注意事项：**

- krilla 不支持 PDF 标准 14 内置字体，需通过 `register_font_from_bytes` 提供真实字体文件。
- krilla 不支持 SVG 嵌入。
- krilla 的字体子集化对 CJK 大字体文件效果显著（可将 10MB+ 字体缩减至数百 KB）。

```toml
# 启用 krilla 引擎
[dependencies]
easypdf = { version = "0.1.1", features = ["writer-krilla"] }
```

---

## 4. Reading PDFs 读取 PDF

### 4.1 基本读取

```rust
// 提取全部文本
let text = EasyPdf::read("input.pdf").extract_text()?;
println!("{text}");

// 限制页码范围
let pages_1_3 = EasyPdf::read("input.pdf")
    .pages(0..3)  // 0-based
    .extract_text()?;

// 获取页数
let count = EasyPdf::read("input.pdf").page_count()?;
println!("{count} pages");

// 提取元数据
let meta = EasyPdf::read("input.pdf").metadata()?;
println!("Title: {:?}", meta.title);
println!("Author: {:?}", meta.author);
```

### 4.2 事件驱动读取

实现 `PdfReadListener` trait：

```rust
struct PageCollector {
    pages: Vec<String>,
}

impl PdfReadListener for PageCollector {
    fn on_page_start(&mut self, page_number: usize) -> easypdf::Result<()> {
        println!("Reading page {page_number}...");
        Ok(())
    }

    fn on_text(&mut self, _page_number: usize, text: &str) -> easypdf::Result<()> {
        self.pages.push(text.to_string());
        Ok(())
    }

    fn on_document_end(&mut self) -> easypdf::Result<()> {
        println!("Done! Read {} pages", self.pages.len());
        Ok(())
    }
}

let mut collector = PageCollector { pages: vec![] };
PdfReader::open("input.pdf")?
    .read_with_listener(&mut collector)?;
```

### 4.3 使用 PdfReader 底层 API

```rust
let reader = PdfReader::open("input.pdf")?;

// 提取文本
let text = reader.extract_text()?;

// 提取元数据
let meta = reader.extract_metadata()?;

// 页数
let count = reader.page_count()?;

// 限定页面范围
let partial = reader.pages(0..5).extract_text()?;
```

---

## 5. Manipulating PDFs 操作 PDF

### 5.1 合并 PDF

```rust
EasyPdf::merge(
    &["cover.pdf", "chapter1.pdf", "chapter2.pdf", "appendix.pdf"],
    "book.pdf",
)?;
```

### 5.2 拆分 PDF

```rust
// 每页一个文件
EasyPdf::split("big.pdf")
    .every_n_pages(1)
    .save_to_dir("./pages/")?;
// → page_001.pdf, page_002.pdf, ...

// 每 5 页一组
EasyPdf::split("big.pdf")
    .every_n_pages(5)
    .save_to_dir("./chunks/")?;
```

### 5.3 旋转页面

```rust
// 所有页面旋转 90°
EasyPdf::manipulate("input.pdf")
    .rotate_all(Rotation::Clockwise90)
    .save("rotated.pdf")?;

// 只旋转特定页面（1-based）
EasyPdf::manipulate("input.pdf")
    .rotate_page(1, Rotation::Clockwise90)
    .rotate_page(3, Rotation::Clockwise180)
    .save("partial-rotated.pdf")?;
```

### 5.4 重排页面

```rust
// 将第 3 页移到最前面（0-based 索引）
EasyPdf::manipulate("input.pdf")
    .reorder_pages(&[2, 0, 1, 3])
    .save("reordered.pdf")?;
```

### 5.5 组合操作

```rust
EasyPdf::manipulate("input.pdf")
    .rotate_all(Rotation::Clockwise90)
    .reorder_pages(&[2, 0, 1])
    .save("processed.pdf")?;
```

### 5.6 添加水印

```rust
let mut manipulator = PdfManipulator::open("input.pdf")?;
manipulator.add_text_watermark("CONFIDENTIAL", 48.0, 0.3)?;
manipulator.save("watermarked.pdf")?;
```

### 5.7 添加图层（OCG）

```rust
let mut manipulator = PdfManipulator::open("input.pdf")?;
let layer_id = manipulator.add_layer("Background")?;
manipulator.save("layered.pdf")?;
```

### 5.8 PDF/A 校验

```rust
let manipulator = PdfManipulator::open("input.pdf")?;
let issues = manipulator.validate_pdfa();
if issues.is_empty() {
    println!("PDF/A compliant!");
} else {
    for issue in &issues {
        println!("  - {issue}");
    }
}
```

---

## 6. Template & Form Filling 模板与表单填充

### 6.1 基本表单填充

```rust
// 使用 PdfModel derive 宏
#[derive(PdfModel)]
struct Invoice {
    #[pdf(field = "customer_name")]
    customer: String,
    #[pdf(field = "total")]
    total: String,
}

EasyPdf::fill_form("template.pdf", &Invoice {
    customer: "Alice".into(),
    total: "$1,234.00".into(),
})
    .save("filled.pdf")?;
```

### 6.2 程序化填充

不依赖 derive 宏：

```rust
EasyPdf::fill_form("template.pdf", &data)
    .field("name", "Bob")
    .field("email", "bob@example.com")
    .field("date", "2026-07-21")
    .fields([
        ("phone", "555-0100"),
        ("address", "123 Main St"),
    ])
    .save("filled.pdf")?;
```

### 6.3 底层 API

```rust
let mut filler = PdfTemplateFiller::open("template.pdf")?;

// 填充字段
filler.fill_field("name", "Alice")?;
filler.fill_fields([("email", "alice@example.com")])?;

// 获取页数
println!("{} pages", filler.page_count());

// 保存
filler.save("output.pdf")?;

// 获取内部 lopdf Document
let doc = filler.into_inner();
```

---

## 7. Encryption & Signing 加密与签名

### 7.1 加密 PDF

```rust
EasyPdf::encrypt("open.pdf", "encrypted.pdf", "password123")?;
```

### 7.2 数字签名（占位符）

```rust
EasyPdf::sign("unsigned.pdf", "signed.pdf", "Approved by Alice")?;
```

> **注意**: 完整的 PKCS#7/RSA 签名需要 `crypto` feature，尚未实现。

---

## 8. HTML & Markdown Conversion 转换

> 需要安装 Chromium 并启用 `html` feature。

### 8.1 HTML → PDF

```rust
// Cargo.toml: easypdf = { version = "0.1", features = ["html"] }

let html = r#"
<html>
  <body>
    <h1>My Document</h1>
    <p>This is a paragraph with <b>bold</b> text.</p>
  </body>
</html>"#;

EasyPdf::from_html(html)?
    .title("HTML Document")
    .save("from-html.pdf")?;
```

### 8.2 Markdown → PDF

内置简单 Markdown 转换器（标题、粗体、斜体、列表、引用）：

```rust
let md = r#"
# Chapter 1

This is **bold** and *italic* text.

- Item one
- Item two

> A quote block
"#;

EasyPdf::from_markdown(md)?
    .title("Markdown Document")
    .save("from-markdown.pdf")?;
```

---

## 9. Layout Engine 布局引擎

使用 `FlowLayout` 自动计算元素位置：

```rust
use easypdf::{FlowLayout, LayoutDirection};

let writer = PdfWriter::new("test");
let mut layout = FlowLayout::vertical(writer, PageSize::A4)
    .margins(50.0)        // 页边距
    .spacing(10.0);       // 元素间距

// 自动堆叠文本
layout.add_text("Title", &PdfFont::helvetica(18.0).bold(), 25.0)?;
layout.add_text("Paragraph 1...", &PdfFont::times_roman(12.0), 15.0)?;
layout.add_text("Paragraph 2...", &PdfFont::times_roman(12.0), 15.0)?;

// 当剩余空间不足时，自动换页
while layout.remaining_space() > 30.0 {
    layout.add_text("Line", &PdfFont::helvetica(10.0), 15.0)?;
}
layout.new_page()?;
layout.add_text("New page!", &PdfFont::helvetica(14.0), 20.0)?;

// 保存
layout.finish("layout-output.pdf")?;
```

---

## 10. Derive Macro 派生宏

### 10.1 基本用法

```rust
use easypdf::prelude::*;

#[derive(PdfModel)]
#[pdf(page = easypdf_core::PageSize::A4, orientation = easypdf_core::Orientation::Portrait)]
struct Report {
    #[pdf(text, position = (100, 700), size = 18)]
    title: String,

    #[pdf(text, position = (100, 650), font = easypdf_core::PdfFont::times_roman(12.0))]
    author: String,

    #[pdf(ignore)]
    _internal_id: u64,
}

let report = Report {
    title: "Annual Report".into(),
    author: "Alice".into(),
    _internal_id: 42,
};

// 渲染为元素列表
let elements: Vec<RenderedElement> = PdfModel::render(&report)?;
println!("{} elements", elements.len()); // → 2

// 获取元数据
let meta: PdfModelMetadata = PdfModel::metadata(&report);
println!("Page: {:?}", meta.page_size); // → A4
```

### 10.2 支持的属性

| 属性 | 位置 | 说明 |
|:---|:---|:---|
| `#[pdf(page = ...)]` | Struct | 页面尺寸 |
| `#[pdf(orientation = ...)]` | Struct | 页面方向 |
| `#[pdf(margins = ...)]` | Struct | 页边距 |
| `#[pdf(text, position = (x, y))]` | Field | 文本元素 |
| `#[pdf(table, position = (x, y))]` | Field | 表格元素 |
| `#[pdf(image, position = (x, y))]` | Field | 图片元素 |
| `#[pdf(font = ...)]` | Field | 字体 |
| `#[pdf(size = n)]` | Field | Helvetica 字体大小 |
| `#[pdf(ignore)]` | Field | 跳过字段 |

---

## 11. Advanced Topics 高级主题

### 11.1 自定义类型转换器

```rust
struct CurrencyConverter;
impl PdfConverter<f64> for CurrencyConverter {
    fn to_pdf_string(&self, value: &f64) -> easypdf::Result<String> {
        Ok(format!("${:.2}", value))
    }
    fn from_pdf_string(&self, s: &str) -> easypdf::Result<f64> {
        s.trim_start_matches('$')
            .replace(',', "")
            .parse()
            .map_err(|e| PdfError::Other(e.to_string()))
    }
}
```

### 11.2 引擎能力查询

```rust
struct LopdfEngine;
impl PdfEngine for LopdfEngine {
    fn name(&self) -> &str { "lopdf" }
    fn capabilities(&self) -> EngineCapabilities {
        EngineCapabilities::lopdf()
    }
}

let caps = EngineCapabilities::lopdf();
println!("Read: {}, Manipulate: {}", caps.read, caps.manipulate);
println!("Create: {}, Encrypt: {}", caps.create, caps.encrypt);
```

### 11.3 XMP 元数据生成

```rust
let xmp = PdfMetadata::new()
    .title("My Document")
    .author("Alice")
    .subject("Finance")
    .keywords("report, 2026, annual")
    .to_xmp();

println!("{xmp}");
// 输出 PDF/A 兼容的 XMP XML
```

### 11.4 错误处理

```rust
use easypdf::PdfError;

match EasyPdf::read("input.pdf").extract_text() {
    Ok(text) => println!("{text}"),
    Err(PdfError::Io(e)) => eprintln!("File error: {e}"),
    Err(PdfError::Parse(msg)) => eprintln!("Invalid PDF: {msg}"),
    Err(PdfError::Encryption(msg)) => eprintln!("Encrypted: {msg}"),
    Err(e) => eprintln!("Other error: {e}"),
}
```

### 11.5 预导入模块

```rust
use easypdf::prelude::*;
// 包含: EasyPdf, PageSize, PdfFont, PdfColor, PdfText, PdfModel, ...
```

---

## 12. API Reference API 参考

### EasyPdf 入口

| Method | Signature | Returns |
|:---|---|:---|
| `create` | `(path) -> PdfCreateBuilder` | 创建构建器 |
| `read` | `(path) -> PdfReadBuilder` | 读取构建器 |
| `merge` | `(inputs, output) -> Result<()>` | 合并 PDF |
| `split` | `(path) -> PdfSplitBuilder` | 拆分构建器 |
| `manipulate` | `(path) -> PdfManipulateBuilder` | 操作构建器 |
| `fill_form` | `(template, data) -> PdfFillBuilder` | 填充构建器 |
| `encrypt` | `(in, out, pwd) -> Result<()>` | 加密 |
| `sign` | `(in, out, reason) -> Result<()>` | 签名 |
| `from_html` | `(html) -> Result<HtmlToPdfBuilder>` | HTML→PDF |
| `from_markdown` | `(md) -> Result<HtmlToPdfBuilder>` | MD→PDF |

### PdfCreateBuilder

| Method | Description |
|:---|:---|
| `title(s)` | 设置文档标题 |
| `page(size)` | 设置页面尺寸 |
| `metadata(m)` | 设置元数据 |
| `register_handler(h)` | 注册写入钩子 |
| `add_text(s)` | 添加文本 → PdfTextBuilder |
| `build()` | 构建 PdfWriter |
| `do_write()` | 构建+写页面+保存 |

### PdfTextBuilder

| Method | Description |
|:---|:---|
| `font(f)` | 设置字体 |
| `position(x, y)` | 设置位置 → PdfPositionedTextBuilder |
| `do_write()` | 默认位置写文本+保存 |

### PdfReadBuilder

| Method | Returns |
|:---|:---|
| `pages(range)` | 限制页码范围 |
| `extract_text()` | `Result<String>` |
| `metadata()` | `Result<PdfMetadata>` |
| `page_count()` | `Result<usize>` |

### PdfManipulateBuilder

| Method | Description |
|:---|:---|
| `rotate_page(n, r)` | 旋转指定页 |
| `rotate_all(r)` | 旋转所有页 |
| `reorder_pages(order)` | 重排页面 |
| `save(path)` | 保存 |

### PdfFillBuilder

| Method | Description |
|:---|:---|
| `field(name, value)` | 添加字段 |
| `fields(iter)` | 批量添加字段 |
| `save(path)` | 保存 |

### FlowLayout

| Method | Description |
|:---|:---|
| `vertical(writer, size)` | 创建垂直布局 |
| `margins(m)` | 设置边距 |
| `spacing(s)` | 设置间距 |
| `add_text(s, font, h)` | 添加文本+自动换行 |
| `remaining_space()` | 剩余空间 |
| `new_page()` | 新页面 |
| `finish(path)` | 保存 |
