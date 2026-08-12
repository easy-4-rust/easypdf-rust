# v0.1.0-alpha.2 丰富内容与安全能力 实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: rust-testing, rust-code-review

**Goal:** 基于原 `docs/implementation-plan.md`（已并入本 plan，见末尾附录）的 F1-F16 功能编号和 C1/C2 架构改进，实现表格渲染、图片插入、自定义字体、矢量图形、页眉页脚、多页写入器、水印、加密、权限、数字签名、HTML→PDF、SVG→PDF、Markdown→PDF、布局引擎等全部功能。

**Architecture:** 复用 Phase 1 的 crate 分层。加密/签名逻辑放在 `easypdf-core::crypto` 模块。HTML→PDF 通过 printpdf 的 `from_html()` 实现（需 Chromium）。SVG 通过 printpdf 的 `Svg::parse()` 实现。

**Tech Stack:** printpdf 0.8（图片/字体/形状/HTML/SVG）, lopdf 0.34（水印注入/图层）, aes crate（加密）, ring 0.17（签名 RSA）, x509-parser（证书解析）。

## Global Constraints

- 加密实现必须符合 ISO 32000 规范（AES-128 V4/R4 / AES-256 V5/R6）。
- 签名使用 PKCS#7/CMS detached SignedData，RSA-PKCS#1v1.5 + SHA-256。
- HTML→PDF 功能通过 feature gate 控制（需系统 Chromium）。
- 每个功能有独立测试，不依赖外部服务。

---

### Task 1: Tables 表格渲染

> Files:
> - Modify: `crates/easypdf/src/writer_helpers.rs`
> - Modify: `crates/easypdf/src/builders.rs`
> - Modify: `crates/easypdf-core/src/content.rs`

**Steps:**
- [x] 扩展 `PdfTable` -- 添加 `column_widths` 自动计算、`row_height` 默认值
- [x] 新增 `TableRenderConfig` -- 边框样式映射到 printpdf Op
- [x] 实现 `PdfWriter::write_table()` -- 遍历 rows/columns，画线+写文本
- [x] 新增 `PdfTableBuilder<T>` Builder -- headers_from + data + position
- [x] 验证表格渲染测试通过

---

### Task 2: Images 图片

> Files:
> - Create: `crates/easypdf-writer/src/image.rs`
> - Modify: `crates/easypdf/src/builders.rs`

**Steps:**
- [x] 实现 `PdfWriter::write_image(&self, img, x, y, w, h)` -- 调用 `RawImage::decode_from_bytes`
- [x] `PdfImage` 添加 `from_path(path)` / `from_bytes(bytes, format)` 构造函数
- [x] 新增 `PdfImageBuilder` -- `.add_image(path)` → `.position(x, y)` → `.size(w, h)`
- [x] 验证图片插入测试通过

---

### Task 3: Custom Fonts 自定义字体 (TTF/OTF)

> Files:
> - Modify: `crates/easypdf-writer/src/writer.rs`
> - Modify: `crates/easypdf-core/src/content.rs`

**Steps:**
- [x] 实现 `PdfWriter::register_font_from_path(path)` -- 读文件 → `ParsedFont::from_bytes` → `doc.add_font`
- [x] 实现 `PdfWriter::register_font_from_bytes(key, font_data)` -- 从内存注册
- [x] 实现 `PdfWriter::write_text_with_custom_font(text, font_key, size, x, y)`
- [x] 修改 `FontFamily` -- 支持 `Custom(Cow<'static, str>)` 变体
- [x] 验证自定义字体测试通过

---

### Task 4: Shapes 矢量图形

> Files:
> - Create: `crates/easypdf-writer/src/shape.rs`

**Steps:**
- [x] 实现 `draw_line(x1, y1, x2, y2, line_width)` -- 构建 `Line` → `Op::DrawLine`
- [x] 实现 `draw_rect_stroke(x, y, w, h, line_width)` -- `Rect::to_line()` + `DrawLine`
- [x] 实现 `draw_circle(cx, cy, radius, line_width)` -- 4 段三次贝塞尔曲线近似（k = 0.5522847498）
- [x] 验证矢量图形测试通过

