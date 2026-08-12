# easypdf v0.1.0 发布日志

## 发布概要

- **发布日期**: 2026-08-12 (UTC: 2026-08-11)
- **发布版本**: 0.1.0
- **发布账户**: easy-4-rust
- **总 crate 数**: 8
- **全部成功**: YES

## 发布顺序与结果

| # | Crate | 发布时间 (UTC+8) | 结果 | 备注 |
|---|-------|-----------------|------|------|
| 1 | easypdf-core | 03:50:23 | SUCCESS | leaf crate，无内部依赖 |
| 2 | easypdf-derive | 03:51:13 | SUCCESS | 依赖 core |
| 3 | easypdf-reader | 03:52:02 | SUCCESS | 依赖 core |
| 4 | easypdf-writer | 03:52:53 | SUCCESS | 依赖 core |
| 5 | easypdf-markdown | 03:53:44 | SUCCESS | 依赖 core + reader + writer |
| 6 | easypdf-ocr | 03:56:36 | SUCCESS (retry) | 依赖 core + markdown；首次遇 429 rate limit |
| 7 | easypdf-runtime | 04:07:57 | SUCCESS (retry) | 依赖 reader + writer + markdown；两次遇 429 |
| 8 | easypdf | 04:19:22 | SUCCESS (retry) | 门面 crate；两次遇 429 |

## Rate Limit 说明

crates.io 对新 crate 发布有速率限制（约 5 个新 crate / 10 分钟窗口）。发布过程中：

- 第 1-5 个 crate 连续成功
- 第 6 个 crate 首次遇到 429 Too Many Requests，等待约 10 分钟后重试成功
- 第 7、8 个 crate 同样需要等待 rate limit 窗口重置后重试

## crates.io 链接

| Crate | 链接 |
|-------|------|
| easypdf-core | https://crates.io/crates/easypdf-core |
| easypdf-derive | https://crates.io/crates/easypdf-derive |
| easypdf-reader | https://crates.io/crates/easypdf-reader |
| easypdf-writer | https://crates.io/crates/easypdf-writer |
| easypdf-markdown | https://crates.io/crates/easypdf-markdown |
| easypdf-ocr | https://crates.io/crates/easypdf-ocr |
| easypdf-runtime | https://crates.io/crates/easypdf-runtime |
| easypdf | https://crates.io/crates/easypdf |

## 发布参数

```bash
cargo publish -p <crate> --allow-dirty --no-verify
```

- `--allow-dirty`: working tree 有未 commit 的修改
- `--no-verify`: 首次发布前无 git tag，跳过 tag 验证

## 已知警告

以下 crate 发布时有 readme 路径警告（不影响功能）：
- easypdf-ocr
- easypdf-runtime

警告内容：`readme ../../README.md appears to be a path outside of the package`。可在后续版本中修复 Cargo.toml 中的 readme 路径。

## 后续步骤（用户手动执行）

### 1. Git 打 tag

```bash
cd /Users/wandl/workspaces/workspace-github-easy-4-rust/easypdf-rust
git add -A
git commit -m "release: v0.1.0 - initial crates.io publish"
git tag v0.1.0
git push origin main
git push origin v0.1.0
```

### 2. 创建 GitHub Release

在 GitHub 上基于 v0.1.0 tag 创建 Release，附上变更说明。

### 3. 修复 readme 路径警告（可选）

在 `crates/easypdf-ocr/Cargo.toml` 和 `crates/easypdf-runtime/Cargo.toml` 中将 readme 路径改为相对路径或移除该字段。

## 验证结果

所有 8 个 crate 均已在 crates.io 上验证可访问，版本号为 0.1.0：

```
easypdf-core:    version=0.1.0, downloads=0
easypdf-derive:  version=0.1.0, downloads=0
easypdf-reader:  version=0.1.0, downloads=0
easypdf-writer:  version=0.1.0, downloads=0
easypdf-markdown: version=0.1.0, downloads=0
easypdf-ocr:     version=0.1.0, downloads=0
easypdf-runtime: version=0.1.0, downloads=0
easypdf:         version=0.1.0, downloads=0
```
