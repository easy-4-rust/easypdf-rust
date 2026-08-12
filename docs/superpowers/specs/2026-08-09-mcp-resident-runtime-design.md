# easypdf-runtime MCP 服务器与 Resident Daemon 设计

- **日期**：2026-08-09
- **作者**：ZCode Agent（协同设计）
- **状态**：已实现（v0.1.0 发布）
- **依赖**：easypdf-runtime 现有 `mcp/`（protocol / server / tools）、`resident/`（transport / unix / tcp / session / server / client / port）

## 1. 目标与范围

为 easypdf-rust 实现 **MCP（Model Context Protocol）服务器**和 **Resident Daemon**，使 LLM Agent 能通过标准化 JSON-RPC 协议操作 PDF，同时通过 resident daemon 实现会话持久化和自适应 autosave。

**核心需求**：

1. MCP 服务器实现 stdio JSON-RPC 传输。
2. 7 个 MCP 工具：pdf_read_text / pdf_to_markdown / pdf_create_text / pdf_merge / pdf_split / pdf_metadata / pdf_page_count。
3. Resident Daemon 支持 Unix socket（默认）和 Windows TCP fallback。
4. 自适应 autosave（EMA 平滑，空闲超时 watchdog）。
5. 会话管理（多文档并发操作）。
6. Feature gate：`mcp` 和 `resident` 独立控制。

**非目标**：

- 不实现 MCP 的 WebSocket 传输（仅 stdio）。
- 不实现 MCP 的 resources / prompts（仅 tools）。
- 不实现 Resident Daemon 的集群模式（单实例）。
- 不实现 PDF 加密/签名的 MCP 工具（由门面 API 承担）。
- 不实现 MCP 工具的热重载。

## 2. 总体架构

```
┌──────────────────────────────────────────────────────────────┐
│                     easypdf-runtime                           │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐  │
│  │  MCP Server (stdio JSON-RPC)                          │  │
│  │  ├── protocol.rs   JSON-RPC 协议解析                   │  │
│  │  ├── server.rs     主循环 + 请求分发                   │  │
│  │  └── tools.rs      7 个工具实现                        │  │
│  │      ├── pdf_read_text       文本提取                  │  │
│  │      ├── pdf_to_markdown     Markdown 转换             │  │
│  │      ├── pdf_create_text     文本 PDF 创建             │  │
│  │      ├── pdf_merge           PDF 合并                  │  │
│  │      ├── pdf_split           PDF 拆分                  │  │
│  │      ├── pdf_metadata        元数据读取                │  │
│  │      └── pdf_page_count      页数查询                  │  │
│  └────────────────────────────────────────────────────────┘  │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐  │
│  │  Resident Daemon                                       │  │
│  │  ├── server.rs      服务器主循环                       │  │
│  │  ├── session.rs     会话管理 + 自适应 autosave         │  │
│  │  ├── client.rs      客户端代理                         │  │
│  │  ├── port.rs        端口分配                           │  │
│  │  └── transport/     传输层                             │  │
│  │      ├── mod.rs      Transport trait                   │  │
│  │      ├── unix.rs     Unix domain socket                │  │
│  │      └── tcp.rs      Windows TCP fallback              │  │
│  └────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────┘
```

## 3. 模块职责划分

### 3.1 `mcp/protocol.rs` — JSON-RPC 协议

| 类型 | 职责 |
|---|---|
| `JsonRpcRequest` | JSON-RPC 2.0 请求（method / params / id） |
| `JsonRpcResponse` | JSON-RPC 2.0 响应（result / error / id） |
| `JsonRpcError` | 错误码（-32700 Parse / -32600 Invalid / -32601 MethodNotFound / -32602 InvalidParams / -32603 Internal） |
| `McpTool` | 工具定义（name / description / inputSchema） |
| `McpToolResult` | 工具执行结果（content / isError） |

**协议流程**：
```
1. Client → Server: initialize (capabilities)
2. Server → Client: initialize response (capabilities)
3. Client → Server: tools/list
4. Server → Client: tools/list response (7 tools)
5. Client → Server: tools/call (tool name + arguments)
6. Server → Client: tools/call response (result)
```

