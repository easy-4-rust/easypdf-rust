# easypdf-rust 输入安全模型设计

- **日期**：2026-08-09
- **作者**：ZCode Agent（协同设计）
- **状态**：已实现（v0.1.0 发布）
- **依赖**：easypdf-core `io/ssrf_guard.rs`、`crypto/`（encrypt / sign / sign_cms / sign_der / sign_pdf）、easypdf-ocr `http/auth.rs`、easypdf-reader `reader/`

## 1. 目标与范围

为 easypdf-rust 建立**全面的输入安全模型**，覆盖 SSRF 防护（含 IPv6）、解压炸弹防护、资源限制、API key Debug 脱敏、加密签名安全（ring 常量时间 RSA）。

**核心需求**：

1. SSRF 防护覆盖 IPv6（loopback / ULA / link-local / IPv4-mapped）。
2. 解压炸弹防护（绝对解压大小检查，无豁免）。
3. 资源限制：文件大小、页面数量、文本长度。
4. API key 在 Debug 输出中脱敏。
5. 加密使用 AES-128/256-CBC（ISO 32000）。
6. 签名使用 ring 常量时间 RSA（修复 Marvin Attack / RUSTSEC-2023-0071）。
7. CMS SignedData 规范合规（不再双重哈希）。

**非目标**：

- 不实现 RSA 密钥生成（使用 rsa crate 的 dev-dependency）。
- 不实现 X.509 证书生成（使用测试证书）。
- 不实现 PKCS#12 密钥库解析。
- 不实现 OCSP/CRL 证书吊销检查。
- 不实现时间戳协议（TSA）。
- 不实现 PDF 增量保存。

## 2. 总体架构

```
┌──────────────────────────────────────────────────────────────┐
│                    输入安全模型                                │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐  │
│  │  SSRF 防护 (easypdf-core::io::ssrf_guard)             │  │
│  │  ├── validate_url(url)                                │  │
│  │  │   ├── 禁止 file:// / ftp://                        │  │
│  │  │   ├── 禁止私有 IP (10.x / 172.16-31.x / 192.168.x)│  │
│  │  │   ├── 禁止 loopback (127.x / ::1)                 │  │
│  │  │   ├── 禁止 IPv6 link-local (fe80::)               │  │
│  │  │   ├── 禁止 IPv6 ULA (fc00:: / fd00::)             │  │
│  │  │   └── 禁止 IPv4-mapped (::ffff:127.0.0.1)         │  │
│  │  └── validate_url_for_ocr(url)                        │  │
│  └────────────────────────────────────────────────────────┘  │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐  │
│  │  解压炸弹防护 (easypdf-reader)                         │  │
│  │  ├── 检查绝对解压大小（无 64KB 豁免）                  │  │
│  │  ├── 解压后检查文件大小                                │  │
│  │  └── 超过阈值拒绝                                      │  │
│  └────────────────────────────────────────────────────────┘  │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐  │
│  │  资源限制 (easypdf-reader)                             │  │
│  │  ├── 文件大小限制                                      │  │
│  │  ├── 页面数量限制                                      │  │
│  │  └── 文本长度限制                                      │  │
│  └────────────────────────────────────────────────────────┘  │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐  │
│  │  API Key 脱敏 (easypdf-ocr)                           │  │
│  │  ├── GlmConfig::Debug → api_key: "[REDACTED]"         │  │
│  │  ├── BaiduConfig::Debug → secret_key: "[REDACTED]"    │  │
│  │  └── HunyuanConfig::Debug → secret_key: "[REDACTED]"  │  │
│  └────────────────────────────────────────────────────────┘  │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐  │
│  │  加密 (easypdf-core::crypto::encrypt)                  │  │
│  │  ├── AES-128-CBC (V4/R4) 128-bit key                  │  │
│  │  ├── AES-256-CBC (V5/R6) 256-bit key (ISO 32000-2)    │  │
│  │  ├── PdfEncryption Builder                             │  │
│  │  │   ├── user_password / owner_password                │  │
│  │  │   ├── algorithm (Aes128 / Aes256)                   │  │
│  │  │   └── permissions (PdfPermissions)                  │  │
│  │  ├── encrypt_pdf() / decrypt_pdf()                     │  │
│  │  └── PdfPermissions (PRINT/MODIFY/COPY/FILL_FORMS...)  │  │
│  └────────────────────────────────────────────────────────┘  │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐  │
│  │  签名 (easypdf-core::crypto::sign*)                    │  │
│  │  ├── sign.rs        RSA-PKCS#1v1.5 + SHA-256          │  │
│  │  ├── sign_cms.rs    CMS SignedData 构建/解析           │  │
│  │  ├── sign_der.rs    DER 编码器/解码器                  │  │
│  │  ├── sign_pdf.rs    PDF 签名集成                       │  │
│  │  │   ├── sign_pdf()                                    │  │
│  │  │   └── verify_pdf_signature()                        │  │
│  │  └── ring 迁移     常量时间 RSA（修复 Marvin Attack）  │  │
│  └────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────┘
```

