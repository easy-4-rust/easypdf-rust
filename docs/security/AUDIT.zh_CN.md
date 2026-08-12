# 安全审计报告

**日期**：2026-08-11
**审计员**：Rust 开发工程师（自动化）
**范围**：easypdf-rust 防护验证和 API 密钥泄露
**工具链**：Rust 1.88，macOS Darwin 25.5.0 arm64

---

## 审计范围

| 区域 | 审计文件 | 方法 |
|------|---------|------|
| 解压炸弹防护 | `easypdf-core/src/io/guards.rs` | 攻击向量测试 + 代码审查 |
| 元素爆炸防护 | `easypdf-core/src/io/guards.rs` | 攻击向量测试 + 代码审查 |
| SSRF URL 验证 | `easypdf-core/src/io/ssrf_guard.rs` | 攻击向量测试 + 代码审查 |
| API 密钥泄露 | `easypdf-ocr/src/*/config.rs`、`easypdf-ocr/src/http/auth.rs` | 静态代码审查（Debug 实现） |

---

## A. 解压炸弹防护

防护函数：`guard_decompression_bomb(compressed_size, decompressed_size, &limits) -> Result<()>`

### 默认限制

| 参数 | 默认值 | 严格值 |
|------|--------|--------|
| `max_decompressed_size` | 2 GB | 512 MB |
| `max_compression_ratio` | 100:1 | 50:1 |
| `MIN_COMPRESSED_FOR_RATIO_CHECK` | 64 KB | 64 KB（常量） |

### 测试结果

| 测试 | 输入 | 预期 | 结果 |
|------|------|------|------|
| 高比率 200:1 | 100 KB -> 20 MB | 拒绝 | 通过 |
| 极端比率 10000:1 | 1 MB -> 10 GB | 拒绝 | 通过 |
| 嵌套压缩 1000:1 | 100 KB -> 100 MB | 拒绝 | 通过 |
| 严格限制 60:1 | 100 KB -> 6 MB（严格） | 拒绝 | 通过 |
| 边界精确 | 2 GB -> 2 GB | 通过 | 通过 |
| 边界超 1 | 2 GB+1 -> 2 GB+1 | 拒绝 | 通过 |
| 小数据比率跳过 | 100 B -> 5 KB（50:1） | 通过（低于 10 KB 安全阈值） | 通过 |
| 小数据比率应用 | 100 B -> 1 MB（10000:1） | 拒绝（比率检查生效） | 通过 |
| 零压缩 | 0 -> 1 MB | 通过（无 panic） | 通过 |
| 错误码检查 | 100 KB -> 100 MB | SecurityViolation | 通过 |
| **小 zip 炸弹已修复** | **1 KB -> 1 GB（1000000:1）** | **拒绝** | **已修复** |

### 发现 1：小压缩载荷绕过比率检查 [中等] -- 已修复

**位置**：`easypdf-core/src/io/guards.rs`

**描述**：当 `compressed_size <= MIN_COMPRESSED_FOR_RATIO_CHECK`（64 KB）时，防护跳过了压缩比率检查。这意味着 1 KB 压缩载荷声称解压 1 GB（1,000,000:1 比率）的情况通过了防护，因为比率检查对小输入完全跳过。

**修复方案**：移除了 `MIN_COMPRESSED_FOR_RATIO_CHECK` 阈值。防护现在使用绝对安全解压大小阈值（10 KB）：
- 如果 `decompressed_size < 10 KB`：跳过比率检查（真正微小，无论比率如何都是安全的）
- 如果 `decompressed_size >= 10 KB`：始终检查比率，无论 `compressed_size` 如何
- 使用 `checked_div` 处理零 `compressed_size` 而不 panic

**回归测试**：`audit_a10_small_zip_bomb_ratio_now_blocked` 验证 1 KB -> 1 GB（1,000,000:1）现在被拒绝。

---

## B. 元素爆炸防护

防护函数：`guard_element_explosion(element_count, &limits) -> Result<()>`

### 默认限制

| 参数 | 默认值 | 严格值 |
|------|--------|--------|
| `max_element_count` | 5,000,000 | 1,000,000 |

### 测试结果

| 测试 | 输入 | 预期 | 结果 |
|------|------|------|------|
| 1000 万元素 | 10,000,000 | 拒绝 | 通过 |
| 10 万元素 | 100,000 | 通过 | 通过 |
| 严格 200 万 | 2,000,000（严格） | 拒绝 | 通过 |
| 严格比默认更紧 | 验证所有限制 | 所有更严格 | 通过 |
| 边界精确 | 5,000,000 | 通过 | 通过 |
| 边界超 1 | 5,000,001 | 拒绝 | 通过 |
| 错误码 | usize::MAX | SecurityViolation | 通过 |

