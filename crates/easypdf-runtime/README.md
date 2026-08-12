# easypdf-runtime

> 运行时层：MCP Server（LLM 工具接口）+ 常驻守护进程（文档会话持久化）。

## 角色

`easypdf-runtime` 是 easypdf-rust 的运行时层，提供两种服务模式：MCP Server 通过标准 MCP 协议将 PDF 操作暴露给 LLM 智能体；常驻守护进程（Resident Daemon）在后台保持 PDF 文档会话，避免重复打开/解析，支持 Unix Socket 和 TCP 两种传输。

## 核心能力

- **MCP Server**（`McpServer`）——通过 stdio 的 JSON-RPC 2.0 协议暴露 7 个 PDF 工具
- **常驻守护**（`ResidentServer` / `ResidentClient`）——后台保持文档会话，支持多会话并发
- **IPC 传输**——Unix Socket（Linux/macOS）和 TCP（跨平台）双传输层
- **会话管理**（`DocumentSession`）——文档打开/读取/关闭的完整生命周期
- **自动保存**（`AutosaveMode`）——支持禁用、固定间隔、自适应间隔三种模式
- **空闲超时**——守护进程自动关闭无活动会话

## 依赖

- `easypdf-core`: 核心类型
- `easypdf-reader`: PDF 读取
- `easypdf-writer`: PDF 创建
- `easypdf-markdown`: Markdown 转换
- `serde` / `serde_json`: 协议序列化
- `tracing`: 日志

## 主要 API

### MCP Server
```rust
use easypdf_runtime::mcp::McpServer;

let server = McpServer::new();
server.run()?; // 阻塞，通过 stdio 处理 JSON-RPC 请求
```

MCP 工具列表：
- `pdf_read_text` —— 提取 PDF 文本
- `pdf_to_markdown` —— PDF 转 Markdown
- `pdf_create_text` —— 创建新 PDF
- `pdf_merge` —— 合并多个 PDF
- `pdf_split` —— 拆分 PDF
- `pdf_metadata` —— 提取元数据
- `pdf_page_count` —— 获取页数

### 常驻守护进程
```rust
use easypdf_runtime::resident::{ResidentServer, ResidentClient, ResidentConfig};

// 启动守护进程
let server = ResidentServer::bind("/tmp/easypdf.sock")?;
server.run()?;

// 连接客户端
let client = ResidentClient::connect("/tmp/easypdf.sock")?;
let session = client.open("doc.pdf", OpenMode::ReadOnly)?;
let text = client.extract_text(session, None)?;
client.close(session)?;
client.shutdown()?;
```

## Feature flags

| Feature | 说明 |
|--------|------|
| `mcp` | 启用 MCP Server 模块（默认启用） |
| `resident` | 启用常驻守护进程模块（默认启用） |

## License

Apache-2.0

---

**项目主页**：https://github.com/easy-4-rust/easypdf-rust
**crates.io**：https://crates.io/crates/easypdf-runtime