---

### Task 5: Headers & Footers 页眉页脚

> Files:
> - Modify: `crates/easypdf/src/handlers.rs`

**Steps:**
- [x] 实现 `PageNumberHandler` / `TextHeaderHandler` / `TextFooterHandler` 内置处理器
- [x] `PdfWriteHandler` 的 `after_page` 回调注入页眉/页脚
- [x] 验证页眉页脚测试通过

---

### Task 6: Multi-page Writer 多页写入器

> Files:
> - Modify: `crates/easypdf-writer/src/writer.rs`

**Steps:**
- [x] 维护 `Vec<PdfPage>` 累积所有页面
- [x] `add_page()` 中将当前 ops 转为 PdfPage 推入 `pages`
- [x] `finish()` 中调用 `doc.with_pages(pages)`
- [x] 新增 `current_page_number()` 方法
- [x] 验证多页写入测试通过

---

### Task 7: Watermarks 水印

> Files:
> - Modify: `crates/easypdf/src/watermark.rs`

**Steps:**
- [x] 实现文本水印 -- 遍历页面，构建 BT/ET 文本块（半透明、居中、旋转 45 度）
- [x] 实现图片水印 -- 注入 XObject，在内容流中 `Do` 引用
- [x] 实现 `PdfWatermarkBuilder` -- opacity / rotation / color 配置
- [x] 验证水印测试通过

---

### Task 8: Encryption 加密

> Files:
> - Create: `crates/easypdf-core/src/crypto/encrypt.rs`
> - Create: `crates/easypdf/src/crypto_facade.rs`

**Steps:**
- [x] 实现 AES-128-CBC 加密（V4/R4）-- 128 位文件加密密钥
- [x] 实现 AES-256-CBC 加密（V5/R6）-- 256 位文件加密密钥（ISO 32000-2）
- [x] 实现 `PdfEncryption` Builder -- user_password / owner_password / algorithm / permissions
- [x] 实现 `encrypt_pdf()` / `decrypt_pdf()` 函数
- [x] 实现 `EasyPdf::encrypt(password)` 门面方法
- [x] 验证加密/解密测试通过

---

### Task 9: Permission Flags 权限标志

> Files:
> - Modify: `crates/easypdf-core/src/crypto/encrypt.rs`

**Steps:**
- [x] 定义 `PdfPermissions` -- PRINT / MODIFY / COPY / FILL_FORMS / EXTRACT / ASSEMBLE / PRINT_HIGH
- [x] 加密时设置 `/P` 权限位掩码
- [x] Builder 方法 `allow_printing()` / `deny_modification()` 等
- [x] 验证权限测试通过

---

### Task 10: Digital Signatures 数字签名

> Files:
> - Create: `crates/easypdf-core/src/crypto/sign.rs`
> - Create: `crates/easypdf-core/src/crypto/sign_cms.rs`
> - Create: `crates/easypdf-core/src/crypto/sign_der.rs`
> - Create: `crates/easypdf-core/src/crypto/sign_pdf.rs`
> - Modify: `crates/easypdf/src/crypto_facade.rs`

**Steps:**
- [x] 实现 PKCS#7/CMS detached SignedData -- RSA-PKCS#1v1.5 + SHA-256
- [x] 使用 ring 0.17 常量时间 RSA（修复 Marvin Attack / RUSTSEC-2023-0071）
- [x] 使用 x509-parser 解析 X.509 证书
- [x] 实现 `PdfSigner` Builder -- certificate / private_key / reason / location
- [x] 实现 `sign_pdf()` / `verify_pdf_signature()` 函数
- [x] 实现 `EasyPdf::sign(signer)` 门面方法
- [x] 验证签名/验证测试通过

