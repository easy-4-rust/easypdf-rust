//! Resident daemon for easypdf -- keeps PDF documents open in memory.
//!
//! Provides a long-running daemon process that maintains open PDF
//! sessions in memory, exposing operations over IPC.

pub mod client;
pub mod config;
pub mod error;
pub mod port;
pub mod protocol;
pub mod server;
pub mod session;
pub mod tcp;
pub mod transport;
#[cfg(unix)]
pub mod unix;

// Re-export primary types at crate root for ergonomic use.
pub use client::ResidentClient;
pub use config::{AutosaveMode, ResidentConfig};
pub use error::{ResidentError, Result};
pub use protocol::{
    OpenMode, PageRange, PdfMetadataDto, Request, Response, ResponseData, SessionId,
};
pub use server::ResidentServer;
pub use session::DocumentSession;
pub use tcp::TcpTransport;
pub use transport::{Connection, Transport};
#[cfg(unix)]
pub use unix::UnixTransport;

/// Compute the default socket path.
///
/// Uses the system temp directory with a fixed name. For per-file isolation,
/// use [`socket_path_for_file`].
///
/// Only meaningful on Unix platforms.
#[must_use]
pub fn default_socket_path() -> std::path::PathBuf {
    std::env::temp_dir().join("easypdf-resident.sock")
}

/// Compute a per-file socket path based on the PDF file path.
///
/// Hashes the absolute path to produce a unique socket name,
/// preventing collisions between different documents.
///
/// Only meaningful on Unix platforms.
#[must_use]
pub fn socket_path_for_file(pdf_path: &std::path::Path) -> std::path::PathBuf {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let abs_path = std::fs::canonicalize(pdf_path).unwrap_or_else(|_| pdf_path.to_path_buf());
    let mut hasher = DefaultHasher::new();
    abs_path.to_string_lossy().hash(&mut hasher);
    let hash = hasher.finish();
    std::env::temp_dir().join(format!("easypdf-{hash:x}.sock"))
}

/// Start a resident server in the foreground (blocking).
///
/// This is a convenience function that creates and runs a server with
/// default configuration. For custom configuration, use [`ResidentServer`]
/// directly.
///
/// On Unix this uses the given socket path (or the default). On non-Unix
/// platforms it falls back to TCP localhost.
///
/// # Errors
///
/// Returns [`ResidentError`] if the server cannot bind or run.
pub fn serve(socket_path: Option<&std::path::Path>) -> Result<()> {
    let path = socket_path.map_or_else(default_socket_path, std::path::Path::to_path_buf);
    let server = ResidentServer::bind(&path)?;
    eprintln!("easypdf-resident listening on {}", server.transport_addr());
    server.run()
}