## 3. 模块职责划分

### 3.1 `ssrf_guard.rs` — SSRF 防护

| 函数 | 职责 |
|---|---|
| `validate_url(url)` | 通用 URL 校验（拒绝私有/loopback/非 HTTP） |
| `validate_url_for_ocr(url)` | OCR 专用 URL 校验（同上，错误信息更详细） |

**IPv6 覆盖清单**：

| 地址类型 | 示例 | 状态 |
|---|---|---|
| IPv6 loopback | `::1` | 禁止 |
| IPv6 link-local | `fe80::1` | 禁止 |
| IPv6 ULA | `fc00::1` / `fd00::1` | 禁止 |
| IPv4-mapped IPv6 | `::ffff:127.0.0.1` | 禁止 |
| IPv6 全局单播 | `2001:db8::1` | 允许 |

### 3.2 解压炸弹防护

**设计要点**：
- 移除 64KB 豁免（旧版本有豁免，导致绕过）
- 检查绝对解压大小（非压缩比）
- 超过阈值（默认 100MB）拒绝
- 在解压过程中持续检查（不等解压完成）

### 3.3 资源限制

| 限制 | 默认值 | 位置 |
|---|---|---|
| 文件大小 | 100MB | `PdfReader::open()` |
| 页面数量 | 10000 | `PdfReader::extract_text()` |
| 文本长度 | 10MB | `PdfReader::extract_text()` |

### 3.4 API Key 脱敏

```rust
impl fmt::Debug for GlmConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GlmConfig")
            .field("base_url", &self.base_url)
            .field("api_key", &"[REDACTED]")
            .finish()
    }
}
```

**覆盖结构体**：GlmConfig / BaiduConfig / HunyuanConfig / DeepSeekConfig

### 3.5 加密模块 (`encrypt.rs`)

| 算法 | PDF 版本 | 密钥长度 | 实现 |
|---|---|---|---|
| AES-128-CBC | V4/R4 (PDF 1.6) | 128-bit | aes + cbc crate |
| AES-256-CBC | V5/R6 (PDF 2.0) | 256-bit | aes + cbc crate |

**加密流程**：
1. 生成随机文件加密密钥
2. 用 AES-CBC 加密所有 stream 和 string 对象
3. 用用户密码加密文件加密密钥（SHA-256 → AES wrap）
4. 构建 `/Encrypt` 字典（/V, /R, /O, /U, /P, /Filter）

**权限位掩码**：
```rust
pub struct PdfPermissions {
    pub print: bool,           // Bit 3
    pub modify: bool,          // Bit 4
    pub copy: bool,            // Bit 5
    pub annotate: bool,        // Bit 6
    pub fill_forms: bool,      // Bit 9
    pub extract: bool,         // Bit 10
    pub assemble: bool,        // Bit 11
    pub print_high: bool,      // Bit 12
}
```

### 3.6 签名模块 (`sign*.rs`)

| 模块 | 职责 |
|---|---|
| `sign.rs` | RSA-PKCS#1v1.5 + SHA-256 签名/验证 |
| `sign_cms.rs` | CMS SignedData 构建/解析 |
| `sign_der.rs` | DER 编码器/解码器（手动实现，最小化依赖） |
| `sign_pdf.rs` | PDF 签名集成（/ByteRange + /Contents） |

**ring 迁移**：
- 问题：`rsa` crate（RUSTSEC-2023-0071）的 RSA PKCS#1v1.5 存在 Marvin Attack 时间侧信道
- 方案：生产代码路径使用 `ring::signature::RSA_PKCS1_SHA256`
- ring 使用 blinding 和常量时间操作，天然抵抗时间侧信道
- `rsa` 保留为 dev-dependency，仅用于测试证书生成

**双重哈希修复**：
- 问题：签名时先 SHA-256 哈希再 RSA 签名（`RSA(SHA-256(data))`），而 CMS 规范要求直接 RSA 签名（`RSA(data)`）
- 方案：签名前不再先哈希，由 CMS 内部处理

## 4. 关键数据流

### 4.1 SSRF 防护流程

```
用户传入 URL
    │
    ▼
ssrf_guard::validate_url(url)
    │
    ├── 解析 URL（scheme / host / port）
    ├── 检查 scheme（仅允许 http / https）
    ├── 解析 host 为 IP 地址
    ├── 检查 IP 类型
    │   ├── IPv4 loopback (127.x) → 拒绝
    │   ├── IPv4 私有 (10.x / 172.16-31.x / 192.168.x) → 拒绝
    │   ├── IPv6 loopback (::1) → 拒绝
    │   ├── IPv6 link-local (fe80::) → 拒绝
    │   ├── IPv6 ULA (fc00:: / fd00::) → 拒绝
    │   ├── IPv4-mapped (::ffff:127.0.0.1) → 拒绝
    │   └── 其他 → 允许
    └── 返回 Result<()>
```

### 4.2 加密/签名完整流程

