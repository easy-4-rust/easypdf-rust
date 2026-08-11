# easypdf-rust 迁移与质量验证记录

- Rust 基线：执行验证时记录当前提交与工作区差异。
- Java 基线：未提供。
- 兼容性声明：easypdf-rust 是独立 Rust PDF 产品，不声明对某个 Java 仓库的逐对象、逐测试无损迁移。
- 全项目验收包：`easypdf-test`。
- 可用证据：Rust 公共 API 端到端测试、编译/Clippy/文档测试、LLVM 覆盖率。
- 不可用证据：Java 测试清单、Java fixtures、Java live/golden oracle、对象级对照表。

因此 `rust-java-migration-testing` 的工程质量门禁适用；SOURCE_PARITY、V3_GOLDEN 与 V4_LIVE_DIFF 当前为 `BLOCKED`，不能据此宣称“Java→Rust 100% 等价”。若后续指定 Java 基线，必须冻结 SHA，并补齐 `source-test-parity.json`、对象表和逐 case MATCH 证据。
