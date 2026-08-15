//! Resident PDF daemon server.
//!
//! Maintains open PDF sessions in memory and accepts commands over IPC
//! (Unix socket or TCP, depending on the platform and transport configuration).

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

/// Shared server state, protected by a mutex.
struct ServerState {
    /// Active document sessions.
    sessions: HashMap<SessionId, DocumentSession>,
    /// Last activity timestamp (for idle timeout).
    last_activity: Instant,
    /// Server configuration.
    config: ResidentConfig,
}

/// Resident PDF daemon.
///
/// Maintains multiple open PDF sessions in memory and accepts commands over
/// IPC. The transport layer is pluggable via the [`Transport`] trait:
///
/// - **Unix (Linux/macOS)**: use [`bind()`](ResidentServer::bind) for Unix
///   domain sockets.
/// - **Windows**: use [`bind_tcp()`](ResidentServer::bind_tcp) for TCP
///   localhost.
/// - **Custom**: use [`with_transport()`](ResidentServer::with_transport) for
///   any [`Transport`] implementation.
///
/// # Examples
///
/// ```no_run
/// use easypdf_runtime::resident::{ResidentServer, ResidentConfig};
///
/// let server = ResidentServer::bind("/tmp/easypdf.sock")?;
/// server.run()?;
/// # Ok::<(), easypdf_runtime::resident::ResidentError>(())
/// ```
pub struct ResidentServer {
    /// The transport layer (listener).
    transport: Box<dyn Transport>,
    /// Socket path for cleanup (Unix) or empty (TCP).
    socket_path: PathBuf,
    /// Shared state.
    state: Arc<Mutex<ServerState>>,
    /// Next session id counter.
    next_session_id: Arc<AtomicU64>,
    /// Running flag (for watchdog coordination).
    running: Arc<AtomicBool>,
}

impl ResidentServer {
    /// Bind to the given socket path with default configuration.
    ///
    /// On Unix this creates a Unix domain socket. On non-Unix platforms this
    /// falls back to TCP localhost -- the `socket_path` parameter is ignored
    /// and a random port is assigned.
    ///
    /// # Errors
    ///
    /// Returns `ResidentError::Io` if binding fails.
    pub fn bind(socket_path: impl AsRef<Path>) -> Result<Self> {
        Self::bind_with_config(socket_path, ResidentConfig::default())
    }

    /// Bind to the given socket path with explicit configuration.
    ///
    /// On Unix this creates a Unix domain socket with the configured
    /// `socket_mode`. On non-Unix platforms this falls back to TCP localhost.
    ///
    /// # Errors
    ///
    /// Returns `ResidentError::Io` if binding fails.
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

    /// Bind to a TCP port on localhost with default configuration.
    ///
    /// Listens exclusively on `127.0.0.1` to prevent remote connections.
    /// Assigns a random available port.
    ///
    /// # Errors
    ///
    /// Returns `ResidentError::Io` if binding fails.
    pub fn bind_tcp() -> Result<Self> {
        Self::bind_tcp_with_config(ResidentConfig::default())
    }

    /// Bind to a specific TCP port on localhost with default configuration.
    ///
    /// # Errors
    ///
    /// Returns `ResidentError::Io` if binding fails
    /// (e.g. port already in use).
    pub fn bind_tcp_port(port: u16) -> Result<Self> {
        Self::bind_tcp_port_with_config(port, ResidentConfig::default())
    }

    /// Bind to a TCP port on localhost with explicit configuration.
    ///
    /// # Errors
    ///
    /// Returns `ResidentError::Io` if binding fails.
    pub fn bind_tcp_with_config(config: ResidentConfig) -> Result<Self> {
        let transport = super::tcp::TcpTransport::bind_localhost()?;
        Ok(Self::with_transport_and_path(
            Box::new(transport),
            PathBuf::new(),
            config,
        ))
    }

    /// Bind to a specific TCP port on localhost with explicit configuration.
    ///
    /// # Errors
    ///
    /// Returns `ResidentError::Io` if binding fails.
    pub fn bind_tcp_port_with_config(port: u16, config: ResidentConfig) -> Result<Self> {
        let transport = super::tcp::TcpTransport::bind_port(port)?;
        Ok(Self::with_transport_and_path(
            Box::new(transport),
            PathBuf::new(),
            config,
        ))
    }