---

### Task 11: HTML → PDF

> Files:
> - Modify: `crates/easypdf/src/lib.rs`
> - Modify: `crates/easypdf/src/html.rs`

**Steps:**
- [x] 实现 `EasyPdf::from_html(html)` -- 调用 `printpdf::PdfDocument::from_html()`
- [x] 实现 `HtmlToPdfBuilder` -- page_size / margins / chromium_path 配置
- [x] feature gate: `html` feature，默认不启用（需系统 Chromium）
- [x] 验证 HTML→PDF 测试通过

---

### Task 12: Markdown → PDF

> Files:
> - Modify: `crates/easypdf/src/lib.rs`

**Steps:**
- [x] 实现 `EasyPdf::from_markdown(md)` -- Markdown → HTML → PDF 两阶段转换
- [x] 使用 pulldown-cmark 转 HTML，再调用 `PdfDocument::from_html()`
- [x] 验证 Markdown→PDF 测试通过

---

### Task 13: Layout Engine 自动布局引擎

> Files:
> - Create: `crates/easypdf-core/src/layout.rs`

**Steps:**
- [x] 定义 `FlowLayout` -- direction / margins / spacing / cursor
- [x] 实现 `LayoutSink` trait -- 后端无关布局抽象
- [x] 实现 `next_position()` / `remaining_space()` / `new_page()`
- [x] 验证布局引擎测试通过

---

## Acceptance / Verification

```bash
cargo test --workspace                    # 105+ tests pass
cargo clippy --workspace -- -D warnings   # 0 warnings
cargo fmt --check                         # 100% compliant
```

## 关键发现（代码核对）

- F8 PDF Layers（OCG）：代码中未找到 OCG 实现，用户需求量低、实现复杂度高，已推迟。
- F11 PDF/A Validation：`easypdf-reader/src/manipulate.rs` 有 `validate_pdfa()` 基础方法（检查加密和 XMP），但完整校验未实现。
- F12 XMP Metadata：未找到 XMP 生成代码，roadmap 标注为 0.5 Compliance 阶段。
- F15 SVG→PDF：未找到 `Svg::parse` 或 `add_svg` 调用，roadmap 标注为 0.6 Converters 阶段。
- C1 Engine Abstraction：未找到 `PdfEngine` trait，已推迟（当前只有一组有效引擎）。
- 以上 5 项均正确标为 `[ ]`，移入对应未来版本 plan。

## 依赖关系

```
Task 1 (Tables)
    │
    ├── Task 2 (Images)
    ├── Task 3 (Fonts)
    ├── Task 4 (Shapes)
    ├── Task 5 (Headers/Footers)
    ├── Task 6 (Multi-page)
    ├── Task 7 (Watermarks)
    │
    ├── Task 8 (Encryption)
    │       │
    │       └── Task 9 (Permissions)
    │
    ├── Task 10 (Signatures)
    ├── Task 11 (HTML→PDF)
    ├── Task 12 (Markdown→PDF)
    └── Task 13 (Layout Engine)
```

---

## 附：历史规划素材（来自 implementation-plan.md）

> 以下内容从原 `docs/implementation-plan.md`（已并入本 plan）中提取，保留历史规划视角。

### Engine Capability Summary 引擎能力摘要

| Feature | printpdf v0.8 | lopdf v0.34 | 第三方方案 |
|:---|:---:|:---:|:---|
| Tables | 无原生支持 | 无 | 用 Line + Text Op 自建 |
| Images from bytes | `RawImage::decode_from_bytes` | 无 | -- |
| Custom TTF/OTF | `ParsedFont::from_bytes` | 无 | -- |
| Lines | `Op::DrawLine` | 原始 PDF 算子 | -- |
| Rectangles | `rect.to_polygon()` / `rect.to_line()` | 原始 PDF 算子 | -- |
| Circles/Ellipses | 无 Op | 贝塞尔曲线 | -- |
| Fill/Stroke control | 丰富 | 原始算子 | -- |
| SVG | `Svg::parse` -> XObject | 无 | usvg + svg2pdf |
| Watermark overlay | 仅生成 | `add_page_contents` | -- |
| Encryption | `PdfSaveOptions.secure` 为空 | 仅解密 | `aes` + 自建 |
| PDF/A | 无 | 无 | `pdf-a` 或自建校验 |
| Digital signatures | 无 | 无 | `rsa` + 自建 |
| HTML->PDF | 无 | 无 | headless chrome / `printpdf::from_html` |

