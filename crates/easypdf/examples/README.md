# easypdf 示例集合

本目录包含 11 个可运行的示例，覆盖 easypdf 的核心 API。

## 运行方式

所有示例位于 `crates/easypdf/examples/`。

```bash
cd crates/easypdf

# 基础示例（无需额外 feature）
cargo run --example create_basic
cargo run --example read_basic

# Markdown 相关示例（需要 markdown feature，默认已启用）
cargo run --example pdf_to_markdown --features markdown
cargo run --example markdown_pipeline --features markdown
```

所有示例使用 `tempfile::tempdir()` 写入临时目录，运行结束后自动清理，不污染当前目录。

---

## 示例列表

### 基础（无额外 feature 需求）

#### `create_basic.rs`

最简单的 PDF 创建示例。展示 `EasyPdf::create()` builder 链：设置标题、自定义字体、彩色文本，最后调用 `do_write()` 输出文件。

```bash
cargo run --example create_basic
```

#### `read_basic.rs`

读取 PDF 的文本内容、页数和元数据。先创建一份示例 PDF，再用 `EasyPdf::read()` 的 `extract_text()` 和 `metadata()` 读回。

```bash
cargo run --example read_basic
```

#### `merge_pdfs.rs`

合并多个单页 PDF 为一个文件。先生成三份单页 PDF，再调用 `EasyPdf::merge()` 合并输出。

```bash
cargo run --example merge_pdfs
```

#### `split_pdf.rs`

按页拆分 PDF。先创建一份多页 PDF，再将其拆分为多个单页文件。

```bash
cargo run --example split_pdf
```

#### `manipulate_rotate.rs`

旋转页面与重新排序。创建一份三页 PDF，演示单页旋转、全部旋转、以及页面重排操作。

```bash
cargo run --example manipulate_rotate
```

#### `create_table.rs`

创建含表格的 PDF。使用 `PdfTable` 构建表头和数据行，通过 `add_table()` 渲染到 PDF。

```bash
cargo run --example create_table
```

#### `create_multi_page.rs`

使用底层 `PdfWriter` API 手动构造多页 PDF。展示 `PdfWriterBuilder` 的 metadata 设置、逐页添加内容、页码控制等完整流程。

```bash
cargo run --example create_multi_page
```

#### `streaming_read.rs`

Streaming 读取策略示例。演示 `ReadStrategy` 的三种模式：

- `Full` —— 一次性加载全部内容（默认，适合小文件）
- `Lazy` —— 延迟加载页面
- `Streaming` —— 增量扫描（适合大文档，避免 OOM）

同时展示自动策略选择。

```bash
cargo run --example streaming_read
```

#### `fill_form.rs`

使用 `#[derive(PdfModel)]` derive 宏进行类型安全的 PDF 表单填充。展示 `#[pdf(field = "...")]` 属性将 Rust 结构体字段映射到 PDF AcroForm 字段，以及 `PdfFillBuilder` 的 builder 模式。

> **注意**：此示例展示 API 模式和 derive 宏的字段内省。实际表单填充需要一个包含交互式表单字段（AcroForm）的 PDF 模板。

```bash
cargo run --example fill_form
```

---

### Markdown 相关（需要 `markdown` feature）

> `markdown` 是默认 feature，通常无需额外指定。以下命令显式声明以确保可用。

#### `pdf_to_markdown.rs`

PDF 转 Markdown。使用 `EasyPdf::to_markdown()` API 将 PDF 转为 Markdown 文本，支持三种 profile 预设：

- `MarkdownProfile::Gfm` —— GitHub Flavored Markdown
- `MarkdownProfile::Llm` —— 面向 LLM 的精简格式
- `MarkdownProfile::Plain` —— 纯文本格式

展示 `do_convert()` 内存转换和 `export_markdown()` 文件导出两种方式，以及 `convert_report()` 转换报告（页数、块数、字节数、警告）。

```bash
cargo run --example pdf_to_markdown --features markdown
```

#### `markdown_pipeline.rs`

自定义 Markdown 处理管线。演示：

- 使用 `EasyPdf::markdown_pipeline()` 创建管线
- 用不同 profile（Gfm / Llm / Plain）分别转换同一 PDF
- 配置 `TablePolicy::Detect` 表格检测策略
- 配置 `OcrPolicy::Disabled` OCR 策略

```bash
cargo run --example markdown_pipeline --features markdown
```

---

## 关键 API 速查

```rust
// --- 创建 PDF ---
EasyPdf::create(&path)
    .title("标题")
    .add_text("内容")
    .do_write()?;

// --- 底层写入（多页控制） ---
let mut writer = EasyPdf::writer("标题")
    .metadata(...)
    .build()?;
writer.add_page(PageSize::A4, Orientation::Portrait)?;
writer.write_text(&text, x, y)?;
writer.finish(&path)?;

// --- 合并 ---
EasyPdf::merge(&[path1, path2], &output_path)?;

// --- 读取 ---
EasyPdf::read(&path).extract_text()?;
EasyPdf::read(&path).metadata()?;
EasyPdf::read(&path).strategy(ReadStrategy::Streaming).extract_text()?;

// --- 表格 ---
let table = PdfTable::new(vec!["列1".into(), "列2".into()]).row(vec!["a".into(), "b".into()]);
EasyPdf::create(&path).add_table(&table).do_write()?;

// --- PDF 转 Markdown ---
let result = EasyPdf::to_markdown(&path)
    .profile(MarkdownProfile::Gfm)
    .tables(TablePolicy::Detect)
    .do_convert()?;
println!("{}", result.markdown());

// --- 文件导出 ---
EasyPdf::export_markdown(&pdf_path, &md_path)
    .profile(MarkdownProfile::Gfm)
    .do_export()?;

// --- 表单填充（derive 宏） ---
#[derive(PdfModel)]
struct MyForm {
    #[pdf(field = "name")]
    name: String,
}
```

## 进一步阅读

- 主项目 README：[`../../../README.md`](../../../README.md)
- API 文档：`cargo doc -p easypdf --open`
- 性能基准：[`../../../docs/performance/BENCHMARK.md`](../../../docs/performance/BENCHMARK.md)
- 安全审计：[`../../../docs/security/AUDIT.md`](../../../docs/security/AUDIT.md)
