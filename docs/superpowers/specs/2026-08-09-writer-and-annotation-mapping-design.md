# easypdf-writer 写入引擎与注解映射设计

- **日期**：2026-08-09
- **作者**：ZCode Agent（协同设计）
- **状态**：已实现（v0.1.0 发布）
- **依赖**：easypdf-writer 现有 `writer.rs`、`backend.rs`、`builder/`、`image.rs`、`shape.rs`、`template/`

## 1. 目标与范围

为 easypdf-rust 实现**完整的 PDF 写入引擎**，支持文本/表格/图片/矢量图形/页眉页脚/水印/自定义字体/多页写入，同时提供 WriteBackend 选择（InMemory/Spill/Auto）和 WriteHandlerChain 回调机制。

**核心需求**：

1. `PdfWriter` 基于 printpdf 实现 PDF 创建和写入。
2. `WriteBackend` 枚举支持 InMemory / Spill / Auto 三种模式。
3. `WriteHandlerChain` 提供优先级排序的写入生命周期钩子。
4. `PdfWriterBuilder` 提供链式配置 API。
5. 支持 14 种内置字体 + 自定义 TTF/OTF 字体注册。
6. 支持表格渲染（`write_table`）、图片插入（`write_image`）、矢量图形（`draw_line/rect/circle`）。
7. 支持页眉页脚（`PageNumberHandler` / `TextHeaderHandler` / `TextFooterHandler`）。
8. 支持文本水印和图片水印。
9. 支持 AcroForm 模板填充（`fill_form`）。
10. 原子文件输出（temp file + rename）。

**非目标**：

- 不实现 PDF 页面渲染（仅写入）。
- 不实现 PDF 加密/签名（由 easypdf-core::crypto 承担）。
- 不实现 PDF/A 校验（未来版本）。
- 不支持 PDF 增量保存（修改 PDF 而不重写整个文件）。

## 2. 总体架构

```
┌─────────────────────────────────────────────────────┐
│                  easypdf-writer                      │
│                                                     │
│  ┌───────────────────────────────────────────────┐  │
│  │  PdfWriterBuilder                             │  │
│  │  ├── backend(WriteBackend)                    │  │
│  │  ├── handler(handler: impl PdfWriteHandler)   │  │
│  │  └── build() → PdfWriter                      │  │
│  └───────────────────────────────────────────────┘  │
│                                                     │
│  ┌───────────────────────────────────────────────┐  │
│  │  PdfWriter                                    │  │
│  │  ├── write_text(text, font, size, x, y)       │  │
│  │  ├── write_table(table)                       │  │
│  │  ├── write_image(img, x, y, w, h)             │  │
│  │  ├── draw_line / draw_rect / draw_circle      │  │
│  │  ├── register_font_from_path / _bytes         │  │
│  │  ├── add_page() / finish()                    │  │
│  │  └── current_page_number()                    │  │
│  └───────────────────────────────────────────────┘  │
│                                                     │
│  ┌───────────────────────────────────────────────┐  │
│  │  WriteBackend                                 │  │
│  │  ├── InMemory (默认，全部在内存)              │  │
│  │  ├── Spill { threshold, temp_dir }            │  │
│  │  │   (页面级临时文件，常量内存)               │  │
│  │  └── Auto(pages) → 自动选择                   │  │
│  └───────────────────────────────────────────────┘  │
│                                                     │
│  ┌───────────────────────────────────────────────┐  │
│  │  WriteHandlerChain                            │  │
│  │  ├── PageNumberHandler (页码注入)             │  │
│  │  ├── TextHeaderHandler (页眉文本)             │  │
│  │  ├── TextFooterHandler (页脚文本)             │  │
│  │  └── 用户自定义 handler                       │  │
│  └───────────────────────────────────────────────┘  │
│                                                     │
│  ┌───────────────────────────────────────────────┐  │
│  │  内容类型                                     │  │
│  │  ├── image.rs    write_image()                │  │
│  │  ├── shape.rs    draw_line/rect/circle        │  │
│  │  └── template/   fill_form() AcroForm 填充    │  │
│  └───────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────┘
```

## 3. 模块职责划分

### 3.1 `writer.rs` — 核心写入器