### Implementation Priority Matrix 优先级矩阵

按**用户价值 x 实现成本**排序：

| Priority | Feature | Phase | Size | Value | Cost | Score |
|:---:|:---|:---:|:---:|:---:|:---:|:---:|
| 1 | F6 Multi-page Writer | v0.2 | S | 最高 | S | **最高** |
| 2 | F2 Images | v0.2 | S | 最高 | S | **最高** |
| 3 | F3 Custom Fonts | v0.2 | M | 高 | M | 高 |
| 4 | F4 Shapes (line/rect) | v0.2 | M | 中 | M | 中 |
| 5 | F15 SVG->PDF | v0.6 | S | 中 | S | 中 |
| 6 | F5 Headers/Footers | v0.2 | M | 高 | M | 中 |
| 7 | F1 Tables | v0.2 | L | 最高 | L | 中 |
| 8 | F12 XMP Metadata | v0.5 | S | 低 | S | 低 |
| 9 | F7 Watermarks | v0.3 | M | 中 | M | 低 |
| 10 | F14 HTML->PDF | v0.6 | L | 中 | L | 低 |
| 11 | F16 Markdown->PDF | v0.6 | M | 低 | M | 低 |
| 12 | F9 Encryption | v0.4 | L | 高 | L | 推迟 |
| 13 | F10 Permissions | v0.4 | S | 低 | S | 推迟 |
| 14 | F4 Circle/Ellipse | v0.2 | M | 低 | M | 推迟 |
| 15 | F8 PDF Layers | v0.3 | L | 极低 | L | 推迟 |
| 16 | F11 PDF/A | v0.5 | XL | 低 | XL | 推迟 |
| 17 | F13 Digital Sig. | v0.5 | XL | 低 | XL | 推迟 |

### v0.2 Recommended Sprint Plan v0.2 推荐冲刺计划

```
Sprint 1 (1 week):
  Day 1-2: F6 Multi-page Writer (S)    <- 修复 v0.1 核心缺陷
  Day 3-4: F2 Images (S)               <- 高频需求
  Day 5:   F3 Custom Fonts (M) 开始

Sprint 2 (1 week):
  Day 1-3: F3 Custom Fonts (M) 完成
  Day 4-5: F4 Shapes line + rect (M)

Sprint 3 (1 week):
  Day 1-3: F5 Headers/Footers (M)
  Day 4-5: F1 Tables basic (L) 开始

Sprint 4 (2 weeks):
  Day 1-7: F1 Tables 完成 (L)
  Day 8-10: F15 SVG->PDF (S) -- 快赢功能
```

### Risk Register 风险登记册

| Risk | Likelihood | Impact | Mitigation |
|:---|:---:|:---:|:---|
| printpdf API 大版本变更 | 中 | 高 | 锁版本，关注 changelog，提前适配 |
| lopdf API 大版本变更 | 中 | 中 | 同上 |
| Chromium 依赖（HTML->PDF） | 高 | 中 | 提供 feature gate，默认不启用 |
| 字体子集化复杂度（TTF） | 中 | 低 | v0.2 嵌入完整字体，后续子集化 |
| 加密实现安全性 | 高 | 高 | 参考成熟实现（ms-offcrypto-writer），加入安全审计 |
| PDF 规范兼容性 | 中 | 中 | 用真实 PDF reader 验证输出（Adobe Acrobat 等） |
