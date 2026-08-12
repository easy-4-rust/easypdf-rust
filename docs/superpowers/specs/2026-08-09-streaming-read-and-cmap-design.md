# easypdf-reader 流式读取与 CMap 设计

- **日期**：2026-08-09
- **作者**：ZCode Agent（协同设计）
- **状态**：已实现（v0.1.0 发布）
- **依赖**：easypdf-reader 现有 `reader/`、`strategy.rs`、`streaming/`、`manipulate.rs`

## 1. 目标与范围

为 easypdf-rust 实现**三层读取策略**（Full / Lazy / Streaming），使大文件（>100MB）能在常量内存下完成文本提取；同时实现 CMap/ToUnicode 支持，修复 CJK 文本提取乱码问题。

**核心需求**：

1. `ReadStrategy` 枚举支持 Full / Lazy / Streaming 三种模式。
2. `ReadStrategy::auto()` 按文件大小自动选择最佳策略。
3. Streaming 模式下不构建完整 `lopdf::Document` 对象，直接扫描字节流。
4. CMap 解析支持 CID-keyed 字体的字符映射。
5. ToUnicode 表读取修复 CJK（中文/日文/韩文）文本提取。
6. 单会话复用（session reuse）避免重复打开同一文件。

**非目标**：

- 不实现 PDF 页面渲染（仅文本提取）。
- 不实现 PDF 表单字段的流式读取。
- 不实现加密 PDF 的流式解密（需先解密再读取）。
- 不支持 PDF 2.0 的新 CMap 格式。

## 2. 总体架构

```
┌──────────────────────────────────────────────┐
│            easypdf-reader                    │
│                                              │
│  ┌─────────────────────────────────────┐     │
│  │  ReadStrategy::auto(file_size)      │     │
│  │  ┌────────┬────────┬─────────────┐  │     │
│  │  │  Full  │  Lazy  │  Streaming  │  │     │
│  │  │ < 5MB  │ 5-100MB│  > 100MB    │  │     │
│  │  └────┬───┴────┬───┴──────┬──────┘  │     │
│  │       │        │          │          │     │
│  │       ▼        ▼          ▼          │     │
│  │  lopdf::Document   byte_finder       │     │
│  │  (完整加载)    (部分加载)  (流式扫描) │     │
│  └─────────────────────────────────────┘     │
│                                              │
│  ┌─────────────────────────────────────┐     │
│  │  streaming/                         │     │
│  │  ├── byte_finder.rs  字节级定位     │     │
│  │  ├── cmap.rs         CMap 解析      │     │
│  │  ├── scanner.rs      内容流扫描     │     │
│  │  └── text_extract.rs 文本提取       │     │
│  └─────────────────────────────────────┘     │
│                                              │
│  ┌─────────────────────────────────────┐     │
│  │  manipulate.rs                      │     │
│  │  ├── merge_files()  合并 PDF        │     │
│  │  ├── rotate_page()  旋转页面        │     │
│  │  └── reorder_pages() 重排页面       │     │
│  └─────────────────────────────────────┘     │
└──────────────────────────────────────────────┘
```

## 3. 模块职责划分

### 3.1 `strategy.rs` — 读取策略

| 策略 | 触发条件 | 内存模型 | 适用场景 |
|---|---|---|---|
| `Full` | 文件 < 5MB | 完整加载 lopdf::Document | 小文件，需要完整对象访问 |
| `Lazy` | 文件 5-100MB | 部分加载，按需解析 | 中等文件，平衡速度和内存 |
| `Streaming` | 文件 > 100MB | 常量内存，字节流扫描 | 大文件，仅需文本提取 |
| `auto(size)` | 用户调用 | 自动选择 | 默认推荐 |

**关键实现**：

```rust
pub enum ReadStrategy {
    Full,      // lopdf::Document 完整加载
    Lazy,      // lopdf::Document 按需解析
    Streaming, // 字节流扫描，不构建 Document
}

impl ReadStrategy {
    pub fn auto(file_size: u64) -> Self {
        if file_size < 5 * 1024 * 1024 { ReadStrategy::Full }
        else if file_size < 100 * 1024 * 1024 { ReadStrategy::Lazy }
        else { ReadStrategy::Streaming }
    }
}
```

### 3.2 `streaming/byte_finder.rs` — 字节级定位

| 函数 | 职责 |
|---|---|
| `find_object_offsets()` | 扫描 PDF 文件，定位所有对象的字节偏移 |
| `find_xref_offset()` | 定位 xref 表的起始位置 |
| `find_content_stream()` | 定位页面内容流的字节范围 |

**设计要点**：
- 使用有限状态机（FSM）解析 PDF 语法
- 只缓存对象偏移表（约 8 bytes/object），不缓存对象内容
- O(1) 内存（仅偏移表，不加载对象）

### 3.3 `streaming/cmap.rs` — CMap 解析

| 结构 | 职责 |
|---|---|
| `CMap` | CID → Unicode 映射表 |
| `parse_cmap(data: &[u8])` | 解析 CMap 流（`beginbfchar` / `beginbfrange` / `beginCIDchar` / `beginCIDRange`） |
| `ToUnicode` | ToUnicode 表读取（PDF `/ToUnicode` stream） |

**CJK 支持**：