### 3.2 `mcp/server.rs` — 主循环

| 方法 | 职责 |
|---|---|
| `run()` | 主循环：stdin 读取 → 解析 → 分发 → stdout 输出 |
| `handle_request(req)` | 请求分发（initialize / tools/list / tools/call） |
| `handle_tool_call(name, args)` | 工具调用分发 |

**关键设计**：
- 单线程阻塞 I/O（stdin/stdout）
- 每个请求独立处理（无并发）
- 错误不 panic，返回 JsonRpcError

### 3.3 `mcp/tools.rs` — 7 个工具

| 工具 | 输入 | 输出 | 实现 |
|---|---|---|---|
| `pdf_read_text` | `{ path: string }` | `{ text: string }` | `EasyPdf::read(path).extract_text()` |
| `pdf_to_markdown` | `{ path: string, profile?: string }` | `{ markdown: string }` | `EasyPdf::read(path).to_markdown()` |
| `pdf_create_text` | `{ path: string, text: string }` | `{ success: boolean }` | `EasyPdf::create(path).add_text(text).do_write()` |
| `pdf_merge` | `{ paths: string[], output: string }` | `{ success: boolean }` | `EasyPdf::merge(paths, output)` |
| `pdf_split` | `{ path: string, output_dir: string }` | `{ files: string[] }` | `EasyPdf::split(path).output_dir(dir).do_split()` |
| `pdf_metadata` | `{ path: string }` | `{ metadata: object }` | `EasyPdf::read(path).extract_metadata()` |
| `pdf_page_count` | `{ path: string }` | `{ count: number }` | `EasyPdf::read(path).page_count()` |

**参数校验**：
- 路径参数做 SSRF 防护（拒绝 `file://` / 私有 IP）
- 必填参数缺失返回 InvalidParams 错误

### 3.4 `resident/server.rs` — Resident Server

| 方法 | 职责 |
|---|---|
| `bind(path)` | 绑定到 Unix socket / TCP 端口 |
| `run()` | 主循环：接受连接 → 分发到 session |
| `shutdown()` | 优雅关闭（保存所有会话） |

### 3.5 `resident/session.rs` — 会话管理

| 方法 | 职责 |
|---|---|
| `new(id)` | 创建新会话 |
| `handle_command(cmd)` | 处理命令（open / read / write / save / close） |
| `autosave()` | 自适应 autosave（EMA 平滑） |

**自适应 autosave 设计**：

```
EMA(exponential moving average) 平滑：
  ema = α * current_interval + (1-α) * ema
  autosave_interval = ema * 1.5

空闲超时 watchdog：
  if idle > 30s && dirty → save
  if idle > 5min → close session

实现：
  - 每次写操作更新 dirty 标志和最后写入时间
  - watchdog 线程定期检查（每 10s）
  - autosave 在下次写操作前触发（如果间隔已过）
```

### 3.6 `resident/transport/` — 传输层

| 实现 | 平台 | 地址 |
|---|---|---|
| `UnixTransport` | macOS / Linux | `/tmp/easypdf-resident-{pid}.sock` |
| `TcpTransport` | Windows | `127.0.0.1:{port}` |

**Transport trait**：
```rust
pub trait Transport {
    fn bind(&self, addr: &str) -> Result<Listener>;
    fn connect(&self, addr: &str) -> Result<Stream>;
}
```

### 3.7 `resident/client.rs` — 客户端代理

| 方法 | 职责 |
|---|---|
| `connect(addr)` | 连接到 resident daemon |
| `open(path)` | 打开 PDF 文件 |
| `read_text(page?)` | 读取文本 |
| `write_text(text, page?)` | 写入文本 |
| `save()` | 保存 |
| `close()` | 关闭 |

### 3.8 `resident/port.rs` — 端口分配

| 方法 | 职责 |
|---|---|
| `allocate_port()` | 分配可用端口（TCP 模式） |
| `release_port(port)` | 释放端口 |

## 4. 关键数据流

### 4.1 MCP 工具调用流程

