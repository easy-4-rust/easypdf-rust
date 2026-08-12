# easypdf-rust Version Plan

> 版本规划、发布节奏与质量门禁。

---

## 版本 → 阶段映射表

| 版本 | 状态 | 对应 roadmap 阶段 | 规划来源 | 发布日期 |
|---|---|---|---|---|
| **v0.1.0** | 已发布 | 0.1 Foundation + 0.2 Architecture Consolidation + 部分 0.3 Rich Content + 部分 0.4 Security | `plans/2026-07-21-easypdf-0.1-foundation.md` + `plans/2026-07-21-easypdf-0.2-rich-content.md` + `plans/2026-08-09-easypdf-0.3-architecture-consolidation.md` | 2026-08-12 |
| **v0.2.0** | 规划中 | 0.3 Rich Content 剩余 + 0.4 OCR 端到端 + 0.5 XMP + 0.6 SVG→PDF | `plans/2026-08-12-easypdf-0.4-rich-content-next.md` | TBD |
| **v0.3.0** | 规划中 | 0.5 Compliance（PDF/A）+ 0.6 PDF→image + 0.6 Markdown→PDF 优化 | `plans/2026-08-12-easypdf-0.4-rich-content-next.md` | TBD |
| **v1.0.0** | 计划中 | 1.0 Stable | `plans/2026-08-12-easypdf-0.4-rich-content-next.md` | TBD |

---

## v0.1.0（已发布 2026-08-12）

### 覆盖范围

- **0.1 Foundation**（22 项全部完成）: EasyPdf 门面 / derive 宏 / 读写/合并/拆分/旋转/重排 / Markdown 转换 / 表单填充 / 资源限制 / 事件系统 / 原子输出
- **0.2 Architecture Consolidation**（19 项全部完成）: 22→9 合并 / Streaming / CMap / WriteBackend / WriteHandlerChain / ConverterRegistry / ProcessorPipeline / 4 云 OCR / MCP + resident / ISO 32000 加密签名 / tracing / fuzz / 91.61% 覆盖率 / 发布
- **0.3 Rich Content**（1/6 完成）: add_table Builder API
- **0.4 Security**（3/4 完成）: AES-256 加密 / 密码保护 / 权限标志

### 质量指标

| 指标 | 值 |
|---|---|
| 测试通过 | 1522 |
| 行覆盖率 | 91.61% |
| Cargo audit CVE | 0 |
| Clippy 警告 | 0 |
| Rustdoc 警告 | 0 |
| Fuzz targets | 6 |
| crates.io 发布 | 8 crate（easypdf-test 不发布） |
| Workspace crate | 9（从 22 合并） |

### 设计文档

- `specs/2026-08-09-crate-consolidation-22-to-9-design.md`
- `specs/2026-08-09-iso-32000-crypto-design.md`
- `specs/2026-08-09-cloud-ocr-unified-design.md`

---

## v0.2.0（下一版本，目标）

### 目标内容

从 `plans/2026-08-12-easypdf-0.4-rich-content-next.md` 中选取以下任务：

**0.3 Rich Content 剩余 5 项:**
1. Table border style enhancements（斑马纹 / 自定义边框 / 合并单元格）
2. Image insertion enhancement（JPEG/PNG 尺寸和位置控制 / 缩放）
3. Vector shapes enhancement（填充控制 / 椭圆 / 多边形 / 路径）
4. Custom TTF/OTF font registration（字体子集化 / 缓存 / 度量）
5. Multi-page auto page breaks（FlowLayout 自动分页 / 跨页表格）

**0.4 Security 剩余 1 项:**
6. PDF to Markdown OCR real integration（端到端接通）

**0.6 Converters 部分:**
7. SVG to PDF（svg2pdf 集成）

**0.5 Compliance 部分:**
8. XMP metadata（XMP XML 生成 / /Metadata stream）

### 质量门禁

- 所有新增功能有独立测试
- 现有 1522 测试不回归
- clippy 零警告
- 新增 fuzz target（如 SVG 解析）
- 文档更新（使用指南 / API 文档）

### 预计规模

- 新增约 8 个功能点
- 预计新增约 200-400 行核心代码
- 预计新增约 50-100 个测试

---

## v0.3.0（规划中）

### 目标内容

从 `plans/2026-08-12-easypdf-0.4-rich-content-next.md` 中选取以下任务：

**0.5 Compliance 剩余:**
1. PDF/A-1b validation
2. PDF/A-2b validation
3. PDF/A-3b validation
4. Document info dictionary standardization

**0.6 Converters 剩余:**
5. PDF to image rasterize（页面光栅化）
6. Markdown to PDF optimization（性能 / CSS 注入 / 代码高亮）

### 质量门禁

- PDF/A 校验通过标准测试用例
- PDF→image 输出可被标准图片查看器打开
- 现有测试不回归
- clippy 零警告

---

## v1.0.0（计划中）

### 目标内容

从 `plans/2026-08-12-easypdf-0.4-rich-content-next.md` 中选取以下任务：

1. Semver guarantees（cargo-semver-checks 验证）
2. Windows MSRV testing（CI 矩阵 + 修复）
3. Property-based testing（proptest for crypto / parse / markdown）
4. Complete migration guide

### 质量门禁

- cargo-semver-checks 零违反
- Windows CI 通过（windows-latest + rust 1.88）
- 属性测试覆盖率 > 80% 公共 API
- 迁移指南覆盖所有 breaking changes
- 1522+ 测试通过
- 91.61%+ 行覆盖率
- 0 CVE
- clippy 零警告
- 6+ fuzz targets

---

## 发布节奏

| 阶段 | 节奏 | 原则 |
|---|---|---|
| v0.1.x → v0.2.x | 功能驱动，完成后发布 | 不定时，按功能完成度 |
| v0.2.x → v0.3.x | 功能驱动 | 同上 |
| v0.3.x → v1.0 | 质量驱动 | 所有门禁通过后发布 |
| v1.0+ | semver 严格 | breaking change 只在 major version |

### Patch 版本（v0.1.x）

- Bug 修复
- 安全补丁
- 文档修正
- 不改变公共 API

### Minor 版本（v0.x.0）

- 新增功能
- 新增 feature gate
- 可能新增 crate（如 easypdf-render）
- 保持向后兼容

---

## 质量门禁总览

| 门禁 | v0.1.0 | v0.2.0 | v0.3.0 | v1.0.0 |
|---|---|---|---|---|
| 测试通过 | 1522 | 1700+ | 1900+ | 2000+ |
| 行覆盖率 | 91.61% | 91%+ | 91%+ | 91%+ |
| Cargo audit CVE | 0 | 0 | 0 | 0 |
| Clippy 警告 | 0 | 0 | 0 | 0 |
| Rustdoc 警告 | 0 | 0 | 0 | 0 |
| Fuzz targets | 6 | 7+ | 8+ | 8+ |
| semver-checks | N/A | N/A | N/A | 通过 |
| Windows CI | N/A | N/A | 通过 | 通过 |
| 属性测试 | N/A | N/A | N/A | 80%+ API |
| 迁移指南 | N/A | N/A | N/A | 完整 |

---

## 相关文档

- Roadmap: `docs/roadmap.md`
- 实施计划历史: `docs/implementation-plan.md`
- 架构文档: `docs/easypdf-rust-Architecture.md`
- 项目事实: `docs/PROJECT_FACTS.md`
- CHANGELOG: `CHANGELOG.md`
- 发布日志: `docs/RELEASE_LOG_0.1.0.md`
- 安全审计: `docs/security/AUDIT.md`
- 性能基准: `docs/performance/BENCHMARK.md`
