# printpdf 依赖评估报告

> 评估日期：2026-08-11
> 评估范围：printpdf 0.12.5 及其传递依赖中的弃用/unsound 问题
> 结论：**短期接受现状 + 监控，中期（3-6 个月）评估 lopdf 直接方案**

---

## 执行摘要

- **当前状态**：printpdf 0.12.5 是 crates.io 上的最新版本，无升级路径。其传递依赖中存在 1 个 unsound 漏洞（lru 0.16.4，use-after-free）和 3 个 unmaintained 警告（bincode、rustybuzz、ttf-parser）。
- **推荐**：短期内接受现状并监控上游进展；中期将 easypdf-writer 的 PDF 创建层迁移到直接使用 lopdf（项目已有 lopdf 依赖且 template.rs 已直接使用）。
- **风险等级**：lru 的 use-after-free 为 **中等风险**（需要特定 panic 路径触发，azul-layout 内部使用，攻击面有限）。

---

## 当前 printpdf 使用情况

### 用量统计

printpdf 在以下 crate 中被直接引用：

| Crate | 文件 | 引用数 | 用途 |
|-------|------|--------|------|
| easypdf-writer | writer.rs | 15 处 | 核心 PDF 创建 |
| easypdf-writer | font.rs | 2 处 | 内置字体映射 |
| easypdf-writer | image.rs | 4 处 | 图片/SVG 嵌入 |
| easypdf-writer | shape.rs | 4 处 | 线条/矩形/圆形绘制 |
| easypdf-writer | backend.rs | 2 处 | Op 类型用于 spill 序列化 |
| easypdf | html.rs | 4 处 | HTML 转 PDF（可选 feature） |

### 关键使用 API

#### 文本 API（writer.rs）

- `PdfDocument::new(title)` -- 创建文档
- `PdfPage::new(width_mm, height_mm, ops)` -- 创建页面
- `Op::StartTextSection` / `Op::EndTextSection` -- 文本段
- `Op::SetTextCursor { pos: Point }` -- 文本位置
- `Op::SetFont { font: PdfFontHandle, size: Pt }` -- 设置字体
- `Op::ShowText { items: Vec<TextItem> }` -- 显示文本
- `PdfFontHandle::Builtin(BuiltinFont)` -- 内置字体句柄
- `PdfFontHandle::External(FontId)` -- 自定义字体句柄
- `TextItem::Text(String)` -- 文本内容

#### 字体 API（writer.rs, font.rs）

- `printpdf::BuiltinFont` 枚举（Helvetica/Times/Courier/Symbol/ZapfDingbats 全系列）
- `printpdf::ParsedFont::from_bytes(data, index, &mut warnings)` -- 解析 TTF/OTF
- `PdfDocument::add_font(&parsed)` -> `FontId` -- 注册自定义字体

#### 图像 API（image.rs）

- `printpdf::RawImage::decode_from_bytes(&data, &mut warnings)` -- 解码图片
- `PdfDocument::add_image(&raw)` -> `XObjectId` -- 添加图片
- `Op::UseXobject { id, transform }` -- 使用XObject
- `printpdf::XObjectTransform` -- 变换矩阵（translate/scale/rotate）

#### SVG API（image.rs）

- `printpdf::Svg::parse(svg_data, &mut warnings)` -- 解析 SVG
- `PdfDocument::add_xobject(&svg)` -> `XObjectId` -- 添加 SVG

#### 形状 API（shape.rs）

- `printpdf::Line` / `printpdf::LinePoint` -- 路径定义
- `printpdf::Point { x: Pt, y: Pt }` -- 坐标点
- `printpdf::Rect::from_wh(w, h)` -- 矩形
- `Op::SetOutlineThickness { pt }` -- 线宽
- `Op::DrawLine { line }` -- 绘制路径

#### 文档 API（writer.rs）

- `PdfDocument::save(&opts, &mut warnings)` -> `Vec<u8>` -- 保存到字节
- `PdfDocument::save_writer(writer, &opts, &mut warnings)` -- 保存到 Writer
- `PdfDocument::with_pages(pages)` -- 设置页面列表
- `PdfDocument::metadata.info` -- 文档元数据
- `PdfSaveOptions` -- 保存选项
- `Mm(f32)` / `Pt(f32)` -- 单位类型

#### HTML API（easypdf/src/html.rs，可选 feature）

- `printpdf::GeneratePdfOptions::default()` -- HTML 渲染选项
- `PdfDocument::from_html(html, &images, &fonts, &options, &mut warnings)` -- HTML 转 PDF
- `PdfDocument::save_writer(writer, &opts, &mut warnings)` -- 保存

### API 深度评估

easypdf-writer 对 printpdf 的使用是**深度耦合**的：