| 方法 | 职责 |
|---|---|
| `write_text(text, font, size, x, y)` | 写入文本到指定位置 |
| `write_text_with_custom_font(text, font_key, size, x, y)` | 用自定义字体写入文本 |
| `register_font_from_path(path)` | 从文件注册 TTF/OTF 字体 |
| `register_font_from_bytes(key, font_data)` | 从内存注册字体 |
| `set_metadata(meta)` | 设置文档元数据 |
| `add_page()` | 添加新页面 |
| `finish()` | 完成写入，输出文件 |
| `current_page_number()` | 获取当前页码 |

**多页写入**：
- 维护 `Vec<PdfPage>` 累积所有页面
- `add_page()` 将当前 ops 转为 PdfPage 推入 `pages`
- `finish()` 调用 `doc.with_pages(pages)`

### 3.2 `backend.rs` — WriteBackend 选择

| 模式 | 内存模型 | 适用场景 |
|---|---|---|
| `InMemory` | 全部在内存 | 默认，小文档 |
| `Spill { threshold, temp_dir }` | 页面级临时文件 | 大文档，常量内存 |
| `Auto(pages)` | 自动选择 | 根据页数判断 |

**关键实现**：
- `PageSpillWriter` 将页面内容序列化到临时文件
- `Auto` 在页数 > threshold 时切换到 Spill
- 临时文件在 drop 时自动清理

### 3.3 `builder/` — Builder API

| Builder | 职责 |
|---|---|
| `PdfWriterBuilder` | 链式配置 writer（backend / handler / metadata） |
| `PdfCreateBuilder` | 链式创建 PDF（add_text / add_image / add_table / do_write） |
| `PdfSplitBuilder` | 链式拆分 PDF |
| `PdfManipulateBuilder` | 链式操作 PDF（rotate / reorder） |
| `PdfTableBuilder<T>` | 链式表格构建（headers_from / data / position） |
| `PdfImageBuilder` | 链式图片构建（add_image / position / size） |
| `PdfWatermarkBuilder` | 链式水印配置（opacity / rotation / color） |
| `HtmlToPdfBuilder` | HTML→PDF 配置（page_size / margins / chromium_path） |
| `PdfFormBuilder` | 表单填充 Builder |

### 3.4 `image.rs` — 图片写入

| 方法 | 职责 |
|---|---|
| `write_image(img, x, y, w, h)` | 写入图片到指定位置 |
| `PdfImage::from_path(path)` | 从文件加载图片 |
| `PdfImage::from_bytes(bytes, format)` | 从内存加载图片 |

**支持格式**：JPEG、PNG（通过 printpdf 的 `RawImage::decode_from_bytes`）

### 3.5 `shape.rs` — 矢量图形

| 方法 | 职责 |
|---|---|
| `draw_line(x1, y1, x2, y2, line_width)` | 画直线 |
| `draw_rect_stroke(x, y, w, h, line_width)` | 画矩形（描边） |
| `draw_circle(cx, cy, radius, line_width)` | 画圆（4 段三次贝塞尔曲线近似，k = 0.5522847498） |

### 3.6 `template/` — AcroForm 模板填充

| 方法 | 职责 |
|---|---|
| `fill_form(template_path, data)` | 按字段名填充值 |
| `PdfFormBuilder` | Builder 模式表单填充 |

**关键约束**：
- 基于 lopdf 的 /AcroForm 字典解析
- 支持文本字段、复选框、单选按钮

### 3.7 内置处理器

| 处理器 | 职责 |
|---|---|
| `PageNumberHandler` | 在页脚注入页码 |
| `TextHeaderHandler` | 在页眉注入文本 |
| `TextFooterHandler` | 在页脚注入文本 |

## 4. 关键数据流

### 4.1 PDF 创建完整流程

