//! Resident PDF 守护进程服务器。
//!
//! 在内存中维护已打开的 PDF 会话，并通过 IPC（Unix socket 或 TCP，
//! 取决于平台和传输配置）接受命令。

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use tracing::{debug, error, info, warn};

use super::config::{AutosaveMode, ResidentConfig};
use super::error::{ResidentError, Result};
use super::protocol::{MAX_MESSAGE_BYTES, Request, Response, ResponseData, SessionId};
use super::session::DocumentSession;
use super::transport::Transport;

/// 共享的服务器状态，由互斥锁保护。
struct ServerState {
    /// 活跃的文档会话。
    sessions: HashMap<SessionId, DocumentSession>,
    /// 最后活动时间戳（用于空闲超时）。
    last_activity: Instant,
    /// 服务器配置。
    config: ResidentConfig,
}

/// Resident PDF 守护进程。
///
/// 在内存中维护多个已打开的 PDF 会话，并通过 IPC 接受命令。
/// 传输层通过 [`Transport`] trait 可插拔：
///
/// - **Unix（Linux/macOS）**：使用 [`bind()`](ResidentServer::bind) 创建 Unix 域 socket。
/// - **Windows**：使用 [`bind_tcp()`](ResidentServer::bind_tcp) 创建 TCP localhost。
/// - **自定义**：使用 [`with_transport()`](ResidentServer::with_transport) 传入任何
///   [`Transport`] 实现。
///
/// # 示例
///
/// ```no_run
/// use easypdf_runtime::resident::{ResidentServer, ResidentConfig};
///
/// let server = ResidentServer::bind("/tmp/easypdf.sock")?;
/// server.run()?;
/// # Ok::<(), easypdf_runtime::resident::ResidentError>(())
/// ```
pub struct ResidentServer {
    /// 传输层（监听器）。
    transport: Box<dyn Transport>,
    /// 用于清理的 socket 路径（Unix）或空路径（TCP）。
    socket_path: PathBuf,
    /// 共享状态。
    state: Arc<Mutex<ServerState>>,
    /// 下一个会话 ID 计数器。
    next_session_id: Arc<AtomicU64>,
    /// 运行标志（用于看门狗线程协调）。
    running: Arc<AtomicBool>,
}

impl ResidentServer {
    /// 使用默认配置绑定到指定 socket 路径。
    ///
    /// 在 Unix 上创建 Unix 域 socket。在非 Unix 平台上回退到
    /// TCP localhost -- `socket_path` 参数被忽略，分配随机端口。
    ///
    /// # Errors
    ///
    /// 如果绑定失败，返回 `ResidentError::Io`。
    pub fn bind(socket_path: impl AsRef<Path>) -> Result<Self> {
        Self::bind_with_config(socket_path, ResidentConfig::default())
    }

    /// 使用显式配置绑定到指定 socket 路径。
    ///
    /// 在 Unix 上使用配置的 `socket_mode` 创建 Unix 域 socket。
    /// 在非 Unix 平台上回退到 TCP localhost。
    ///
    /// # Errors
    ///
    /// 如果绑定失败，返回 `ResidentError::Io`。
    pub fn bind_with_config(socket_path: impl AsRef<Path>, config: ResidentConfig) -> Result<Self> {
        #[cfg(unix)]
        {
            let socket_path = socket_path.as_ref().to_path_buf();
            let transport =
                super::unix::UnixTransport::bind_with_mode(&socket_path, config.socket_mode)?;
            Ok(Self::with_transport_and_path(
                Box::new(transport),
                socket_path,
                config,
            ))
        }
        #[cfg(not(unix))]
        {
            let _ = socket_path;
            Self::bind_tcp_with_config(config)
        }
    }

    /// 使用默认配置绑定到 localhost 的 TCP 端口。
    ///
    /// 仅监听 `127.0.0.1` 以阻止远程连接。
    /// 分配随机可用端口。
    ///
    /// # Errors
    ///
    /// 如果绑定失败，返回 `ResidentError::Io`。
    pub fn bind_tcp() -> Result<Self> {
        Self::bind_tcp_with_config(ResidentConfig::default())
    }