1. **类型依赖**：直接使用 printpdf 的 `Op`、`Point`、`Pt`、`Mm`、`Line`、`LinePoint`、`Rect`、`TextItem`、`PdfFontHandle`、`BuiltinFont`、`FontId`、`RawImage`、`XObjectTransform`、`Svg`、`PdfDocument`、`PdfPage`、`PdfSaveOptions` 等核心类型。
2. **操作依赖**：通过 `Vec<Op>` 构建页面内容，Op 是 printpdf 的 PDF 操作指令集。
3. **序列化依赖**：`SpilledPageData` 结构体序列化 `Vec<Op>`，backend.rs 的 spill 机制依赖 Op 的 Serialize/Deserialize。
4. **文档生命周期**：PdfDocument 的创建、页面添加、字体注册、图片添加、保存全流程。

---

## 弃用依赖深度分析

### bincode 1.3.3

| 属性 | 值 |
|------|-----|
| 状态 | unmaintained |
| RUSTSEC | RUSTSEC-2025-0141 |
| 日期 | 2025-12-16 |
| 传递路径 | printpdf -> azul-layout -> hyphenation -> bincode |
| 触发场景 | 使用 hyphenation（断字）功能时 |
| 直接影响 | 无（仅在 azul-layout 内部使用） |

**风险评估**：低。bincode 1.x 被标记为 unmaintained，但无已知安全漏洞。hyphenation 功能在 PDF 创建中很少被触发（仅用于自动断字排版）。azul-layout 是 printpdf 用于 HTML 布局的组件，常规 PDF 创建不经过此路径。

### rustybuzz 0.20.1

| 属性 | 值 |
|------|-----|
| 状态 | unmaintained |
| RUSTSEC | RUSTSEC-2026-0206 |
| 日期 | 2026-07-11 |
| 传递路径 | printpdf -> svg2pdf -> usvg -> rustybuzz |
| 触发场景 | 使用 SVG 渲染时 |
| 直接影响 | SVG 中的文本 shaping |

**风险评估**：低-中。rustybuzz 是 HarfBuzz 的 Rust 移植，用于 SVG 中的文本 shaping。仅在调用 `write_svg()` 时触发。对于不使用 SVG 功能的场景，此依赖不会被加载。rustybuzz 被标记为 unmaintained 但无已知安全漏洞。

### ttf-parser 0.25.1

| 属性 | 值 |
|------|-----|
| 状态 | unmaintained |
| RUSTSEC | RUSTSEC-2026-0192 |
| 日期 | 2026-06-28 |
| 传递路径 | printpdf -> svg2pdf -> ttf-parser / fontdb / usvg -> rustybuzz -> ttf-parser |
| 触发场景 | 使用 SVG 渲染或字体解析时 |
| 直接影响 | TrueType/OpenType 字体解析 |

**风险评估**：低-中。ttf-parser 是纯 Rust 字体解析库，被多个 SVG 相关依赖使用。仅在 SVG 功能或自定义字体路径中触发。无已知安全漏洞。

### lru 0.16.4

| 属性 | 值 |
|------|-----|
| 状态 | **unsound** |
| RUSTSEC | RUSTSEC-2026-0253 |
| 日期 | 2026-05-12 |
| 传递路径 | printpdf -> azul-layout -> lru |
| 触发场景 | azul-layout 内部缓存操作 |
| 直接影响 | 潜在 use-after-free |
| 修复版本 | >=0.18.2（当前最新 0.18.2） |

**风险评估**：中。这是一个 use-after-free 漏洞，源于 `LruCache::pop()` 缺乏 panic 安全性。当 `pop()` 内部发生 panic 时，可能导致已释放内存被访问。

**关键因素**：
1. 漏洞存在于 lru 0.16.4，修复版本为 0.18.2
2. lru 0.18.x 要求 MSRV 1.85.0（0.16.x 要求 1.70.0）
3. azul-layout 0.0.13 硬依赖 lru 0.16.x，无法通过 Cargo patch 升级
4. azul-layout 是 printpdf 的直接依赖，用于 HTML 布局
5. 触发条件需要 azul-layout 内部的 LruCache::pop() 发生 panic

**实际影响**：在常规 PDF 创建流程中（不使用 HTML 功能），azul-layout 可能不会被完全初始化或使用其 LRU 缓存。但 printpdf 的依赖图会拉入此 crate。

---

## 替代方案对比

### 方案 A：直接使用 lopdf（完全重写 PDF 创建层）

| 维度 | 评估 |
|------|------|
| 工作量 | 3-6 人月 |
| 风险 | 高 |
| 收益 | 彻底消除所有弃用依赖 |
| 可行性 | 项目已有 lopdf 依赖，template.rs 已直接使用 |