```
问题：CJK PDF 使用 CID-keyed 字体，文本提取得到 CID 而非 Unicode。
方案：
1. 读取字体的 /ToUnicode stream
2. 解析 CMap（beginbfchar / beginbfrange 映射）
3. 将 CID 查表转为 Unicode
4. 若无 ToUnicode，尝试使用 Adobe-GB1 / Adobe-Japan1 等预定义 CMap
```

### 3.4 `streaming/scanner.rs` — 内容流扫描

| 函数 | 职责 |
|---|---|
| `scan_content_stream()` | 解析 PDF 内容流操作符（BT/ET/Tj/TJ/Td/TD） |
| `extract_text_from_stream()` | 从单个内容流提取文本 |
| `decode_text_string()` | 解码 PDF 文本字符串（UTF-16BE / PDFDocEncoding） |

### 3.5 `streaming/text_extract.rs` — 文本提取

| 函数 | 职责 |
|---|---|
| `extract_text_streaming()` | Streaming 模式的主入口 |
| `extract_page_text()` | 单页文本提取 |
| `merge_text_fragments()` | 合并同一行的文本片段 |

### 3.6 `manipulate.rs` — PDF 操作

| 函数 | 职责 |
|---|---|
| `merge_files(paths, output)` | 合并多个 PDF，修正 /Pages 树 |
| `rotate_page(page, rotation)` | 每页独立旋转 0/90/180/270 |
| `reorder_pages(order)` | 按指定顺序重排页面 |

**关键约束**：
- 操作基于 lopdf 的对象模型（Full/Lazy 模式）
- Streaming 模式不支持操作（只读）

## 4. 关键数据流

### 4.1 Streaming 文本提取

```
input.pdf (>100MB)
    │
    ▼
byte_finder::find_object_offsets()  → 偏移表
    │
    ▼
byte_finder::find_content_stream(page)  → 内容流字节范围
    │
    ▼
scanner::scan_content_stream(bytes)  → 操作符序列
    │
    ▼
text_extract::decode_text_string()  → Unicode 文本
    │  （查 CMap 表转换 CID）
    ▼
合并同页文本片段 → String
```

### 4.2 CJK 文本提取

```
CJK PDF
    │
    ▼
读取字体 /ToUnicode stream
    │
    ▼
cmap::parse_cmap(data)  → CMap { cid_to_unicode: HashMap<u16, char> }
    │
    ▼
Tj/TJ 操作符 → CID 序列
    │
    ▼
查 CMap 表 → Unicode 文本
```

### 4.3 PDF 合并

```
[pdf1.pdf, pdf2.pdf, pdf3.pdf]
    │
    ▼
逐个加载 lopdf::Document
    │
    ▼
复制所有对象到目标 Document（重编号避免冲突）
    │
    ▼
合并 /Pages 树（/Kids 数组拼接）
    │
    ▼
更新 /Count（总页数）
    │
    ▼
原子文件输出
```

## 5. 技术决策与权衡

| # | 决策 | 理由 | 权衡 |
|---|---|---|---|
| 1 | Streaming 用 FSM 而非正则 | PDF 语法不是正则语言，FSM 更可靠 | 实现复杂度高 |
| 2 | CMap 缓存为 HashMap | O(1) 查找 | 内存占用（大 CMap 约 100KB） |
| 3 | Streaming 不支持操作 | 操作需要完整对象模型 | 用户需先判断策略 |
| 4 | auto() 阈值 5MB/100MB | 基于实测性能数据 | 可能不适合所有硬件 |
| 5 | 合并用对象重编号 | 避免对象号冲突 | 大文件合并较慢 |
| 6 | session reuse 在 Reader 级别 | 避免重复打开同一文件 | 需要持有 Document 引用 |

## 6. 测试与验收

### 6.1 现有测试覆盖

| 测试 | 断言点 | 文件 |
|---|---|---|
| `test_read_strategy_auto` | 5MB/50MB/100MB 阈值正确 | `strategy.rs` |
| `test_extract_text_basic` | 英文文本提取 | `reader/` tests |
| `test_extract_text_cjk` | CJK 文本提取（CJK PDF fixture） | `reader/` tests |
| `test_merge_files` | 合并后页数和内容正确 | `manipulate.rs` tests |
| `test_rotate_page` | 旋转后 /Rotate 值正确 | `manipulate.rs` tests |
| `test_reorder_pages` | 重排后页面顺序正确 | `manipulate.rs` tests |
| `test_session_reuse` | 129x 加速验证 | `reader/` benches |
| `test_byte_finder` | 偏移表正确性 | `streaming/` tests |
| `test_cmap_parse` | CMap 解析正确性 | `streaming/cmap.rs` tests |

### 6.2 已知局限

- Streaming 模式不支持加密 PDF。
- Streaming 模式不支持表单字段提取。
- CMap 解析不支持 PDF 2.0 新格式。
- 合并大文件（>500MB）可能较慢（对象复制开销）。

## 7. 引用

- 架构文档：`docs/easypdf-rust-Architecture.md` 第 5 节「easypdf-reader 读取引擎」
- 使用指南：`docs/usage-guide.md` 第 4 节「PDF 读取」
- Roadmap：`docs/roadmap.md` 0.2 Architecture Consolidation（Streaming / CMap）
- 源码：`crates/easypdf-reader/src/strategy.rs`、`crates/easypdf-reader/src/streaming/`
