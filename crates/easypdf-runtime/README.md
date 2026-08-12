# easypdf-runtime

> Runtime layer: MCP Server (LLM agent interface) + Resident Daemon (in-memory document session persistence).

## Role

`easypdf-runtime` is the runtime layer of easypdf-rust, providing two service modes. The MCP Server exposes PDF operations to LLM agents via the standard MCP protocol (JSON-RPC 2.0 over stdio). The Resident Daemon keeps PDF document sessions in memory in the background, avoiding repeated open/parse cycles, and supports both Unix Socket and TCP transports.

## Core Capabilities

- **MCP Server** (`McpServer`) -- exposes 7 PDF tools via JSON-RPC 2.0 over stdio (`crates/easypdf-runtime/src/mcp/`)
- **Resident Daemon** (`ResidentServer` / `ResidentClient`) -- background document session persistence with multi-session concurrency (`crates/easypdf-runtime/src/resident/`)
- **IPC transports** -- Unix Socket (Linux/macOS via `UnixTransport`) and TCP (cross-platform via `TcpTransport`) (`crates/easypdf-runtime/src/resident/transport.rs`)
- **Session management** (`DocumentSession`) -- full lifecycle: open/read/close with `OpenMode::ReadOnly` / `ReadWrite` (`crates/easypdf-runtime/src/resident/session.rs`)
- **Autosave** (`AutosaveMode`) -- Disabled / Fixed interval / Adaptive interval (`crates/easypdf-runtime/src/resident/config.rs`)
- **Idle timeout** -- daemon auto-closes inactive sessions (`crates/easypdf-runtime/src/resident/config.rs`)
- **Protocol types** -- `Request`, `Response`, `ResponseData`, `SessionId` with serde_json serialization (`crates/easypdf-runtime/src/resident/protocol.rs`)
- **MCP binary** -- standalone `easypdf-mcp` binary (`crates/easypdf-runtime/src/mcp/main.rs`)

## Dependencies

### Internal

| Crate | Purpose |
|-------|---------|
| `easypdf-core` | Core types |
| `easypdf-reader` | PDF reading |
| `easypdf-writer` | PDF creation |
| `easypdf-markdown` | Markdown conversion |

### External

| Crate | Version | Purpose |
|-------|---------|---------|
| `serde` / `serde_json` | 1.x | Protocol serialization |
| `tracing` | 0.1 | Structured logging |

## Main API

### MCP Server

```rust
use easypdf_runtime::mcp::McpServer;

let server = McpServer::new();
server.run()?; // blocks, handles JSON-RPC requests via stdio
```

MCP tools (7 tools defined in `crates/easypdf-runtime/src/mcp/tools.rs:54-63`):

| Tool | Description |
|------|-------------|
| `pdf_read_text` | Extract text from PDF |
| `pdf_to_markdown` | Convert PDF to Markdown |
| `pdf_create_text` | Create a new text PDF |
| `pdf_merge` | Merge multiple PDFs |
| `pdf_split` | Split a PDF |
| `pdf_metadata` | Extract document metadata |
| `pdf_page_count` | Get page count |

### Resident Daemon

```rust
use easypdf_runtime::resident::{
    ResidentServer, ResidentClient, ResidentConfig,
    OpenMode, default_socket_path, socket_path_for_file,
};

// Start daemon
let server = ResidentServer::bind(default_socket_path())?;
server.run()?;

// Connect client
let client = ResidentClient::connect(socket_path_for_file("doc.pdf"))?;
let session = client.open("doc.pdf", OpenMode::ReadOnly)?;
let text = client.extract_text(session, None)?;
client.close(session)?;
client.shutdown()?;
```

### Convenience Functions

```rust
use easypdf_runtime::resident::{serve, try_attach, default_socket_path};

// Start foreground server
serve(default_socket_path())?;

// Try to attach to running daemon
if let Ok(client) = try_attach() {
    // reuse existing session
}
```

## Feature Flags

| Feature | Description |
|---------|-------------|
| `default` | `mcp + resident` |
| `mcp` | Enable MCP Server module |
| `resident` | Enable Resident Daemon module |

## License

Apache-2.0

---

**Project**: https://github.com/easy-4-rust/easypdf-rust
**crates.io**: https://crates.io/crates/easypdf-runtime
**docs.rs**: https://docs.rs/easypdf-runtime