    /// 使用默认配置绑定到 localhost 的指定 TCP 端口。
    ///
    /// # Errors
    ///
    /// 如果绑定失败（例如端口已被占用），返回 `ResidentError::Io`。
    pub fn bind_tcp_port(port: u16) -> Result<Self> {
        Self::bind_tcp_port_with_config(port, ResidentConfig::default())
    }

    /// 使用显式配置绑定到 localhost 的 TCP 端口。
    ///
    /// # Errors
    ///
    /// 如果绑定失败，返回 `ResidentError::Io`。
    pub fn bind_tcp_with_config(config: ResidentConfig) -> Result<Self> {
        let transport = super::tcp::TcpTransport::bind_localhost()?;
        Ok(Self::with_transport_and_path(
            Box::new(transport),
            PathBuf::new(),
            config,
        ))
    }

    /// 使用显式配置绑定到 localhost 的指定 TCP 端口。
    ///
    /// # Errors
    ///
    /// 如果绑定失败，返回 `ResidentError::Io`。
    pub fn bind_tcp_port_with_config(port: u16, config: ResidentConfig) -> Result<Self> {
        let transport = super::tcp::TcpTransport::bind_port(port)?;
        Ok(Self::with_transport_and_path(
            Box::new(transport),
            PathBuf::new(),
            config,
        ))
    }

    /// 使用显式 [`Transport`] 实现创建服务器。
    ///
    /// 这是最灵活的构造函数。用于自定义传输或需要完全控制监听器的场景。
    ///
    /// # Errors
    ///
    /// 如果传输初始化失败，返回 `ResidentError::Io`。
    pub fn with_transport(transport: Box<dyn Transport>) -> Result<Self> {
        Ok(Self::with_transport_and_path(
            transport,
            PathBuf::new(),
            ResidentConfig::default(),
        ))
    }

    fn with_transport_and_path(
        transport: Box<dyn Transport>,
        socket_path: PathBuf,
        config: ResidentConfig,
    ) -> Self {
        let state = ServerState {
            sessions: HashMap::new(),
            last_activity: Instant::now(),
            config,
        };

        Self {
            transport,
            socket_path,
            state: Arc::new(Mutex::new(state)),
            next_session_id: Arc::new(AtomicU64::new(1)),
            running: Arc::new(AtomicBool::new(true)),
        }
    }

    /// 运行服务器主循环（阻塞）。
    ///
    /// 接受连接并同步处理每个连接。
    /// 启动空闲超时和自动保存的看门狗线程。
    ///
    /// # Errors
    ///
    /// 如果监听器失败，返回 `ResidentError::Io`。
    pub fn run(&self) -> Result<()> {
        info!(addr = %self.transport.local_addr(), "resident server starting");

        self.transport.set_nonblocking(true)?;

        // 启动看门狗线程
        self.start_idle_watchdog();
        self.start_autosave_watchdog();

        // 收集已生成的处理线程句柄，以便在关闭时 join。
        let mut handles = Vec::new();

        while self.running.load(Ordering::SeqCst) {
            match self.transport.accept() {
                Ok(conn) => {
                    // 设置较短的读取超时，使处理线程可以观察关闭信号。
                    let _ = conn.set_read_timeout(std::time::Duration::from_millis(500));

                    let state = Arc::clone(&self.state);
                    let next_id = Arc::clone(&self.next_session_id);
                    let running = Arc::clone(&self.running);
                    handles.push(std::thread::spawn(move || {
                        Self::handle_connection(conn, &state, &next_id, &running);
                    }));
                }
                Err(ResidentError::Io(ref e)) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // 没有待处理的连接；短暂休眠以避免忙等待
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
                Err(e) => {
                    if self.running.load(Ordering::SeqCst) {
                        error!(error = %e, "accept error");
                    }
                    break;
                }
            }
        }

        // join 所有处理线程，确保在 run() 返回前完成清理。
        for handle in handles {
            let _ = handle.join();
        }

        Ok(())
    }