**详情**：lopdf 是一个底层 PDF 库，支持读取、修改和创建 PDF。但它不提供 printpdf 的高层抽象（Op 指令集、字体嵌入、图片处理、SVG 渲染）。需要手写：
- PDF 对象模型（xref、objects、streams）
- 字体嵌入和子集化（最复杂的部分）
- 图片编码和 XObject 创建
- 文本定位和渲染
- SVG 到 PDF 的转换

**结论**：不推荐作为首选方案。工作量大，PDF 创建比读取复杂得多。

### 方案 B：升级 printpdf 到更新版本

| 维度 | 评估 |
|------|------|
| 工作量 | 0（无新版本可用） |
| 风险 | N/A |
| 收益 | N/A |
| 可行性 | 不可行 |

**详情**：printpdf 0.12.5 已是 crates.io 上的最新版本（2024 年 12 月发布）。printpdf GitHub 有 11 个 open issues，但无关于升级弃用依赖的 issue 或 PR。

**结论**：此方案不可行，无新版本可升级。

### 方案 C：最小化 lopdf 方案（仅支持内置字体 + 文本）

| 维度 | 评估 |
|------|------|
| 工作量 | 4-8 人周 |
| 风险 | 中 |
| 收益 | 消除大部分弃用依赖，保留核心功能 |
| 可行性 | 中等 |

**详情**：使用 lopdf 直接创建 PDF，但限制功能范围：
- 仅支持内置 14 种 PDF 字体（Helvetica/Times/Courier/Symbol/ZapfDingbats）
- 仅支持文本和基本形状（线条、矩形）
- 不支持自定义字体嵌入、图片、SVG
- 保留 PDF 元数据、页面大小控制

**优点**：
- 消除 azul-layout（及其 lru/bincode 依赖）和 svg2pdf（及其 rustybuzz/ttf-parser 依赖）
- 工作量可控
- 保留最常用的 PDF 创建功能

**缺点**：
- 丢失图片和 SVG 支持
- 丢失自定义字体支持
- 需要维护两套 PDF 创建代码（printpdf 用于高级功能，lopdf 用于基本功能）

**结论**：可作为中期方案，但需要明确功能边界。

### 方案 D：接受现状 + 监控

| 维度 | 评估 |
|------|------|
| 工作量 | 0 |
| 风险 | 中（持续存在） |
| 收益 | 无 |
| 可行性 | 短期可行 |

**详情**：
- printpdf 0.12.5 在生产环境广泛使用
- lru 的 use-after-free 需要特定 panic 路径触发，实际利用难度高
- bincode/rustybuzz/ttf-parser 仅为 unmaintained 警告，无安全漏洞
- 持续跟进 printpdf 上游进展

**风险缓解**：
1. 监控 printpdf GitHub issues 和 releases
2. 监控 RUSTSEC advisory 更新
3. 定期运行 `cargo audit` 检查新增漏洞
4. 考虑在 Cargo.toml 中使用 `[patch]` 尝试覆盖 lru 版本（需要验证兼容性）

**结论**：可作为短期方案（3-6 个月），但需要设定监控指标和升级触发条件。

### 方案 E：Cargo patch 覆盖 lru 版本

| 维度 | 评估 |
|------|------|
| 工作量 | 1-2 人天 |
| 风险 | 中-高 |
| 收益 | 仅解决 lru unsound 问题 |
| 可行性 | 需要验证 |

**详情**：在 workspace Cargo.toml 中使用 `[patch.crates-io]` 覆盖 lru 版本：

```toml
[patch.crates-io]
lru = { version = "0.18.2" }
```

**风险**：
- lru 0.18.x 的 API 可能与 0.16.x 不兼容
- azul-layout 0.0.13 可能依赖 lru 0.16.x 的特定行为
- 需要验证 azul-layout 能否正常编译和运行

**结论**：值得尝试，但需要充分测试。如果编译通过且测试正常，这是最低成本的解决方案。

---

## 推荐决策

### 短期（立即）

**方案 D：接受现状 + 监控**

理由：
1. 无直接安全威胁（lru 的 use-after-free 需要特定条件触发）
2. printpdf 0.12.5 是最新版本，无升级路径
3. 项目仍在维护中（GitHub 有近期活动）

行动项：
- [ ] 在项目 README 中记录已知依赖问题
- [ ] 设置定期 `cargo audit` 检查（建议每月一次）
- [ ] 订阅 printpdf GitHub releases 和 issues 通知

### 中期（1-3 个月）

**方案 E：尝试 Cargo patch 覆盖 lru**

行动项：
- [ ] 在 workspace Cargo.toml 中添加 `[patch.crates-io]` 覆盖 lru 到 0.18.2
- [ ] 运行 `cargo build --workspace` 验证编译
- [ ] 运行 `cargo test --workspace` 验证功能
- [ ] 如果成功，提交 PR 并监控 CI

