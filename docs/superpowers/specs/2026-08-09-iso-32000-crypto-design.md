# ISO 32000 Crypto Design

**日期**: 2026-08-09
**作用范围**: `easypdf-core::crypto`（encrypt / sign / sign_cms / sign_der / sign_pdf）
**类型**: 安全关键设计

---

## 1. 背景与问题

easypdf-rust 需要实现 ISO 32000 规范的 PDF 加密和数字签名能力。printpdf 和 lopdf 均不提供完整的加密/签名实现：

- printpdf 的 `PdfSaveOptions.secure` 字段为空（只有 `secure: bool`，无实际逻辑）
- lopdf 仅支持 RC4 解密（已被弃用）

此外，加密/签名实现面临以下安全挑战：

### 1.1 Marvin Attack (RUSTSEC-2023-0071)
`rsa` crate 存在 Marvin Attack 漏洞 -- RSA PKCS#1v1.5 解密的时间侧信道攻击。生产代码路径必须迁移到常量时间实现。

### 1.2 双重哈希问题
签名时如果先 SHA-256 哈希再 RSA 签名（即 `RSA(SHA-256(data))`），而 CMS 规范要求直接 RSA 签名（即 `RSA(data)`，哈希由 CMS 内部处理），会导致签名验证失败。

### 1.3 ISO 32000 版本差异
- V4/R4（AES-128）：PDF 1.6 引入
- V5/R6（AES-256）：PDF 2.0（ISO 32000-2）引入，安全性更高

---

## 2. 设计方案

### 2.1 加密模块 (`encrypt.rs`)

#### 2.1.1 算法枚举

```rust
pub enum PdfEncryptionAlgorithm {
    /// AES-128-CBC (`/V 4`, `/R 4`). 128-bit file encryption key.
    Aes128,
    /// AES-256-CBC (`/V 5`, `/R 6`). 256-bit file encryption key (ISO 32000-2).
    Aes256,
}
```

#### 2.1.2 权限位掩码

```rust
pub struct PdfPermissions {
    // PDF permission bits (ISO 32000 Table 22)
    pub print: bool,           // Bit 3
    pub modify: bool,          // Bit 4
    pub copy: bool,            // Bit 5
    pub annotate: bool,        // Bit 6
    pub fill_forms: bool,      // Bit 9
    pub extract: bool,         // Bit 10 (accessibility)
    pub assemble: bool,        // Bit 11
    pub print_high: bool,      // Bit 12
}
```

#### 2.1.3 加密流程

1. 生成随机文件加密密钥（128-bit 或 256-bit）
2. 用 AES-CBC 加密所有 stream 和 string 对象
3. 用用户密码加密文件加密密钥（SHA-256 → AES wrap）
4. 构建 `/Encrypt` 字典（/V, /R, /O, /U, /P, /Filter）
5. 将 `/Encrypt` 字典添加到 PDF trailer

#### 2.1.4 解密流程

1. 读取 `/Encrypt` 字典
2. 用密码解密文件加密密钥
3. 用文件加密密钥解密所有 stream 和 string 对象
4. 返回解密后的 PDF 字节

### 2.2 签名模块 (`sign.rs` / `sign_cms.rs` / `sign_der.rs` / `sign_pdf.rs`)

#### 2.2.1 CMS SignedData 结构

```
ContentInfo ::= SEQUENCE {
    contentType ContentType,
    content [0] EXPLICIT ANY DEFINED BY contentType
}

SignedData ::= SEQUENCE {
    version CMSVersion,
    digestAlgorithms DigestAlgorithmIdentifiers,
    encapContentInfo EncapsulatedContentInfo,
    certificates [0] IMPLICIT CertificateSet OPTIONAL,
    signerInfos SignerInfos
}
```

- 使用 detached 签名（`encapContentInfo` 为空）
- 签名覆盖 PDF 的 `/ByteRange` 指定的字节范围

#### 2.2.2 签名流程

1. 在 PDF 中预留签名占位空间（`/ByteRange` + `/Contents`）
2. 计算 `/ByteRange` 指定范围的 SHA-256 摘要
3. 用 RSA 私钥签名摘要（RSA-PKCS#1v1.5，**不再先哈希**）
4. 构建 CMS `SignedData` 包（含 X.509 证书）
5. 将签名写入 `/Contents` 占位空间
6. 更新 `/ByteRange` 为实际偏移

#### 2.2.3 验证流程

1. 读取签名字段（`/ByteRange` + `/Contents`）
2. 解析 CMS `SignedData` 包
3. 提取 X.509 证书（使用 x509-parser）
4. 重新计算 `/ByteRange` 范围的 SHA-256 摘要
5. 用证书公钥验证签名