    /// 优雅关闭服务器。
    ///
    /// 向所有看门狗线程和主循环发送停止信号。
    /// 关闭前保存所有脏会话。
    pub fn shutdown(&self) {
        self.running.store(false, Ordering::SeqCst);
        // 保存脏会话
        if let Ok(mut state) = self.state.lock() {
            for session in state.sessions.values_mut() {
                if session.is_dirty() {
                    let _ = session.save(None);
                }
            }
            state.sessions.clear();
        }
    }

    /// 活跃会话数量。
    #[must_use]
    pub fn session_count(&self) -> usize {
        self.state.lock().map_or(0, |state| state.sessions.len())
    }

    /// 此服务器正在监听的 socket 路径（仅 Unix）。
    ///
    /// 对于 TCP 传输返回空路径。
    #[must_use]
    pub fn socket_path(&self) -> &Path {
        &self.socket_path
    }

    /// 传输地址的人类可读描述（用于日志记录）。
    #[must_use]
    pub fn transport_addr(&self) -> String {
        self.transport.local_addr()
    }

    // --- 私有辅助方法 ---

    fn handle_connection(
        conn: Box<dyn super::transport::Connection>,
        state: &Arc<Mutex<ServerState>>,
        next_session_id: &Arc<AtomicU64>,
        running: &Arc<AtomicBool>,
    ) {
        debug!("new connection accepted");

        let writer_conn = match conn.try_clone() {
            Ok(c) => c,
            Err(e) => {
                error!(error = %e, "failed to clone connection");
                return;
            }
        };

        let mut writer: Box<dyn Write> = writer_conn;
        let mut buf_reader = BufReader::new(conn);

        loop {
            if !running.load(Ordering::SeqCst) {
                break;
            }

            // 读取一行（JSON 请求）。
            // 连接有 500ms 读取超时，因此定期检查 `running`。
            let mut line = String::new();
            match buf_reader.read_line(&mut line) {
                Ok(0) => break, // 客户端断开
                Ok(_) => {}
                Err(ref e)
                    if e.kind() == std::io::ErrorKind::WouldBlock
                        || e.kind() == std::io::ErrorKind::TimedOut =>
                {
                    // 读取超时 -- 循环回来检查 `running`。
                    continue;
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(e) => {
                    warn!(error = %e, "connection read error");
                    break;
                }
            }

            if line.len() > MAX_MESSAGE_BYTES {
                let resp = Response::error("MESSAGE_TOO_LARGE", "request exceeds maximum size");
                let _ = Self::write_response(&mut writer, &resp);
                continue;
            }

            // 解析请求
            let request: Request = match serde_json::from_str(line.trim()) {
                Ok(r) => r,
                Err(e) => {
                    let resp =
                        Response::error("INVALID_JSON", format!("failed to parse request: {e}"));
                    let _ = Self::write_response(&mut writer, &resp);
                    continue;
                }
            };

            // 处理请求
            let is_shutdown = matches!(request, Request::Shutdown);
            let response = Self::handle_request(request, state, next_session_id);
            let _ = Self::write_response(&mut writer, &response);

            if is_shutdown {
                running.store(false, Ordering::SeqCst);
                break;
            }
        }
    }

    #[allow(clippy::too_many_lines)]
    fn handle_request(
        request: Request,
        state: &Arc<Mutex<ServerState>>,
        next_session_id: &Arc<AtomicU64>,
    ) -> Response {
        // 任何请求都重置空闲计时器
        if let Ok(mut s) = state.lock() {
            s.last_activity = Instant::now();
        }

        match request {
            Request::Ping => {
                debug!("ping");
                Response::ok_data(None, ResponseData::Pong)
            }

            Request::Shutdown => {
                info!("shutdown requested");
                if let Ok(mut s) = state.lock() {
                    for session in s.sessions.values_mut() {
                        if session.is_dirty() {
                            let _ = session.save(None);
                        }
                    }
                    s.sessions.clear();
                }
                Response {
                    ok: true,
                    session_id: None,
                    data: Some(ResponseData::None),
                    error_code: None,
                    error_message: None,
                }
            }

            Request::Open { path, mode } => {
                info!(path = %path, ?mode, "open document");
                let path = PathBuf::from(&path);
                let session_id = next_session_id.fetch_add(1, Ordering::SeqCst);

                let Ok(mut s) = state.lock() else {
                    return Response::error("LOCK_ERROR", "state lock poisoned");
                };

                if s.sessions.len() >= s.config.max_sessions {
                    return Response::error(
                        "MAX_SESSIONS",
                        format!("maximum sessions ({}) reached", s.config.max_sessions),
                    );
                }

                match DocumentSession::open(session_id, &path, mode) {
                    Ok(session) => {
                        s.sessions.insert(session_id, session);
                        Response::ok_session(session_id)
                    }
                    Err(e) => Response::error("OPEN_FAILED", e.to_string()),
                }
            }

            Request::ExtractText { session_id, pages } => {
                debug!(session_id, ?pages, "extract text");
                let Ok(mut s) = state.lock() else {
                    return Response::error("LOCK_ERROR", "state lock poisoned");
                };
                let Some(session) = s.sessions.get_mut(&session_id) else {
                    return Response::error(
                        "SESSION_NOT_FOUND",
                        format!("session {session_id} not found"),
                    );
                };
                match session.extract_text(pages.as_ref()) {
                    Ok(text) => {
                        Response::ok_data(Some(session_id), ResponseData::Text { content: text })
                    }
                    Err(e) => Response::error("EXTRACT_FAILED", e.to_string()),
                }
            }

            Request::ExtractMetadata { session_id } => {
                debug!(session_id, "extract metadata");
                let Ok(mut s) = state.lock() else {
                    return Response::error("LOCK_ERROR", "state lock poisoned");
                };
                let Some(session) = s.sessions.get_mut(&session_id) else {
                    return Response::error(
                        "SESSION_NOT_FOUND",
                        format!("session {session_id} not found"),
                    );
                };
                match session.extract_metadata() {
                    Ok(meta) => Response::ok_data(
                        Some(session_id),
                        ResponseData::Metadata { metadata: meta },
                    ),
                    Err(e) => Response::error("METADATA_FAILED", e.to_string()),
                }
            }

            Request::PageCount { session_id } => {
                debug!(session_id, "page count");
                let Ok(mut s) = state.lock() else {
                    return Response::error("LOCK_ERROR", "state lock poisoned");
                };
                let Some(session) = s.sessions.get_mut(&session_id) else {
                    return Response::error(
                        "SESSION_NOT_FOUND",
                        format!("session {session_id} not found"),
                    );
                };
                match session.page_count() {
                    Ok(count) => {
                        Response::ok_data(Some(session_id), ResponseData::PageCount { count })
                    }
                    Err(e) => Response::error("PAGE_COUNT_FAILED", e.to_string()),
                }
            }

            Request::RotatePage {
                session_id,
                page,
                rotation,
            } => {
                info!(session_id, page, rotation, "rotate page");
                let Ok(mut s) = state.lock() else {
                    return Response::error("LOCK_ERROR", "state lock poisoned");
                };
                let Some(session) = s.sessions.get_mut(&session_id) else {
                    return Response::error(
                        "SESSION_NOT_FOUND",
                        format!("session {session_id} not found"),
                    );
                };
                match session.rotate_page(page, rotation) {
                    Ok(()) => Response::ok_session(session_id),
                    Err(e) => Response::error("ROTATE_FAILED", e.to_string()),
                }
            }

            Request::Save { session_id, path } => {
                info!(session_id, ?path, "save document");
                let Ok(mut s) = state.lock() else {
                    return Response::error("LOCK_ERROR", "state lock poisoned");
                };
                let Some(session) = s.sessions.get_mut(&session_id) else {
                    return Response::error(
                        "SESSION_NOT_FOUND",
                        format!("session {session_id} not found"),
                    );
                };
                let save_path = path.as_ref().map(Path::new);
                match session.save(save_path) {
                    Ok(saved_path) => Response::ok_data(
                        Some(session_id),
                        ResponseData::Saved {
                            path: saved_path.to_string_lossy().into_owned(),
                        },
                    ),
                    Err(e) => Response::error("SAVE_FAILED", e.to_string()),
                }
            }

            Request::Close { session_id } => {
                info!(session_id, "close session");
                let Ok(mut s) = state.lock() else {
                    return Response::error("LOCK_ERROR", "state lock poisoned");
                };
                if let Some(session) = s.sessions.get_mut(&session_id)
                    && session.is_dirty()
                {
                    let _ = session.save(None);
                }
                s.sessions.remove(&session_id);
                Response::ok()
            }
        }
    }

    fn write_response(writer: &mut impl Write, response: &Response) -> std::io::Result<()> {
        let mut json = serde_json::to_string(response)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
        json.push('\n');
        writer.write_all(json.as_bytes())?;
        writer.flush()
    }

    fn start_idle_watchdog(&self) {
        let state = Arc::clone(&self.state);
        let running = Arc::clone(&self.running);

        std::thread::spawn(move || {
            while running.load(Ordering::SeqCst) {
                std::thread::sleep(std::time::Duration::from_secs(1));

                if !running.load(Ordering::SeqCst) {
                    break;
                }

                let should_shutdown = state
                    .lock()
                    .is_ok_and(|s| s.last_activity.elapsed() >= s.config.idle_timeout);

                if should_shutdown {
                    info!("idle timeout reached, shutting down");
                    if let Ok(mut s) = state.lock() {
                        for session in s.sessions.values_mut() {
                            if session.is_dirty() {
                                let _ = session.save(None);
                            }
                        }
                        s.sessions.clear();
                    }
                    running.store(false, Ordering::SeqCst);
                    break;
                }
            }
        });
    }

    fn start_autosave_watchdog(&self) {
        let state = Arc::clone(&self.state);
        let running = Arc::clone(&self.running);

        std::thread::spawn(move || {
            while running.load(Ordering::SeqCst) {
                std::thread::sleep(std::time::Duration::from_secs(5));

                if !running.load(Ordering::SeqCst) {
                    break;
                }

                let Ok(mut s) = state.lock() else {
                    continue;
                };

                let autosave_mode = s.config.autosave.clone();

                match autosave_mode {
                    AutosaveMode::Disabled => {}
                    AutosaveMode::Fixed(interval) => {
                        let now = Instant::now();
                        let mut to_save = Vec::new();
                        for (id, session) in &s.sessions {
                            if session.is_dirty()
                                && now.duration_since(session.last_accessed) >= interval
                            {
                                to_save.push(*id);
                            }
                        }
                        for id in to_save {
                            if let Some(session) = s.sessions.get_mut(&id) {
                                let _ = session.save(None);
                            }
                        }
                    }
                    AutosaveMode::Adaptive { .. } => {
                        let now = Instant::now();
                        let mut to_save = Vec::new();
                        for (id, session) in &s.sessions {
                            if session.is_dirty() {
                                let interval = session
                                    .autosave_interval
                                    .unwrap_or(std::time::Duration::from_secs(60));
                                if now.duration_since(session.last_accessed) >= interval {
                                    to_save.push(*id);
                                }
                            }
                        }
                        for id in to_save {
                            if let Some(session) = s.sessions.get_mut(&id) {
                                let _ = session.save(None);
                            }
                        }
                    }
                }
            }
        });
    }
}

impl Drop for ResidentServer {
    fn drop(&mut self) {
        self.shutdown();
        // 清理 socket 文件（仅 Unix）
        if !self.socket_path.as_os_str().is_empty() {
            let _ = std::fs::remove_file(&self.socket_path);
        }
        // 清理端口文件（TCP / Windows）
        super::port::remove_port_file();
    }
}
