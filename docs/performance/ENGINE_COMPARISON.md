# 引擎对比基准 / Engine Comparison Benchmark

> 对比 easypdf-rust 的两个写入后端：printpdf（默认）和 krilla（`writer-krilla` feature）。
> Comparison of easypdf-rust write backends: printpdf (default) and krilla (`writer-krilla` feature).

---

## 测试环境 / Test Environment

| 项目 | 值 |
|---|---|
| 日期 | 2026-08-16 |
| 平台 | macOS aarch64 (Apple Silicon) |
| Rust | 1.88 (Edition 2024) |
| 迭代次数 | 10 |
| 测试内容 | 3 页 A4 文档，每页含 2 段文本 + 1 条线段 + 1 个矩形 |

## 测试结果 / Results

| 指标 / Metric | printpdf | krilla |
|---|---|---|
| 平均生成耗时（3 页） | 6.3 ms | 7.1 ms |
| 输出体积（bytes） | 3,003 | 13,167 |
| 输出体积（KB） | 2.9 | 12.9 |
| 页数 | 3 | 3 |
| Base14 内置字体 | 支持（无需嵌入） | 不支持（需提供真实字体文件） |
| SVG | 支持 | 不支持 |
| 字体子集化 | 否 | 是 |
| CJK 优化 | 否 | 是（配合字体子集化） |
| 体积比（krilla vs printpdf） | 基线 | 4.4x |

## 分析 / Analysis

### printpdf 优势

- **内置字体零嵌入**：使用 PDF 标准 14 字体时，输出体积最小（不嵌入字体数据）。
- **SVG 支持**：可直接嵌入 SVG 矢量图形。
- **成熟稳定**：printpdf 是 Rust 生态中最成熟的 PDF 库之一。

### krilla 优势

- **字体子集化**：仅嵌入文档实际使用的字形，对 CJK 大字体文件效果显著。
  - 示例：10MB 的中文字体文件，printpdf 嵌入全部 → 10MB+ 输出；
    krilla 子集化 → 仅嵌入使用过的字形，输出可降至数百 KB。
- **体积优化**：对使用自定义字体（尤其是 CJK）的文档，输出体积可大幅缩小。

### 选择建议

| 场景 | 推荐引擎 |
|---|---|
| 西文文档、使用内置字体 | printpdf（默认） |
| CJK 文档、需要体积优化 | krilla（`writer-krilla`） |
| 需要 SVG 嵌入 | printpdf |
| 需要 Base14 字体 | printpdf |
| 批量生成、关注存储成本 | krilla（CJK）/ printpdf（西文） |

## 复现 / Reproduction

```bash
# 运行基准测试
cargo run --release -p easypdf-test --all-features --bin engine_bench
```

## 备注 / Notes

- 本基准使用 Helvetica 系统字体（macOS）。在 Linux CI 环境中可能无此字体，
  测试会自动跳过文本写入部分。
- krilla 的体积优势主要体现在**自定义字体嵌入**场景。使用内置字体时，
  printpdf 因零嵌入而体积更小。
- 后续可扩展基准覆盖 CJK 文档、图片嵌入等更多场景。
