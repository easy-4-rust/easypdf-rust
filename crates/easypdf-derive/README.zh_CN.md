# easypdf-derive

> 过程宏 crate：提供 `#[derive(PdfModel)]` 及 12+ 个 `#[pdf(...)]` 属性宏，声明式 PDF 内容映射。

## 角色

`easypdf-derive` 是 easypdf-rust 的过程宏 crate。通过 `#[derive(PdfModel)]` 为 Rust 结构体自动生成 `PdfModel` trait 实现。配合丰富的 `#[pdf(...)]` 属性，开发者可以用声明式的方式将 Rust 结构体映射为 PDF 内容元素（文本、表格、图片、表单字段），无需手写渲染逻辑。

## 核心能力

- **`#[derive(PdfModel)]`**——自动实现 `PdfModel` trait，生成 `render()`、`metadata()` 和 `field_descriptors()` 方法（`crates/easypdf-derive/src/lib.rs:54`）
- **结构体级属性**——`#[pdf(page = A4, orientation = Portrait)]` 页面配置（`crates/easypdf-derive/src/lib.rs:44`）
- **字段级属性**——`text`、`table`、`image`、`ignore`/`skip`、`field`、`order`、`default`、`required`、`format`、`nested`、`font`、`size`（`crates/easypdf-derive/src/lib.rs:46-55`）
- **字段描述符生成**——为表单填充和数据映射生成 `PdfFieldDescriptor`（`crates/easypdf-core/src/traits.rs:40`）
- **编译期验证**——`trybuild` 测试确保无效属性产生清晰的错误信息（`crates/easypdf-derive/Cargo.toml:dev-dependencies`）

## 依赖

| Crate | 版本 | 用途 |
|-------|------|------|
| `syn` | 3.0.3 | Rust 源码解析（features = ["full"]） |
| `quote` | 1.0.47 | 代码生成 |
| `proc-macro2` | 1.0.107 | proc-macro 2.0 基础设施 |
| `proc-macro-crate` | 3.5.0 | crate 名称解析 |

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

    #[pdf(order = 1)]
    date: String,

    #[pdf(nested)]
    address: Address,

    #[pdf(ignore)]
    internal_note: String,
}
```

### 属性一览

| 属性 | 说明 |
|------|------|
| `#[pdf(page = A4, orientation = Portrait)]` | 结构体级页面配置 |
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

## 生成的代码

派生宏会自动生成：

```rust
// 对每个 #[derive(PdfModel)] 的结构体
impl PdfModel for MyStruct {
    fn render(&self) -> Result<Vec<RenderedElement>> {
        // 根据字段属性生成渲染逻辑
        // text -> 定位文本, table -> 表格, image -> 图片
    }

    fn metadata(&self) -> PdfModelMetadata {
        // 从结构体级 #[pdf(...)] 获取页面大小、方向
    }

    fn field_descriptors(&self) -> Vec<PdfFieldDescriptor> {
        // 从 #[pdf(field = "...", required, default = "...")] 生成
    }
}
```

## 编译期验证

无效属性会产生清晰的编译器错误：

```rust
#[derive(PdfModel)]
struct Bad {
    #[pdf(text)]  // 错误：text 字段缺少 position
    name: String,

    #[pdf(field = "x", default = "val", required)]  // 错误：default 和 required 互斥
    value: String,
}
```

## License

Apache-2.0

---

**项目主页**：https://github.com/easy-4-rust/easypdf-rust
**crates.io**：https://crates.io/crates/easypdf-derive
**docs.rs**：https://docs.rs/easypdf-derive