/// Try to attach to a running resident daemon.
///
/// Returns `Some(client)` if a daemon is running at the default socket path
/// (Unix) or port file (TCP), or `None` if no daemon is found.
#[must_use]
pub fn try_attach() -> Option<ResidentClient> {
    #[cfg(unix)]
    {
        let path = default_socket_path();
        if ResidentClient::is_running(&path) {
            ResidentClient::connect(&path).ok()
        } else {
            None
        }
    }
    #[cfg(not(unix))]
    {
        ResidentClient::auto_connect().ok()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::similar_names, clippy::items_after_statements)]
    use super::protocol::{OpenMode, PageRange, Request, Response, ResponseData};
    use super::*;
    use std::time::Duration;

    // --- Helper: create a minimal valid PDF file ---

    fn make_test_pdf(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir();
        let path = dir.join(name);
        let mut doc = lopdf::Document::new();

        let content_id = doc.add_object(lopdf::Object::Stream(lopdf::Stream::new(
            lopdf::Dictionary::new(),
            b"BT /F1 12 Tf (Hello Resident) Tj ET".to_vec(),
        )));

        let mut font_dict = lopdf::Dictionary::new();
        font_dict.set("Type", lopdf::Object::Name(b"Font".to_vec()));
        font_dict.set("Subtype", lopdf::Object::Name(b"Type1".to_vec()));
        font_dict.set("BaseFont", lopdf::Object::Name(b"Helvetica".to_vec()));
        let font_id = doc.add_object(lopdf::Object::Dictionary(font_dict));

        let mut resources = lopdf::Dictionary::new();
        let mut fonts = lopdf::Dictionary::new();
        fonts.set("F1", lopdf::Object::Reference(font_id));
        resources.set("Font", lopdf::Object::Dictionary(fonts));
        let resources_id = doc.add_object(lopdf::Object::Dictionary(resources));

        let mut page_dict = lopdf::Dictionary::new();
        page_dict.set("Type", lopdf::Object::Name(b"Page".to_vec()));
        page_dict.set(
            "MediaBox",
            lopdf::Object::Array(vec![0.into(), 0.into(), 595.into(), 842.into()]),
        );
        page_dict.set("Contents", lopdf::Object::Reference(content_id));
        page_dict.set("Resources", lopdf::Object::Reference(resources_id));
        let page_id = doc.add_object(lopdf::Object::Dictionary(page_dict));

        let mut pages_dict = lopdf::Dictionary::new();
        pages_dict.set("Type", lopdf::Object::Name(b"Pages".to_vec()));
        pages_dict.set(
            "Kids",
            lopdf::Object::Array(vec![lopdf::Object::Reference(page_id)]),
        );
        pages_dict.set("Count", lopdf::Object::Integer(1));
        let pages_id = doc.add_object(lopdf::Object::Dictionary(pages_dict));

        let mut catalog = lopdf::Dictionary::new();
        catalog.set("Type", lopdf::Object::Name(b"Catalog".to_vec()));
        catalog.set("Pages", lopdf::Object::Reference(pages_id));
        let catalog_id = doc.add_object(lopdf::Object::Dictionary(catalog));

        doc.trailer
            .set("Root", lopdf::Object::Reference(catalog_id));
        doc.save(&path).unwrap();
        path
    }

    fn unique_socket_path(label: &str) -> std::path::PathBuf {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("easypdf-test-{label}-{ts}.sock"))
    }

    // --- Protocol tests ---

    #[test]
    fn test_request_serialize_open() {
        let req = Request::Open {
            path: "/tmp/test.pdf".to_string(),
            mode: OpenMode::ReadOnly,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("Open"));
        assert!(json.contains("/tmp/test.pdf"));
        let deserialized: Request = serde_json::from_str(&json).unwrap();
        match deserialized {
            Request::Open { path, mode } => {
                assert_eq!(path, "/tmp/test.pdf");
                assert_eq!(mode, OpenMode::ReadOnly);
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_request_serialize_ping() {
        let req = Request::Ping;
        let json = serde_json::to_string(&req).unwrap();
        let deserialized: Request = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, Request::Ping));
    }

    #[test]
    fn test_request_serialize_shutdown() {
        let req = Request::Shutdown;
        let json = serde_json::to_string(&req).unwrap();
        let deserialized: Request = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, Request::Shutdown));
    }

    #[test]
    fn test_request_serialize_extract_text_with_pages() {
        let req = Request::ExtractText {
            session_id: 42,
            pages: Some(PageRange {
                start: 0,
                end: Some(5),
            }),
        };
        let json = serde_json::to_string(&req).unwrap();
        let deserialized: Request = serde_json::from_str(&json).unwrap();
        match deserialized {
            Request::ExtractText { session_id, pages } => {
                assert_eq!(session_id, 42);
                let p = pages.unwrap();
                assert_eq!(p.start, 0);
                assert_eq!(p.end, Some(5));
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_response_serialize_ok() {
        let resp = Response::ok();
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains(r#""ok":true"#));
        let deserialized: Response = serde_json::from_str(&json).unwrap();
        assert!(deserialized.ok);
        assert!(deserialized.session_id.is_none());
        assert!(deserialized.data.is_none());
    }

    #[test]
    fn test_response_serialize_with_data() {
        let resp = Response::ok_data(Some(7), ResponseData::PageCount { count: 42 });
        let json = serde_json::to_string(&resp).unwrap();
        let deserialized: Response = serde_json::from_str(&json).unwrap();
        assert!(deserialized.ok);
        assert_eq!(deserialized.session_id, Some(7));
        match deserialized.data.unwrap() {
            ResponseData::PageCount { count } => assert_eq!(count, 42),
            _ => panic!("wrong data variant"),
        }
    }

    #[test]
    fn test_response_serialize_error() {
        let resp = Response::error("NOT_FOUND", "session 99 not found");
        let json = serde_json::to_string(&resp).unwrap();
        let deserialized: Response = serde_json::from_str(&json).unwrap();
        assert!(!deserialized.ok);
        assert_eq!(deserialized.error_code.as_deref(), Some("NOT_FOUND"));
        assert!(deserialized.error_message.unwrap().contains("99"));
    }

    #[test]
    fn test_open_mode_serialize_roundtrip() {
        for mode in [OpenMode::ReadOnly, OpenMode::ReadWrite] {
            let json = serde_json::to_string(&mode).unwrap();
            let deserialized: OpenMode = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized, mode);
        }
    }

    // --- Config tests ---

    #[test]
    fn test_config_default() {
        let config = ResidentConfig::default();
        assert_eq!(config.idle_timeout, Duration::from_secs(300));
        assert_eq!(config.max_sessions, 16);
    }

    #[test]
    fn test_autosave_adaptive_interval() {
        let mode = AutosaveMode::Adaptive {
            min_interval: Duration::from_secs(10),
            max_interval: Duration::from_secs(300),
            initial: Duration::from_secs(60),
        };

        // First sample: 1 second save -> interval = 4 * 1.0 = 4s -> clamped to 10s
        let interval = mode.next_adaptive_interval(None, Duration::from_secs(1));
        assert_eq!(interval, Some(Duration::from_secs(10)));

        // With previous EMA of 5s: 0.3 * 1 + 0.7 * 5 = 3.8 -> 4 * 3.8 = 15.2
        let interval = mode.next_adaptive_interval(Some(5.0), Duration::from_secs(1));
        assert!(interval.is_some());
        let d = interval.unwrap();
        assert!(d >= Duration::from_secs(10));
        assert!(d <= Duration::from_secs(300));
    }

    #[test]
    fn test_autosave_disabled_no_interval() {
        let mode = AutosaveMode::Disabled;
        assert!(
            mode.next_adaptive_interval(None, Duration::from_secs(1))
                .is_none()
        );
        assert!(mode.initial_interval().is_none());
    }

    #[test]
    fn test_autosave_fixed_interval() {
        let mode = AutosaveMode::Fixed(Duration::from_secs(30));
        assert!(
            mode.next_adaptive_interval(None, Duration::from_secs(1))
                .is_none()
        );
        assert_eq!(mode.initial_interval(), Some(Duration::from_secs(30)));
    }

    // --- Integration tests: Unix server/client ---

    #[test]
    fn test_server_ping_shutdown() {
        let socket = unique_socket_path("ping");
        let server = ResidentServer::bind(&socket).unwrap();

        let socket_clone = socket.clone();
        let handle = std::thread::spawn(move || {
            // Give server time to start
            std::thread::sleep(Duration::from_millis(100));

            let client = ResidentClient::connect(&socket_clone).unwrap();
            client.ping().unwrap();
            client.shutdown().unwrap();
        });

        server.run().unwrap();
        handle.join().unwrap();
        let _ = std::fs::remove_file(&socket);
    }

    #[test]
    fn test_open_extract_text_page_count_close() {
        let socket = unique_socket_path("open");
        let pdf_path = make_test_pdf("resident_test_open.pdf");

        let server = ResidentServer::bind(&socket).unwrap();
        let socket_clone = socket.clone();
        let pdf_clone = pdf_path.clone();

        let handle = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(100));

            let client = ResidentClient::connect(&socket_clone).unwrap();

            // Open
            let session = client
                .open(pdf_clone.to_str().unwrap(), OpenMode::ReadOnly)
                .unwrap();
            assert!(session > 0);

            // Page count
            let count = client.page_count(session).unwrap();
            assert_eq!(count, 1);

            // Extract text
            let text = client.extract_text(session, None).unwrap();
            assert!(text.contains("Hello Resident"));

            // Extract metadata
            let meta = client.extract_metadata(session).unwrap();
            // Our test PDF has no metadata set
            assert!(meta.title.is_none());

            // Close
            client.close(session).unwrap();

            // Shutdown
            client.shutdown().unwrap();
        });

        server.run().unwrap();
        handle.join().unwrap();
        let _ = std::fs::remove_file(&socket);
        let _ = std::fs::remove_file(&pdf_path);
    }

    #[test]
    fn test_multi_session() {
        let socket = unique_socket_path("multi");
        let pdf1 = make_test_pdf("resident_multi1.pdf");
        let pdf2 = make_test_pdf("resident_multi2.pdf");

        let config = ResidentConfig {
            max_sessions: 8,
            ..Default::default()
        };
        let server = ResidentServer::bind_with_config(&socket, config).unwrap();
        let socket_clone = socket.clone();
        let pdf1_clone = pdf1.clone();
        let pdf2_clone = pdf2.clone();

        let handle = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(100));

            let client = ResidentClient::connect(&socket_clone).unwrap();

            // Open two sessions
            let s1 = client
                .open(pdf1_clone.to_str().unwrap(), OpenMode::ReadOnly)
                .unwrap();
            let s2 = client
                .open(pdf2_clone.to_str().unwrap(), OpenMode::ReadOnly)
                .unwrap();
            assert_ne!(s1, s2);

            // Both should return page count
            assert_eq!(client.page_count(s1).unwrap(), 1);
            assert_eq!(client.page_count(s2).unwrap(), 1);

            // Both should return text
            let t1 = client.extract_text(s1, None).unwrap();
            let t2 = client.extract_text(s2, None).unwrap();
            assert!(t1.contains("Hello Resident"));
            assert!(t2.contains("Hello Resident"));

            // Close both
            client.close(s1).unwrap();
            client.close(s2).unwrap();

            client.shutdown().unwrap();
        });

        server.run().unwrap();
        handle.join().unwrap();
        let _ = std::fs::remove_file(&socket);
        let _ = std::fs::remove_file(&pdf1);
        let _ = std::fs::remove_file(&pdf2);
    }

    #[test]
    fn test_session_not_found() {
        let socket = unique_socket_path("notfound");
        let server = ResidentServer::bind(&socket).unwrap();
        let socket_clone = socket.clone();

        let handle = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(100));

            let client = ResidentClient::connect(&socket_clone).unwrap();

            // Try to get page count for non-existent session
            let result = client.page_count(999);
            assert!(result.is_err());
            match result.unwrap_err() {
                ResidentError::Server { code, .. } => assert_eq!(code, "SESSION_NOT_FOUND"),
                other => panic!("expected Server error, got {other:?}"),
            }

            client.shutdown().unwrap();
        });

        server.run().unwrap();
        handle.join().unwrap();
        let _ = std::fs::remove_file(&socket);
    }

    #[test]
    fn test_idle_timeout() {
        let socket = unique_socket_path("idle");
        let config = ResidentConfig {
            idle_timeout: Duration::from_secs(2),
            ..Default::default()
        };
        let server = ResidentServer::bind_with_config(&socket, config).unwrap();
        let socket_clone = socket.clone();

        let handle = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(100));

            let client = ResidentClient::connect(&socket_clone).unwrap();
            client.ping().unwrap();

            // Wait for idle timeout to trigger
            std::thread::sleep(Duration::from_secs(4));

            // Server should have shut down; connection should fail
            let result = client.ping();
            assert!(result.is_err());
        });

        server.run().unwrap();
        handle.join().unwrap();
        let _ = std::fs::remove_file(&socket);
    }

    #[test]
    fn test_client_not_running() {
        let socket = unique_socket_path("notrunning");
        assert!(!ResidentClient::is_running(&socket));
        let result = ResidentClient::connect(&socket);
        assert!(result.is_err());
        match result.unwrap_err() {
            ResidentError::ServerNotRunning(_) => {}
            other => panic!("expected ServerNotRunning, got {other:?}"),
        }
    }

    #[test]
    fn test_max_sessions_limit() {
        let socket = unique_socket_path("maxsess");
        let pdf = make_test_pdf("resident_maxsess.pdf");

        let config = ResidentConfig {
            max_sessions: 2,
            ..Default::default()
        };
        let server = ResidentServer::bind_with_config(&socket, config).unwrap();
        let socket_clone = socket.clone();
        let pdf_clone = pdf.clone();

        let handle = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(100));

            let client = ResidentClient::connect(&socket_clone).unwrap();

            let s1 = client
                .open(pdf_clone.to_str().unwrap(), OpenMode::ReadOnly)
                .unwrap();
            let s2 = client
                .open(pdf_clone.to_str().unwrap(), OpenMode::ReadOnly)
                .unwrap();
            assert_ne!(s1, s2);

            // Third should fail
            let result = client.open(pdf_clone.to_str().unwrap(), OpenMode::ReadOnly);
            assert!(result.is_err());
            match result.unwrap_err() {
                ResidentError::Server { code, .. } => assert_eq!(code, "MAX_SESSIONS"),
                other => panic!("expected MAX_SESSIONS error, got {other:?}"),
            }

            client.close(s1).unwrap();
            client.close(s2).unwrap();
            client.shutdown().unwrap();
        });

        server.run().unwrap();
        handle.join().unwrap();
        let _ = std::fs::remove_file(&socket);
        let _ = std::fs::remove_file(&pdf);
    }

    #[test]
    fn test_extract_text_with_page_range() {
        let socket = unique_socket_path("pagerange");
        let pdf = make_test_pdf("resident_pagerange.pdf");

        let server = ResidentServer::bind(&socket).unwrap();
        let socket_clone = socket.clone();
        let pdf_clone = pdf.clone();

        let handle = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(100));

            let client = ResidentClient::connect(&socket_clone).unwrap();
            let session = client
                .open(pdf_clone.to_str().unwrap(), OpenMode::ReadOnly)
                .unwrap();

            // Extract with page range
            let text = client
                .extract_text(
                    session,
                    Some(PageRange {
                        start: 0,
                        end: Some(1),
                    }),
                )
                .unwrap();
            assert!(text.contains("Hello Resident"));

            client.close(session).unwrap();
            client.shutdown().unwrap();
        });

        server.run().unwrap();
        handle.join().unwrap();
        let _ = std::fs::remove_file(&socket);
        let _ = std::fs::remove_file(&pdf);
    }

    // --- Convenience function tests ---

    #[test]
    fn test_default_socket_path() {
        let path = default_socket_path();
        assert!(path.to_string_lossy().contains("easypdf-resident"));
    }

    #[test]
    fn test_socket_path_for_file_deterministic() {
        let p = std::path::Path::new("/tmp/test.pdf");
        let a = socket_path_for_file(p);
        let b = socket_path_for_file(p);
        assert_eq!(a, b);
    }

    #[test]
    fn test_socket_path_for_file_different_paths() {
        let a = socket_path_for_file(std::path::Path::new("/tmp/a.pdf"));
        let b = socket_path_for_file(std::path::Path::new("/tmp/b.pdf"));
        assert_ne!(a, b);
    }

    // --- TCP transport integration tests (cross-platform) ---

    #[test]
    fn test_tcp_server_ping_shutdown() {
        let server = ResidentServer::bind_tcp().unwrap();
        let addr = server.transport_addr();

        // Extract port from "127.0.0.1:PORT"
        let port: u16 = addr.split(':').nth(1).unwrap().parse().unwrap();
        let socket_addr: std::net::SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();

        let handle = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(100));

            let client = ResidentClient::connect_tcp(socket_addr).unwrap();
            client.ping().unwrap();
            client.shutdown().unwrap();
        });

        server.run().unwrap();
        handle.join().unwrap();
    }

    #[test]
    fn test_tcp_open_extract_close() {
        let pdf_path = make_test_pdf("resident_tcp_test.pdf");

        let server = ResidentServer::bind_tcp().unwrap();
        let addr = server.transport_addr();
        let port: u16 = addr.split(':').nth(1).unwrap().parse().unwrap();
        let socket_addr: std::net::SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();
        let pdf_clone = pdf_path.clone();

        let handle = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(100));

            let client = ResidentClient::connect_tcp(socket_addr).unwrap();

            // Open
            let session = client
                .open(pdf_clone.to_str().unwrap(), OpenMode::ReadOnly)
                .unwrap();
            assert!(session > 0);

            // Page count
            let count = client.page_count(session).unwrap();
            assert_eq!(count, 1);

            // Extract text
            let text = client.extract_text(session, None).unwrap();
            assert!(text.contains("Hello Resident"));

            // Close
            client.close(session).unwrap();

            // Shutdown
            client.shutdown().unwrap();
        });

        server.run().unwrap();
        handle.join().unwrap();
        let _ = std::fs::remove_file(&pdf_path);
    }

    #[test]
    fn test_tcp_transport_bind_and_accept() {
        use super::tcp::TcpTransport;
        use super::transport::Transport;

        let transport = TcpTransport::bind_localhost().unwrap();
        let port = transport.port();
        assert!(port > 0);
        assert_eq!(transport.local_addr(), format!("127.0.0.1:{port}"));

        // Connect and send a ping via raw TCP
        let handle = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(50));
            let mut stream = std::net::TcpStream::connect(format!("127.0.0.1:{port}")).unwrap();
            use std::io::Write;
            // Send a ping request
            let req = Request::Ping;
            let mut json = serde_json::to_string(&req).unwrap();
            json.push('\n');
            stream.write_all(json.as_bytes()).unwrap();
            stream.flush().unwrap();

            // Read response
            use std::io::BufRead;
            let mut reader = std::io::BufReader::new(&stream);
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            let resp: Response = serde_json::from_str(line.trim()).unwrap();
            assert!(resp.ok);
        });

        let mut conn = transport.accept().unwrap();
        // Verify connection was accepted
        assert!(!conn.peer_addr().is_empty());

        // Read request from connection and write response
        use std::io::{BufRead, Write};
        let reader_conn = conn.try_clone().unwrap();
        let mut reader = std::io::BufReader::new(reader_conn);
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();

        let request: Request = serde_json::from_str(line.trim()).unwrap();
        assert!(matches!(request, Request::Ping));

        let response = Response::ok_data(None, ResponseData::Pong);
        let mut resp_json = serde_json::to_string(&response).unwrap();
        resp_json.push('\n');
        conn.write_all(resp_json.as_bytes()).unwrap();
        conn.flush().unwrap();

        handle.join().unwrap();
    }
}
