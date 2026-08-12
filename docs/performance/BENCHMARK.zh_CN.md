# 性能基准报告：easypdf vs pdftotext

**日期**：2026-08-11
**状态**：基准已建立

## 测试环境

| 项目 | 值 |
|------|-----|
| 操作系统 | macOS Darwin 25.5.0 arm64 |
| CPU | Apple M4 Pro |
| Rust | 1.97.1（stable，2026-07-14） |
| Rust edition | 2024（主 crate）/ 2021（bench crate） |
| Profile | release（优化） |
| pdftotext | 26.02.0（Poppler） |
| qpdf | 未安装（TODO：安装以完善内存对比） |

## 测试语料

来自 `easypdf-test/samples/benchmark_corpus/` 的 8 个 PDF，从主 samples 目录符号链接。

| PDF | 大小（字节） | 描述 |
|-----|-------------|------|
| minimal.pdf | 1,295 | 单页，最少内容 |
| with_acroform.pdf | 860 | 含表单字段的 PDF |
| with-metadata.pdf | 1,378 | 含元数据字段 |
| with_table_text.pdf | 1,852 | 表格布局带文本 |
| multi_column_heuristic.pdf | 2,077 | 多栏布局 |
| multipage.pdf | 2,147 | 多页 |
| nested_objects.pdf | 3,797 | 深层嵌套 PDF 对象 |
| large_100page.pdf | 72,118 | 100 页压力测试 |

**注意**：`corrupted_xref.pdf`、`encrypted_dummy.pdf` 和 `image_only.pdf` 已排除（无可提取文本或不适合对比）。

---

## 基准 1：文本提取速度（Criterion）

使用 Criterion 0.5 测量（每个基准 10 个样本，release 模式）。

### 墙钟时间（绝对值）

| PDF | 大小 | easypdf 中位数 | 95% CI |
|-----|------|---------------|--------|
| with_acroform.pdf | 860 B | 96.6 us | [93.6, 101.3] |
| minimal.pdf | 1.3 KB | 94.6 us | [89.7, 99.3] |
| with-metadata.pdf | 1.4 KB | 102.6 us | [101.4, 104.5] |
| with_table_text.pdf | 1.8 KB | 120.8 us | [119.7, 122.2] |
| multi_column_heuristic.pdf | 2.0 KB | 121.8 us | [119.5, 123.8] |
| multipage.pdf | 2.1 KB | 128.9 us | [122.1, 134.8] |
| nested_objects.pdf | 3.7 KB | 146.7 us | [138.7, 153.2] |
| large_100page.pdf | 70.4 KB | 2,439 us | [2,360, 2,515] |

### 吞吐量（large_100page.pdf）

| 指标 | 值 |
|------|-----|
| 吞吐量 | 28.7 MiB/s |
| 95% CI | [28.2, 29.1] MiB/s |

---

## 基准 2：速度对比（compare.sh）

每个 PDF 运行 3 次取中位数。两个工具均为冷启动模式。

| PDF | 大小 | easypdf（ms） | pdftotext（ms） | 加速比 |
|-----|------|-------------|----------------|--------|
| minimal.pdf | 1,295 B | 0 | 12 | pdftotext 较慢 |
| with_acroform.pdf | 860 B | 0 | 12 | pdftotext 较慢 |
| with-metadata.pdf | 1,378 B | 0 | 12 | pdftotext 较慢 |
| with_table_text.pdf | 1,852 B | 0 | 13 | pdftotext 较慢 |
| multi_column_heuristic.pdf | 2,077 B | 0 | 12 | pdftotext 较慢 |
| multipage.pdf | 2,147 B | 0 | 12 | pdftotext 较慢 |
| nested_objects.pdf | 3,797 B | 0 | 12 | pdftotext 较慢 |
| large_100page.pdf | 72,118 B | 3 | 17 | ~5.7x 更快 |

**注意**：亚毫秒级的 easypdf 时间在 shell 计时器中四舍五入为 0 ms（毫秒分辨率）。Criterion 在上方提供了精确的微秒测量。shell 对比主要表明 easypdf 的进程启动 + 提取时间被 pdftotext 的进程启动开销（~12 ms 最小值）所主导。对于 100 页 PDF，easypdf 大约快 5.7 倍。

---

## 基准 3：文本提取准确性

提取文本对比：easypdf vs pdftotext（基准真相）。

