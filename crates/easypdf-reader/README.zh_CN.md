# easypdf-reader

> PDF 读取层：解析、文本提取、页面操作（合并/拆分/旋转/重排/水印），支持三种自适应读取策略。

## 角色

`easypdf-reader` 负责 easypdf-rust 工作区中所有的 PDF 输入操作。它基于 `lopdf` 后端解析 PDF 文档，提供文本提取、元数据读取、页面操作（合并、拆分、旋转、重排、提取、水印、图层）、PDF/A 验证等能力。根据文件大小自动选择最佳读取策略，兼顾小文件的速度与大文件的内存效率。

## 核心能力

- **三种读取策略**（`Full` / `Lazy` / `Streaming`）——按文件大小自动选择：0-5 MB = Full，5-100 MB = Lazy，>100 MB = Streaming（`crates/easypdf-reader/src/strategy.rs:56-68`）
- **文本提取**（`extract_text()`）——从 PDF 中提取纯文本，支持 CMap/ToUnicode 编码字体（CJK）（`crates/easypdf-reader/src/reader/extract.rs`）
- **页面操作**（`PdfManipulator`）——合并、拆分、旋转、重排、提取页面、添加文字水印、添加可选内容组（图层）（`crates/easypdf-reader/src/manipulate.rs`）
- **PDF 修复**（`open_with_repair()`）——自动检测并修复损坏的 PDF 文件（`crates/easypdf-core/src/io/repair.rs`）
- **资源守卫**——通过 `ResourceLimits` 防止解压炸弹和元素爆炸（`crates/easypdf-core/src/io/guards.rs`）
- **流式扫描器**（`StreamScanner`）——字节流扫描，不构建完整 `Document` 对象，适用于超大文件（`crates/easypdf-reader/src/streaming/`）
- **PDF/A 验证**（`validate_pdfa()`）——检查 PDF/A-1b 合规性（`crates/easypdf-reader/src/manipulate.rs:260`）
- **性能基准**——reader session 基准测试（`crates/easypdf-reader/benches/reader_session.rs`）

## 依赖

### 内部依赖

| Crate | 用途 |
|-------|------|
| `easypdf-core` | 核心类型（`PdfInput`、`ResourceLimits`、`PageRange`、错误类型、IO 守卫） |

### 外部依赖

| Crate | 版本 | 用途 |
|-------|------|------|
| `lopdf` | 0.44.0 | PDF 解析引擎 |
| `flate2` | 1.1.9 | 流解压缩（Streaming 策略） |

## 主要 API

### PdfReader

```rust
use easypdf_reader::{PdfReader, ReadStrategy};

// 自动策略（根据文件大小选择）
let reader = PdfReader::open("document.pdf")?;
let text = reader.extract_text()?;

// 指定策略
let reader = PdfReader::open_with_strategy("large.pdf", ReadStrategy::Lazy)?;
let text = reader.pages(0..5).extract_text()?;

// 从内存字节
let reader = PdfReader::from_bytes(pdf_bytes)?;

// 自动修复模式
let reader = PdfReader::open_with_repair("corrupted.pdf", true, ReadStrategy::Full)?;

// 自定义资源限制
let reader = PdfReader::open_with_limits(input, ResourceLimits::default())?;
```

### ReadStrategy

```rust
use easypdf_reader::ReadStrategy;

// 按文件大小自动选择
let strategy = ReadStrategy::auto(50_000_000); // 50 MB -> Lazy

// 手动选择
let s = ReadStrategy::Full;      // <5 MB，完整对象树
let s = ReadStrategy::Lazy;      // 5-100 MB，按需加载页面
let s = ReadStrategy::Streaming;  // >100 MB，字节流扫描
```

### PdfManipulator

```rust
use easypdf_reader::PdfManipulator;

// 合并多个 PDF
PdfManipulator::merge_files(&["a.pdf", "b.pdf"], "merged.pdf")?;

// 打开并操作
let mut m = PdfManipulator::open("input.pdf")?;
m.rotate_page(0, Rotation::Clockwise90)?;
m.reorder_pages(&[2, 0, 1])?;
m.extract_pages(&(0..5))?;
m.add_text_watermark("机密", 48.0, 0.3)?;
m.add_layer("批注")?;
m.validate_pdfa()?;
```

## 已知限制

- `ReadStrategy::Streaming` 不构建完整对象树——精度低于 Full/Lazy，尤其是 CJK 文本边界（`crates/easypdf-reader/src/strategy.rs:47-51`）
- Streaming 模式跳过交叉引用解析和字体编码（CMap/ToUnicode）以换取速度

## License

Apache-2.0

---

**项目主页**：https://github.com/easy-4-rust/easypdf-rust
**crates.io**：https://crates.io/crates/easypdf-reader
**docs.rs**：https://docs.rs/easypdf-reader