```
EasyPdf::encrypt(password)
    │
    ▼
PdfEncryption::builder()
    .user_password(password)
    .algorithm(PdfEncryptionAlgorithm::Aes256)
    .permissions(PdfPermissions { print: true, .. })
    .build()
    │
    ▼
encrypt_pdf(pdf_bytes, &encryption)
    │
    ├── 生成随机 256-bit 文件加密密钥
    ├── AES-256-CBC 加密所有 stream/string
    ├── SHA-256(password) → AES wrap 文件加密密钥
    └── 构建 /Encrypt 字典 → 添加到 trailer
    │
    ▼
加密后的 PDF 字节

EasyPdf::sign(signer)
    │
    ▼
PdfSigner::builder()
    .certificate(cert)
    .private_key(key)
    .reason("Approval")
    .build()
    │
    ▼
sign_pdf(pdf_bytes, &signer)
    │
    ├── 预留 /ByteRange + /Contents 占位
    ├── 计算 /ByteRange 范围的 SHA-256 摘要
    ├── ring RSA 签名（常量时间，不再先哈希）
    ├── 构建 CMS SignedData（含 X.509 证书）
    └── 写回签名
    │
    ▼
签名后的 PDF 字节
```

## 5. 技术决策与权衡

| # | 决策 | 理由 | 权衡 |
|---|---|---|---|
| 1 | SSRF 覆盖 IPv6 全类型 | 防止 IPv6 绕过 | 解析复杂度增加 |
| 2 | 解压炸弹无豁免 | 防止 64KB 豁免被绕过 | 小文件也检查 |
| 3 | ring 替代 rsa（生产路径） | 修复 Marvin Attack | ring 无 keygen API |
| 4 | DER 手动实现 | 最小化依赖 | 实现复杂、需充分测试 |
| 5 | 权限用 struct 而非 bitflags | 更直观的 API | 无法直接做位运算 |
| 6 | API key 用 "[REDACTED]" 而非部分显示 | 安全优先 | 调试不便 |

## 6. 测试与验收

### 6.1 现有测试覆盖

| 测试 | 断言点 | 文件 |
|---|---|---|
| `test_ssrf_ipv4_loopback` | 127.x 拒绝 | `ssrf_guard.rs` tests |
| `test_ssrf_ipv4_private` | 10.x / 172.x / 192.168.x 拒绝 | `ssrf_guard.rs` tests |
| `test_ssrf_ipv6_loopback` | ::1 拒绝 | `ssrf_guard.rs` tests |
| `test_ssrf_ipv6_link_local` | fe80:: 拒绝 | `ssrf_guard.rs` tests |
| `test_ssrf_ipv6_ula` | fc00:: / fd00:: 拒绝 | `ssrf_guard.rs` tests |
| `test_ssrf_ipv4_mapped` | ::ffff:127.0.0.1 拒绝 | `ssrf_guard.rs` tests |
| `test_ssrf_valid_url` | 正常 URL 通过 | `ssrf_guard.rs` tests |
| `test_decompression_bomb` | 超大解压拒绝 | `reader/` tests |
| `test_resource_limits` | 超限拒绝 | `reader/` tests |
| `test_api_key_redact` | Debug 输出脱敏 | `ocr/` tests |
| `test_encrypt_decrypt_roundtrip` | AES-128/256 加解密 | `crypto/encrypt.rs` tests |
| `test_sign_verify_roundtrip` | 签名/验证 | `crypto/sign_pdf.rs` tests |
| `test_permissions` | 权限位掩码正确 | `crypto/encrypt.rs` tests |
| `test_cms_signed_data` | CMS 构建/解析 | `crypto/sign_cms.rs` tests |
| `test_der_encode_decode` | DER 编码/解码 | `crypto/sign_der.rs` tests |
| `test_ring_rsa` | ring RSA 签名/验证 | `crypto/sign.rs` tests |
| `test_no_double_hash` | 不再双重哈希 | `crypto/sign_pdf.rs` tests |

**Fuzz targets**：
- `pdf_encrypt_decrypt` — 加密/解密 fuzz
- `pdf_sign_verify` — 签名/验证 fuzz
- `ssrf_url` — SSRF 防护 fuzz

### 6.2 已知局限

- 不支持 RSA 密钥生成（使用 dev-dependency）。
- 不支持 X.509 证书生成（使用测试证书）。
- 不支持 PKCS#12 密钥库。
- 不支持 OCSP/CRL 证书吊销检查。
- 不支持时间戳协议（TSA）。
- AES-CBC padding oracle 风险（不暴露解密错误详情）。

## 7. 引用

- 架构文档：`docs/easypdf-rust-Architecture.md` 第 9 节「安全模型」
- 安全审计：`docs/security/AUDIT.md`
- 使用指南：`docs/usage-guide.md` 第 11 节「加密与签名」
- Roadmap：`docs/roadmap.md` 0.2 Architecture Consolidation（加密签名）、0.4 Security
- 源码：`crates/easypdf-core/src/io/ssrf_guard.rs`、`crates/easypdf-core/src/crypto/`
- Spec：`specs/2026-08-09-iso-32000-crypto-design.md`（加密签名详细设计）