```
LLM Agent
    │
    ▼ (stdin JSON-RPC)
McpServer::run()
    │
    ▼
handle_request(JsonRpcRequest { method: "tools/call", params })
    │
    ▼
handle_tool_call("pdf_read_text", { path: "/tmp/test.pdf" })
    │
    ▼
EasyPdf::read("/tmp/test.pdf").extract_text()
    │
    ▼
JsonRpcResponse { result: { text: "..." } }
    │
    ▼ (stdout JSON-RPC)
LLM Agent
```

### 4.2 Resident Daemon 会话流程

```
Client → ResidentServer: connect(unix_socket)
    │
    ▼
ResidentServer: 新建 Session { id, transport }
    │
    ▼
Client → Session: open("/tmp/test.pdf")
    │
    ▼
Session: PdfReader::open(path) → 持有 Document
    │
    ▼
Client → Session: read_text(page=0)
    │
    ▼
Session: extract_text() → 返回文本（复用 Document，129x 加速）
    │
    ▼
Client → Session: write_text("Hello", page=0)
    │
    ▼
Session: 标记 dirty = true, 更新 last_write_time
    │
    ▼ (autosave watchdog)
Session: if dirty && idle > 30s → save()
```

### 4.3 MCP + Resident 协同

```
LLM Agent
    │
    ▼ (MCP stdio)
McpServer → ResidentClient → ResidentServer
    │                              │
    │                              ▼
    │                         Session (持久化)
    │                              │
    ▼                              ▼
EasyPdf API ←──────────────→ PDF 文件
```

## 5. 技术决策与权衡

| # | 决策 | 理由 | 权衡 |
|---|---|---|---|
| 1 | MCP 仅 stdio 传输 | 最简单、最广泛支持 | 无法支持远程调用 |
| 2 | MCP 仅 tools（无 resources/prompts） | LLM Agent 主要通过 tools 交互 | 功能受限 |
| 3 | Resident 用 Unix socket | 本地高效、权限可控 | Windows 需 TCP fallback |
| 4 | 自适应 autosave 用 EMA | 平滑响应写入模式 | 参数需调优 |
| 5 | 单线程阻塞 I/O | 实现简单、无并发问题 | 性能受限 |
| 6 | Session 持有 Document | 复用解析结果（129x 加速） | 内存占用 |

## 6. 测试与验收

### 6.1 现有测试覆盖

| 测试 | 断言点 | 文件 |
|---|---|---|
| `test_mcp_initialize` | initialize 握手正确 | `mcp/server.rs` tests |
| `test_mcp_tools_list` | 7 个工具定义正确 | `mcp/tools.rs` tests |
| `test_mcp_pdf_read_text` | 文本提取工具 | `mcp/tools.rs` tests |
| `test_mcp_pdf_merge` | 合并工具 | `mcp/tools.rs` tests |
| `test_mcp_error_handling` | 错误处理 | `mcp/protocol.rs` tests |
| `test_resident_bind` | Unix socket 绑定 | `resident/server.rs` tests |
| `test_resident_session` | 会话创建和管理 | `resident/session.rs` tests |
| `test_resident_autosave` | 自适应 autosave | `resident/session.rs` tests |
| `test_resident_client` | 客户端连接和命令 | `resident/client.rs` tests |
| `test_transport_unix` | Unix 传输 | `resident/transport/unix.rs` tests |
| `test_transport_tcp` | TCP 传输 | `resident/transport/tcp.rs` tests |

### 6.2 已知局限

- MCP 不支持 WebSocket 传输（仅 stdio）。
- MCP 不支持 resources / prompts（仅 tools）。
- Resident Daemon 不支持集群模式（单实例）。
- Session 持有 Document 会占用内存（大文件需注意）。
- autosave 参数（EMA α、超时阈值）可能需要根据使用场景调优。

## 7. 引用

- 架构文档：`docs/easypdf-rust-Architecture.md` 第 8 节「easypdf-runtime 运行时」
- 使用指南：`docs/usage-guide.md` 第 10 节「MCP 集成」
- Roadmap：`docs/superpowers/version-plan.md` 0.2 Architecture Consolidation（MCP + Resident）
- 源码：`crates/easypdf-runtime/src/mcp/`、`crates/easypdf-runtime/src/resident/`