    /// Create a server with an explicit [`Transport`] implementation.
    ///
    /// This is the most flexible constructor. Use it for custom transports
    /// or when you need full control over the listener.
    ///
    /// # Errors
    ///
    /// Returns `ResidentError::Io` if the transport fails
    /// to initialize.
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

    /// Run the server main loop (blocking).
    ///
    /// Accepts connections and handles each one synchronously.
    /// Starts watchdog threads for idle timeout and autosave.
    ///
    /// # Errors
    ///
    /// Returns `ResidentError::Io` if the listener fails.
    pub fn run(&self) -> Result<()> {
        info!(addr = %self.transport.local_addr(), "resident server starting");

        self.transport.set_nonblocking(true)?;

        // Start watchdog threads
        self.start_idle_watchdog();
        self.start_autosave_watchdog();

        // Collect handles for spawned handler threads so we can join them on shutdown.
        let mut handles = Vec::new();

        while self.running.load(Ordering::SeqCst) {
            match self.transport.accept() {
                Ok(conn) => {
                    // Set a short read timeout so handler threads can observe shutdown.
                    let _ = conn.set_read_timeout(std::time::Duration::from_millis(500));

                    let state = Arc::clone(&self.state);
                    let next_id = Arc::clone(&self.next_session_id);
                    let running = Arc::clone(&self.running);
                    handles.push(std::thread::spawn(move || {
                        Self::handle_connection(conn, &state, &next_id, &running);
                    }));
                }
                Err(ResidentError::Io(ref e)) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // No connection pending; sleep briefly to avoid busy-spin
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

        // Join all handler threads so they are cleaned up before run() returns.
        for handle in handles {
            let _ = handle.join();
        }

        Ok(())
    }

    /// Gracefully shut down the server.
    ///
    /// Signals all watchdog threads and the main loop to stop.
    /// Saves all dirty sessions before shutting down.
    pub fn shutdown(&self) {
        self.running.store(false, Ordering::SeqCst);
        // Save dirty sessions
        if let Ok(mut state) = self.state.lock() {
            for session in state.sessions.values_mut() {
                if session.is_dirty() {
                    let _ = session.save(None);
                }
            }
            state.sessions.clear();
        }
    }

    /// Number of active sessions.
    #[must_use]
    pub fn session_count(&self) -> usize {
        self.state.lock().map_or(0, |state| state.sessions.len())
    }

    /// The socket path this server is listening on (Unix only).
    ///
    /// Returns an empty path for TCP transports.
    #[must_use]
    pub fn socket_path(&self) -> &Path {
        &self.socket_path
    }

    /// Human-readable description of the transport address (for logging).
    #[must_use]
    pub fn transport_addr(&self) -> String {
        self.transport.local_addr()
    }

    // --- Private helpers ---

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

            // Read one line (JSON request).
            // The connection has a 500ms read timeout so we periodically check `running`.
            let mut line = String::new();
            match buf_reader.read_line(&mut line) {
                Ok(0) => break, // Client disconnected
                Ok(_) => {}
                Err(ref e)
                    if e.kind() == std::io::ErrorKind::WouldBlock
                        || e.kind() == std::io::ErrorKind::TimedOut =>
                {
                    // Read timeout -- loop back to check `running`.
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

            // Parse request
            let request: Request = match serde_json::from_str(line.trim()) {
                Ok(r) => r,
                Err(e) => {
                    let resp =
                        Response::error("INVALID_JSON", format!("failed to parse request: {e}"));
                    let _ = Self::write_response(&mut writer, &resp);
                    continue;
                }
            };

            // Handle request
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
        // Reset idle timer on any request
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
        // Clean up socket file (Unix only)
        if !self.socket_path.as_os_str().is_empty() {
            let _ = std::fs::remove_file(&self.socket_path);
        }
        // Clean up port file (TCP / Windows)
        super::port::remove_port_file();
    }
}
