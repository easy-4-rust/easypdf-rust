# easypdf-derive

> 过程宏 crate：提供 `#[derive(PdfModel)]` 及 10+ 个 `#[pdf(...)]` 属性宏。

## 角色

`easypdf-derive` 是 easypdf-rust 的过程宏 crate，为 Rust 结构体自动生成 `PdfModel` trait 实现。通过 `#[derive(PdfModel)]` 和丰富的 `#[pdf(...)]` 属性，开发者可以用声明式的方式将 Rust 结构体映射为 PDF 内容元素（文本、表格、图片），无需手写渲染逻辑。

## 核心能力

- **`#[derive(PdfModel)]`**——自动实现 `PdfModel` trait，生成 `render()` 和 `metadata()` 方法
- **结构体级属性**——`#[pdf(page = A4, orientation = Portrait, margins = 72)]`
- **字段级属性**——`text`、`table`、`image`、`ignore`、`field`、`order`、`default`、`required`、`format`、`nested`、`font`、`size`
- **字段描述符生成**——为表单填充和数据映射生成 `PdfFieldDescriptor`

## 依赖

- `syn`: Rust 语法解析
- `quote`: 代码生成
- `proc-macro2`: 过程宏基础设施
- `proc-macro-crate`: crate 名称解析

## 主要 API

### `#[derive(PdfModel)]`
```rust
use easypdf_derive::PdfModel;

#[derive(PdfModel)]
#[pdf(page = A4, orientation = Portrait)]
struct Invoice {
    #[pdf(text, position = (100, 700), font = "Helvetica", size = 14)]
    title: String,

    #[pdf(table, position = (50, 600))]
    items: Vec<Vec<String>>,

    #[pdf(image, position = (400, 700))]
    logo: Vec<u8>,

    #[pdf(field = "invoice_number", required)]
    number: String,

    #[pdf(ignore)]
    internal_note: String,
}
```

### 属性一览

| 属性 | 说明 |
|------|------|
| `#[pdf(text, position = (x, y))]` | 字段渲染为定位文本 |
| `#[pdf(table, position = (x, y))]` | 字段渲染为表格 |
| `#[pdf(image, position = (x, y))]` | 字段渲染为图片 |
| `#[pdf(ignore)]` / `#[pdf(skip)]` | 跳过该字段 |
| `#[pdf(field = "name")]` | 映射到 PDF 表单字段名 |
| `#[pdf(order = N)]` | 显示/渲染顺序 |
| `#[pdf(default = "value")]` | 空值时的默认值 |
| `#[pdf(required)]` | 字段必须非空 |
| `#[pdf(format = "pattern")]` | 格式化模式（如 `"YYYY-MM-DD"`） |
| `#[pdf(nested)]` | 递归包含内部模型的元素 |
| `#[pdf(font = ...)]` | 设置文本渲染字体 |
| `#[pdf(size = N)]` | 设置文本渲染字号 |

## License

Apache-2.0

---

**项目主页**：https://github.com/easy-4-rust/easypdf-rust
**crates.io**：https://crates.io/crates/easypdf-derive
