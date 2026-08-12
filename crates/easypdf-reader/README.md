# easypdf-reader

> PDF 读取层：解析、文本提取、页面操作（合并/拆分/旋转），支持三种读取策略。

## 角色

`easypdf-reader` 负责从 PDF 文件中提取内容。它基于 `lopdf` 后端解析 PDF 文档，提供文本提取、元数据读取、页面操作（合并、拆分、旋转）等能力。根据文件大小自动选择最佳读取策略，兼顾小文件的速度与大文件的内存效率。

## 核心能力

- **三种读取策略**（`Full` / `Lazy` / `Streaming`）——根据文件大小自动选择最优解析方式
- **文本提取**（`extract_text`）——从 PDF 中提取纯文本内容
- **页面操作**（`PdfManipulator`）——合并多个 PDF、拆分页面、旋转页面
- **PDF 修复**（`open_with_repair`）——自动检测并修复损坏的 PDF 文件
- **资源限制**——防止解压炸弹和元素爆炸

## 依赖

- `easypdf-core`: 核心类型（`PdfInput`、`ResourceLimits`、`PageRange`、错误类型）
- `lopdf`: PDF 底层解析引擎
- `flate2`: 流解压缩（Streaming 策略）

## 主要 API

### `PdfReader`
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
```

### `ReadStrategy`
```rust
use easypdf_reader::ReadStrategy;

let strategy = ReadStrategy::auto(50_000_000); // 50 MB -> Lazy
assert!(strategy.is_lazy());
```

### `PdfManipulator`
```rust
use easypdf_reader::PdfManipulator;

// 合并多个 PDF
PdfManipulator::merge_files(&["a.pdf", "b.pdf"], "merged.pdf")?;

// 拆分
let manipulator = PdfManipulator::open("input.pdf")?;
manipulator.split(0..5, "part1.pdf")?;
manipulator.split(5..10, "part2.pdf")?;

// 旋转
manipulator.rotate_pages(&[0, 1], Rotation::Degrees90, "rotated.pdf")?;
```

## License

Apache-2.0

---

**项目主页**：https://github.com/easy-4-rust/easypdf-rust
**crates.io**：https://crates.io/crates/easypdf-reader