### 2.3 ring 常量时间 RSA 迁移

**问题**: `rsa` crate（RUSTSEC-2023-0071）的 RSA PKCS#1v1.5 实现存在时间侧信道（Marvin Attack）。

**方案**: 
- 生产代码路径使用 `ring::signature::RSA_PKCS1_SHA256` 进行签名和验证
- ring 使用 blinding 和常量时间操作，天然抵抗时间侧信道
- `rsa` crate 保留为 dev-dependency，仅用于测试证书生成（ring 没有 keygen API）
- 在 `.cargo/audit.toml` 中忽略 RUSTSEC-2023-0071（仅影响 dev-dependency）

### 2.4 DER 编码 (`sign_der.rs`)

手动实现最小化的 DER 编码器/解码器，支持：
- SEQUENCE / SET / INTEGER / OID / OCTET STRING / NULL
- X.509 TBSCertificate 结构
- CMS ContentInfo / SignedData / SignerInfo 结构

**不使用** `der` 或 `yasna` crate，保持依赖最小化。

### 2.5 PDF 签名集成 (`sign_pdf.rs`)

```rust
pub fn sign_pdf(pdf_bytes: &[u8], signer: &PdfSigner) -> Result<Vec<u8>, CryptoError> {
    // 1. 解析 PDF，找到或创建签名字段
    // 2. 预留 /ByteRange + /Contents 占位
    // 3. 计算摘要
    // 4. RSA 签名（ring，常量时间）
    // 5. 构建 CMS SignedData
    // 6. 写回签名
}

pub fn verify_pdf_signature(pdf_bytes: &[u8]) -> Result<SignatureInfo, CryptoError> {
    // 1. 解析 PDF，找到签名字段
    // 2. 读取 /ByteRange 和 /Contents
    // 3. 解析 CMS SignedData
    // 4. 提取证书和签名
    // 5. 验证摘要和签名
}
```

---

## 3. 测试改动范围

- `easypdf-core/src/crypto/sign_tests.rs` -- 签名/验证 roundtrip 测试（已有 6+ 个测试）
- `easypdf-core/src/crypto/encrypt.rs` -- 加密/解密 roundtrip 测试
- `fuzz/fuzz_targets/pdf_encrypt_decrypt.rs` -- 加密/解密 fuzz target
- `fuzz/fuzz_targets/pdf_sign_verify.rs` -- 签名/验证 fuzz target
- `easypdf/src/crypto_facade.rs` -- 门面 API 测试

---

## 4. 不在范围内（YAGNI）

- 不实现 RSA 密钥生成（使用 `rsa` crate 的 dev-dependency 生成测试密钥）
- 不实现 X.509 证书生成（使用测试证书）
- 不实现 PKCS#12 密钥库解析
- 不实现 OCSP/CRL 证书吊销检查
- 不实现时间戳协议（TSA）
- 不实现 PDF 增量保存（修改 PDF 而不重写整个文件）

---

## 5. 风险与缓解

| 风险 | 影响 | 缓解 |
|---|---|---|
| ring API 不支持 RSA 密钥生成 | 测试不便 | rsa 保留为 dev-dependency，仅测试使用 |
| DER 手动编码错误 | 签名验证失败 | 充分的 roundtrip 测试 + fuzz 测试 |
| PDF /ByteRange 偏移计算错误 | 签名无效 | 参考 ISO 32000 规范，用真实 PDF reader 验证 |
| AES-CBC padding oracle | 加密不安全 | 使用 AES-CBC + PKCS#7 padding，不暴露解密错误详情 |
| Marvin Attack 残留 | 时间侧信道 | 生产路径全部使用 ring，rsa 仅 dev-dependency |

---

## 6. 实施顺序

1. 实现 AES-128/256 加密/解密（`encrypt.rs`）
2. 实现权限位掩码（`PdfPermissions`）
3. 实现 DER 编码器/解码器（`sign_der.rs`）
4. 实现 CMS SignedData 构建/解析（`sign_cms.rs`）
5. 实现 PDF 签名集成（`sign_pdf.rs`）
6. ring RSA 迁移（替换生产代码路径中的 rsa 调用）
7. 修复双重哈希问题
8. 实现门面 API（`EasyPdf::encrypt()` / `EasyPdf::sign()`）
9. fuzz 测试（pdf_encrypt_decrypt / pdf_sign_verify）
10. 安全审计文档（docs/security/AUDIT.md）