### 评估

元素爆炸防护工作正常。边界条件处理正确（包含限制）。严格限制约为默认值的 1/4，如文档所述。

**未发现漏洞。**

---

## C. SSRF URL 验证

防护函数：`validate_url(url: &str) -> Result<()>`

### 拦截规则

1. 协议必须是 `http` 或 `https`
2. 主机不能为空
3. 主机不得匹配被阻止的主机名：`localhost`、`0.0.0.0`、`127.0.0.1`、`metadata.google.internal`、`169.254.169.254`
4. 主机不得解析为私有/回环 IPv4：`127.0.0.0/8`、`10.0.0.0/8`、`172.16.0.0/12`、`192.168.0.0/16`、`169.254.0.0/16`、`0.0.0.0/8`

### 测试结果（35 个攻击 URL，含 IPv6）

| 类别 | 测试 URL 数 | 全部拒绝？ |
|------|------------|-----------|
| 被阻止的协议（file/ftp/gopher/javascript/data） | 5 | 是 |
| 被阻止的主机名（localhost、metadata） | 4 | 是 |
| 回环 IP（127.x.x.x） | 3 | 是 |
| 私有 10.x.x.x | 2 | 是 |
| 私有 172.16-31.x.x | 2 | 是 |
| 私有 192.168.x.x | 2 | 是 |
| 链路本地 169.254.x.x | 2 | 是 |
| 零网络 0.x.x.x | 2 | 是 |
| 格式错误（无协议、空主机） | 2 | 是 |
| IPv6 回环（`[::1]`） | 2 | 是 |
| IPv6 未指定（`[::]`） | 1 | 是 |
| IPv4 映射 IPv6（`[::ffff:x.x.x.x]`） | 3 | 是 |
| IPv6 ULA（`[fc00::1]`、`[fd00::1]`） | 2 | 是 |
| IPv6 链路本地（`[fe80::1]`） | 2 | 是 |

### 测试结果（9 个合法 URL）

| 测试 URL 数 | 全部允许？ |
|------------|-----------|
| HTTPS/HTTP 公共主机、带端口、带查询字符串、公共 IP | 是 |

### 发现 2：IPv6 回环 SSRF 绕过 [高危] -- 已修复

**位置**：`easypdf-core/src/io/ssrf_guard.rs`

**描述**：SSRF 防护仅通过 `is_private_ipv4()` 检查 IPv4 私有范围。IPv6 地址如 `::1`（回环）未被检查，允许攻击者使用 IPv6 表示法绕过 SSRF 防护。

**修复方案**：使用 `std::net::IpAddr` 解析添加了全面的 IPv6 验证：
- `is_blocked_ipv6()` 检查：回环（`::1`）、未指定（`::`）、ULA（`fc00::/7`）、链路本地（`fe80::/10`）
- IPv4 映射 IPv6 地址（`::ffff:x.x.x.x`）被解包，嵌入的 IPv4 由 `is_private_ipv4_addr()` 检查
- `is_private_ipv4_addr()` 使用 `std::net::Ipv4Addr` 方法加上显式 `0.0.0.0/8` 范围检查

**被阻止的 URL**（现在全部正确拒绝）：
- `http://[::1]/` -- IPv6 回环
- `http://[::]/` -- IPv6 未指定
- `http://[::ffff:127.0.0.1]/` -- IPv4 映射回环
- `http://[::ffff:10.0.0.1]/` -- IPv4 映射私有
- `http://[::ffff:169.254.169.254]/` -- IPv4 映射 metadata
- `http://[fc00::1]/`、`http://[fd00::1]/` -- ULA
- `http://[fe80::1]/` -- 链路本地

**回归测试**：`audit_c3` 到 `audit_c8` 验证所有 IPv6 攻击向量被阻止。

---

## D. API 密钥泄露（静态代码审查）

审查了所有包含密钥的配置类型的 `Debug` 实现。

### 测试结果