| PDF | easypdf 字符数 | pdftotext 字符数 | 字符比率 |
|-----|--------------|-----------------|---------|
| with_table_text.pdf | 186 | 189 | 0.9841 |
| large_100page.pdf | 20,183 | 18,992 | 0.9410 |
| with-metadata.pdf | 23 | 25 | 0.9200 |
| nested_objects.pdf | 20 | 22 | 0.9091 |
| with_acroform.pdf | 29 | 31 | 0.9355 |
| multipage.pdf | 23 | 27 | 0.8519 |
| minimal.pdf | 12 | 14 | 0.8571 |
| multi_column_heuristic.pdf | 496 | 363 | 0.7319 |

**总结**：
- **平均字符比率**：0.89（89%）
- **最佳**：with_table_text.pdf 为 98.4%
- **最差**：multi_column_heuristic.pdf 为 73.2%
- **大文件（100 页）**：94.1% -- easypdf 提取的字符数多于 pdftotext

**注意**：字符比率 = min(easypdf, pdftotext) / max(easypdf, pdftotext)。比率低于 1.0 不一定意味着提取不正确 -- 不同工具可能包含不同的空白字符、页眉/页脚或栏顺序。`multi_column_heuristic.pdf` 的情况显示 easypdf 提取了明显更多的文本（496 vs 363 字符），表明它可能捕获了 pdftotext 的栏启发式算法遗漏的内容，反之亦然。

---

## 基准 4：峰值内存（RSS）

使用 macOS 的 `/usr/bin/time -l` 测量（最大常驻集大小）。

| PDF | 大小 | easypdf RSS（KB） | pdftotext RSS（KB） | 比率 |
|-----|------|------------------|--------------------|----|
| with_acroform.pdf | 860 B | 6,992 | 9,872 | 0.71 |
| minimal.pdf | 1,295 B | 7,040 | 9,760 | 0.72 |
| with-metadata.pdf | 1,378 B | 7,024 | 9,808 | 0.72 |
| with_table_text.pdf | 1,852 B | 7,104 | 10,000 | 0.71 |
| multi_column_heuristic.pdf | 2,077 B | 7,040 | 9,952 | 0.71 |
| multipage.pdf | 2,147 B | 7,088 | 9,840 | 0.72 |
| nested_objects.pdf | 3,797 B | 7,152 | 9,808 | 0.73 |
| large_100page.pdf | 72,118 B | 8,720 | 10,464 | 0.83 |

**总结**：
- easypdf 对小文件使用约 **pdftotext 内存的 70-73%**
- 对于 100 页文件，差距缩小到 **83%**
- easypdf 基础 RSS（~7 MB）反映了 Rust 运行时 + tokio/allocator 开销
- pdftotext 基础 RSS（~10 MB）反映了 Poppler/C++ 运行时开销

---

## 关键发现

1. **速度**：easypdf 很快。对于 100 页压力测试，Criterion 测量墙钟时间为 2.4 ms，吞吐量为 28.7 MiB/s。pdftotext 对同一文件需要 ~17 ms（慢 7 倍，含进程启动）。

2. **内存**：easypdf 始终使用比 pdftotext 更少的峰值内存（小文件少 29%，100 页文件少 17%）。

3. **准确性**：平均字符比率为 89%。主要异常是 `multi_column_heuristic.pdf`，easypdf 比 pdftotext 多提取 37% 的文本。对于结构良好的 PDF（表格、元数据、表单），准确性为 92-98%。

4. **Criterion 统计说明**：一些基准在小文件上显示高方差（运行间变化 10-59%），测量噪声占主导。large_100page.pdf 基准在统计上最稳定。

## 已知限制

- **qpdf 未安装**：内存对比仅覆盖 easypdf vs pdftotext。TODO：通过 `brew install qpdf` 安装 qpdf 以进行完整的 3 工具对比。
- **语料较小**：共 8 个 PDF。应使用真实文档（扫描 PDF、大型文本 PDF、复杂布局）扩展语料以获得生产级基准。
- **准确性指标**：字符级比率是粗粒度指标。适当的 diff/Levenshtein 分析将提供更多关于提取差异的洞察。
- **单机测试**：结果特定于 Apple M4 Pro。CI 基准应在 Linux x86_64 上运行以进行跨平台对比。
- **easypdf-core::crypto 编译**：`easypdf-core` 中的 `crypto` 模块在主 workspace 的 lockfile 之外编译时存在预存的 API 不兼容性。这不影响读取功能，但阻止 bench crate 成为 workspace 成员。

## 复现方法

```bash
# 构建并运行对比
cd easypdf-rust/benches/external_comparison
./compare.sh ../../easypdf-test/samples/benchmark_corpus

# 构建并运行内存对比
./compare_memory.sh ../../easypdf-test/samples/benchmark_corpus

# 运行 Criterion 基准
cargo bench --bench text_extraction
cargo bench --bench accuracy
```