```
EasyPdf::create(path)
    │
    ▼
PdfCreateBuilder::add_text("Hello")     ← 文本
    .add_table(&table)                   ← 表格
    .add_image(&img)                     ← 图片
    .do_write()                          ← 触发写入
    │
    ▼
PdfWriter::write_blocks(blocks)
    │
    ├── write_text() → printpdf Op::WriteText
    ├── write_table() → 遍历行列，画线+写文本
    ├── write_image() → RawImage::decode + Op::UseXobject
    └── draw_line/rect/circle() → Op::DrawLine
    │
    ▼
WriteHandlerChain::execute()
    ├── before_page → TextHeaderHandler
    ├── [内容写入]
    └── after_page → PageNumberHandler + TextFooterHandler
    │
    ▼
WriteBackend::InMemory / Spill
    │
    ▼
原子文件输出 (temp + rename)
```

### 4.2 表格渲染

```
PdfTable { headers, rows, column_widths }
    │
    ▼
遍历 rows/columns
    │
    ├── 画边框线（draw_line）
    ├── 写单元格文本（write_text）
    └── 应用 TableRenderConfig（边框样式）
    │
    ▼
printpdf::Table → PDF 页面
```

### 4.3 自定义字体

```
PdfWriter::register_font_from_path("font.ttf")
    │
    ▼
读取文件 → ParsedFont::from_bytes
    │
    ▼
doc.add_font("custom_font", parsed_font)
    │
    ▼
write_text_with_custom_font("Hello", "custom_font", 12.0, x, y)
    │
    ▼
使用自定义字体输出文本
```

## 5. 技术决策与权衡

| # | 决策 | 理由 | 权衡 |
|---|---|---|---|
| 1 | 基于 printpdf 而非从零实现 | printpdf 已处理 PDF 对象模型和语法 | 受限于 printpdf 的能力边界 |
| 2 | WriteBackend 用 enum 而非 trait | 模式匹配简洁，3 种模式已知 | 无法在运行时动态注册新后端 |
| 3 | 表格渲染用逐行画线 | 实现简单，printpdf 无内置表格 | 性能不如流式布局 |
| 4 | 自定义字体用 register 模式 | 避免重复解析同一字体 | 需要手动管理字体 key |
| 5 | 水印用内容流注入 | 不修改 PDF 结构 | 水印不可单独删除 |
| 6 | AcroForm 用 lopdf 直接操作 | printpdf 不支持表单 | 与写入引擎的抽象层不统一 |

## 6. 测试与验收

### 6.1 现有测试覆盖

| 测试 | 断言点 | 文件 |
|---|---|---|
| `test_write_text` | 文本写入 + 读取验证 | `writer.rs` tests |
| `test_write_table` | 表格渲染 + 读取验证 | `writer_helpers.rs` tests |
| `test_write_image` | 图片插入 + 文件大小验证 | `image.rs` tests |
| `test_draw_shapes` | 矢量图形 + 读取验证 | `shape.rs` tests |
| `test_register_font` | 自定义字体注册 + 使用 | `writer.rs` tests |
| `test_page_number_handler` | 页码注入 | handler tests |
| `test_watermark` | 文本/图片水印 | watermark tests |
| `test_write_backend_inmemory` | InMemory 模式 | `backend.rs` tests |
| `test_write_backend_spill` | Spill 模式 + 临时文件清理 | `backend.rs` tests |
| `test_write_backend_auto` | Auto 模式自动选择 | `backend.rs` tests |
| `test_fill_form` | AcroForm 填充 | `template/` tests |
| `test_multi_page` | 多页写入 + 页数验证 | `writer.rs` tests |
| `test_atomic_output` | 原子文件输出 | `writer.rs` tests |

### 6.2 已知局限

- 表格渲染不支持合并单元格（未来版本）。
- 矢量图形不支持填充（仅描边）。
- 自定义字体不支持字体子集化（嵌入完整字体）。
- AcroForm 不支持签名字段。
- Spill 模式的临时文件在进程崩溃时可能残留。

## 7. 引用

- 架构文档：`docs/easypdf-rust-Architecture.md` 第 6 节「easypdf-writer 写入引擎」
- 使用指南：`docs/usage-guide.md` 第 5 节「PDF 写入」、第 6 节「表格和图片」
- Roadmap：`docs/roadmap.md` 0.1 Foundation（基础写入）、0.3 Rich Content（增强）
- 源码：`crates/easypdf-writer/src/`（writer.rs / backend.rs / builder/ / image.rs / shape.rs）