### 长期（3-6 个月）

**方案 C：最小化 lopdf 方案**

触发条件：
- printpdf 上游无进展（6 个月内无新版本）
- 出现新的安全漏洞且无修复路径
- 项目需要支持更多 PDF 功能（加密、签名等）

行动项：
- [ ] 评估 easypdf-writer 中哪些功能是核心需求
- [ ] 设计 lopdf-based 的最小 PDF 创建 API
- [ ] 实现基本文本和形状功能
- [ ] 保持 printpdf 作为可选高级功能（feature gate）

---

## 影响范围

### 功能影响矩阵

| 功能 | 依赖 printpdf | 依赖弃用 crate | 替代难度 |
|------|---------------|----------------|----------|
| 内置字体文本 | 是 | 否 | 低（lopdf 可直接支持） |
| 自定义字体 | 是 | 否 | 高（需要字体子集化） |
| 图片嵌入 | 是 | 否 | 中（lopdf 支持 XObject） |
| SVG 渲染 | 是 | 是（rustybuzz/ttf-parser） | 高（需要 SVG 解析器） |
| 形状绘制 | 是 | 否 | 低（lopdf 可直接支持） |
| HTML 转 PDF | 是 | 是（azul-layout/lru/bincode） | 高（需要 HTML 布局引擎） |
| PDF 元数据 | 是 | 否 | 低（lopdf 可直接支持） |
| 页面管理 | 是 | 否 | 低（lopdf 可直接支持） |

### 测试影响

如果迁移到 lopdf，以下测试需要重写：
- easypdf-writer/src/lib.rs 中的所有测试（约 30 个）
- easypdf-writer/src/backend.rs 中的 spill 测试（约 6 个）
- easypdf-writer/src/builder.rs 中的 builder 测试（约 5 个）
- easypdf/src/html.rs 中的 HTML 测试（如果保留此功能）

### 维护成本评估

| 方案 | 初始成本 | 持续成本 | 总成本（1年） |
|------|----------|----------|---------------|
| 接受现状 | 0 | 低（监控） | 低 |
| Cargo patch | 1-2 天 | 低（验证兼容性） | 低 |
| 最小化 lopdf | 4-8 周 | 中（维护两套代码） | 中 |
| 完全重写 | 3-6 月 | 低（单一代码库） | 高 |

---

## 附录

### 依赖传递路径图

```
printpdf 0.12.5
├── azul-layout 0.0.13
│   ├── hyphenation 0.8.4
│   │   └── bincode 1.3.3          [unmaintained]
│   └── lru 0.16.4                 [unsound: use-after-free]
├── svg2pdf 0.13.0
│   ├── usvg 0.45.1
│   │   └── rustybuzz 0.20.1       [unmaintained]
│   └── ttf-parser 0.25.1          [unmaintained]
└── lopdf 0.44.0                   [正常，无问题]
```

### RUSTSEC 汇总

| Crate | RUSTSEC | 状态 | 严重度 | 修复版本 |
|-------|---------|------|--------|----------|
| bincode 1.3.3 | RUSTSEC-2025-0141 | unmaintained | 警告 | N/A |
| rustybuzz 0.20.1 | RUSTSEC-2026-0206 | unmaintained | 警告 | N/A |
| ttf-parser 0.25.1 | RUSTSEC-2026-0192 | unmaintained | 警告 | N/A |
| lru 0.16.4 | RUSTSEC-2026-0253 | unsound | 中 | >=0.18.2 |

### printpdf 上游状态

- 最新版本：0.12.5（2024-12-09）
- GitHub open issues：11
- 最近活动：2025 年 11 月（issue 提交）
- 弃用依赖 issue：无
- 维护状态：活跃但更新频率低

### 参考项目

- **lopdf**（https://github.com/J-F-Liu/lopdf）：纯 Rust PDF 库，无弃用依赖
- **genpdf**（https://github.com/estokes/genpdf）：基于 lopdf 的高层 PDF 生成器，但依赖类似
- **justpdf**：实验性 PDF 库，功能不完整

---

## 变更摘要

本次评估为纯分析文档，不涉及代码变更。

**关键发现**：
1. printpdf 0.12.5 是最新版本，无升级路径
2. lru 0.16.4 的 use-after-free 是唯一的安全问题，但触发条件苛刻
3. easypdf-writer 对 printpdf 的使用深度耦合，迁移工作量大
4. 项目已有 lopdf 依赖和使用经验（template.rs），为未来迁移提供基础
5. Cargo patch 覆盖 lru 版本值得尝试，可能以最小成本解决核心问题