| 类型 | 文件 | `Debug` 泄露密钥？ | 状态 |
|------|------|-------------------|------|
| `HunyuanConfig` | `easypdf-ocr/src/hunyuan/config.rs` | 否 -- 自定义 `Debug` 脱敏 `secret_id` 和 `secret_key` | 安全 |
| `AuthMethod::Bearer` | `easypdf-ocr/src/http/auth.rs` | 否 -- 显示 `***` 而非 token | 安全 |
| `AuthMethod::ApiKeyHeader` | `easypdf-ocr/src/http/auth.rs` | 否 -- 显示 `***` 而非 key | 安全 |
| `AuthMethod::BearerFromOAuth` | `easypdf-ocr/src/http/auth.rs` | 否 -- 脱敏 `secret_key`，显示 `api_key`（client ID） | 安全 |
| `AuthMethod::TencentCloud` | `easypdf-ocr/src/http/auth.rs` | 否 -- 脱敏 `secret_id`（前4...后4）和 `secret_key`（`***`） | 安全 |
| `OcrHttpClient` | `easypdf-ocr/src/http/client.rs` | 否 -- 委托给 `AuthMethod::Debug` | 安全 |
| `HttpClientConfig` | `easypdf-ocr/src/http/client.rs` | 否 -- 结构体中无密钥 | 安全 |

### 发现 3：GlmConfig Debug 输出泄露 API 密钥 [高危] -- 已修复

**位置**：`easypdf-ocr/src/glm/config.rs`

**描述**：`GlmConfig` 使用 `#[derive(Debug)]`，在调试输出中以明文包含 `api_key` 字段。

**修复方案**：将 `#[derive(Debug)]` 替换为 `#[derive(Clone)]`，并添加手动 `Debug` 实现，将 `api_key` 脱敏为 `"***redacted***"`，与 `HunyuanConfig` 使用的模式一致。

**回归测试**：`audit_d1_glm_config_redacts_api_key` 验证 API 密钥不出现在 Debug 输出中。

### 发现 4：BaiduConfig Debug 输出泄露 API 密钥和 secret key [高危] -- 已修复

**位置**：`easypdf-ocr/src/baidu/config.rs`

**描述**：`BaiduConfig` 使用 `#[derive(Debug)]`，在调试输出中以明文包含 `api_key` 和 `secret_key`。

**修复方案**：将 `#[derive(Debug)]` 替换为 `#[derive(Clone)]`，并添加手动 `Debug` 实现，将 `api_key` 和 `secret_key` 均脱敏为 `"***redacted***"`，与 `HunyuanConfig` 使用的模式一致。

**回归测试**：`audit_d2_baidu_config_redacts_both_keys` 验证两个密钥均不出现在 Debug 输出中。

---

## 发现摘要

| # | 发现 | 严重度 | 位置 | 状态 |
|---|------|--------|------|------|
| 1 | 小压缩载荷绕过比率检查 | 中等 | `guards.rs` | **已修复** -- 绝对安全阈值 + 始终检查比率 |
| 2 | IPv6 回环 SSRF 绕过 | 高危 | `ssrf_guard.rs` | **已修复** -- `std::net::IpAddr` 解析 + IPv6 范围检查 |
| 3 | GlmConfig Debug 泄露 API 密钥 | 高危 | `glm/config.rs` | **已修复** -- 手动 `Debug` 脱敏 |
| 4 | BaiduConfig Debug 泄露 API 密钥 + secret | 高危 | `baidu/config.rs` | **已修复** -- 手动 `Debug` 脱敏 |

### 表现良好的方面

- 元素爆炸防护稳固，边界处理正确
- SSRF 防护现在正确阻止所有 IPv4 和 IPv6 私有/回环范围
- 解压炸弹防护现在通过绝对安全阈值捕获小载荷炸弹
- 所有包含密钥的配置类型（HunyuanConfig、GlmConfig、BaiduConfig、AuthMethod）均有正确的 Debug 脱敏
- OcrHttpClient 正确委托 Debug 脱敏
- 所有防护返回 `PdfError::SecurityViolation` 并附带描述性消息
- 27 个安全审计回归测试覆盖所有 4 个发现领域

---

## 交付物

1. **测试文件**：`easypdf-test/tests/security_audit.rs` -- 27 个回归测试覆盖所有 4 个审计领域（全部通过）
2. **审计报告**：`docs/security/AUDIT.md` -- 本文件（所有 4 个发现标记为已修复）
3. **依赖变更**：`easypdf-test/Cargo.toml` -- 添加 `easypdf-ocr` 作为 dev-dependency 用于 Debug 脱敏测试
4. **源码修复**：`guards.rs`、`ssrf_guard.rs`、`glm/config.rs`、`baidu/config.rs`
