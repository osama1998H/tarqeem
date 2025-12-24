//! DAP Server - Debug Adapter Protocol server implementation
//!
//! Provides TCP and stdio transports for IDE integration.
//! VS Code and other DAP clients communicate with this server.
//!
//! ## Architecture
//!
//! ```text
//! ┌──────────────────────┐     ┌──────────────────────┐
//! │   IDE (VS Code)      │────▶│     DapServer        │
//! │   عميل التصحيح       │◀────│     خادم التصحيح     │
//! └──────────────────────┘     └──────────────────────┘
//!          │                            │
//!          │  DAP Protocol              │
//!          │  (Content-Length + JSON)   │
//!          │                            ▼
//!          │                   ┌──────────────────────┐
//!          └──────────────────▶│    DapAdapter        │
//!                              │    معالج الطلبات    │
//!                              └──────────────────────┘
//! ```

use std::io::{self, BufRead, BufReader, BufWriter, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader as TokioBufReader};
use tokio::net::TcpListener as TokioTcpListener;
use tokio::sync::mpsc;

use super::adapter::{DapAdapter, DapEvent, DapRequest, DapResponse};
use super::{DebugError, DebugResult};

#[derive(Debug)]
pub enum DapMessage {
    Request(DapRequest),
    Response(DapResponse),
    Event(DapEvent),
}

impl DapMessage {
    pub fn to_json(&self) -> serde_json::Result<String> {
        match self {
            DapMessage::Request(r) => serde_json::to_string(r),
            DapMessage::Response(r) => serde_json::to_string(r),
            DapMessage::Event(e) => serde_json::to_string(e),
        }
    }

    pub fn from_json(json: &str) -> serde_json::Result<Self> {
        let value: serde_json::Value = serde_json::from_str(json)?;

        let msg_type = value
            .get("type")
            .and_then(|t| t.as_str())
            .unwrap_or("unknown");

        match msg_type {
            "request" => Ok(DapMessage::Request(serde_json::from_value(value)?)),
            "response" => Ok(DapMessage::Response(serde_json::from_value(value)?)),
            "event" => Ok(DapMessage::Event(serde_json::from_value(value)?)),
            _ => Ok(DapMessage::Request(serde_json::from_value(value)?)),
        }
    }
}

#[derive(Debug)]
pub struct TransportError {
    pub message: String,
    pub message_ar: String,
}

impl TransportError {
    pub fn new(message: impl Into<String>, message_ar: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            message_ar: message_ar.into(),
        }
    }

    pub fn io_error(err: io::Error) -> Self {
        Self {
            message: format!("I/O error: {}", err),
            message_ar: format!("خطأ في الإدخال/الإخراج: {}", err),
        }
    }

    pub fn parse_error(msg: &str) -> Self {
        Self {
            message: format!("Parse error: {}", msg),
            message_ar: format!("خطأ في التحليل: {}", msg),
        }
    }

    pub fn connection_closed() -> Self {
        Self {
            message: "Connection closed".to_string(),
            message_ar: "تم إغلاق الاتصال".to_string(),
        }
    }
}

impl std::fmt::Display for TransportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for TransportError {}

impl From<io::Error> for TransportError {
    fn from(err: io::Error) -> Self {
        TransportError::io_error(err)
    }
}

impl From<serde_json::Error> for TransportError {
    fn from(err: serde_json::Error) -> Self {
        TransportError::parse_error(&err.to_string())
    }
}

pub type TransportResult<T> = Result<T, TransportError>;

pub struct DapProtocol;

impl DapProtocol {
    pub fn read_message<R: BufRead>(reader: &mut R) -> TransportResult<DapMessage> {
        let content_length = Self::read_headers(reader)?;
        let json = Self::read_content(reader, content_length)?;
        let message = DapMessage::from_json(&json)?;
        Ok(message)
    }

    pub fn write_message<W: Write>(writer: &mut W, message: &DapMessage) -> TransportResult<()> {
        let json = message.to_json()?;
        let header = format!("Content-Length: {}\r\n\r\n", json.len());

        writer.write_all(header.as_bytes())?;
        writer.write_all(json.as_bytes())?;
        writer.flush()?;

        Ok(())
    }

    fn read_headers<R: BufRead>(reader: &mut R) -> TransportResult<usize> {
        let mut content_length: Option<usize> = None;
        let mut line = String::new();

        loop {
            line.clear();
            let bytes_read = reader.read_line(&mut line)?;

            if bytes_read == 0 {
                return Err(TransportError::connection_closed());
            }

            let trimmed = line.trim();
            if trimmed.is_empty() {
                break;
            }

            if let Some(len_str) = trimmed.strip_prefix("Content-Length:") {
                let len_str = len_str.trim();
                content_length = Some(len_str.parse().map_err(|_| {
                    TransportError::parse_error(&format!("Invalid Content-Length: {}", len_str))
                })?);
            }
        }

        content_length.ok_or_else(|| TransportError::parse_error("Missing Content-Length header"))
    }

    fn read_content<R: BufRead>(reader: &mut R, length: usize) -> TransportResult<String> {
        let mut buffer = vec![0u8; length];
        reader.read_exact(&mut buffer)?;
        String::from_utf8(buffer)
            .map_err(|_| TransportError::parse_error("Invalid UTF-8 in message body"))
    }
}

pub struct DapProtocolAsync;

impl DapProtocolAsync {
    pub async fn read_message<R>(reader: &mut TokioBufReader<R>) -> TransportResult<DapMessage>
    where
        R: tokio::io::AsyncRead + Unpin,
    {
        let content_length = Self::read_headers(reader).await?;
        let json = Self::read_content(reader, content_length).await?;
        let message = DapMessage::from_json(&json)?;
        Ok(message)
    }

    pub async fn write_message<W>(writer: &mut W, message: &DapMessage) -> TransportResult<()>
    where
        W: tokio::io::AsyncWrite + Unpin,
    {
        let json = message.to_json()?;
        let header = format!("Content-Length: {}\r\n\r\n", json.len());

        writer.write_all(header.as_bytes()).await?;
        writer.write_all(json.as_bytes()).await?;
        writer.flush().await?;

        Ok(())
    }

    async fn read_headers<R>(reader: &mut TokioBufReader<R>) -> TransportResult<usize>
    where
        R: tokio::io::AsyncRead + Unpin,
    {
        let mut content_length: Option<usize> = None;
        let mut line = String::new();

        loop {
            line.clear();
            let bytes_read = reader.read_line(&mut line).await?;

            if bytes_read == 0 {
                return Err(TransportError::connection_closed());
            }

            let trimmed = line.trim();
            if trimmed.is_empty() {
                break;
            }

            if let Some(len_str) = trimmed.strip_prefix("Content-Length:") {
                let len_str = len_str.trim();
                content_length = Some(len_str.parse().map_err(|_| {
                    TransportError::parse_error(&format!("Invalid Content-Length: {}", len_str))
                })?);
            }
        }

        content_length.ok_or_else(|| TransportError::parse_error("Missing Content-Length header"))
    }

    async fn read_content<R>(
        reader: &mut TokioBufReader<R>,
        length: usize,
    ) -> TransportResult<String>
    where
        R: tokio::io::AsyncRead + Unpin,
    {
        let mut buffer = vec![0u8; length];
        reader.read_exact(&mut buffer).await?;
        String::from_utf8(buffer)
            .map_err(|_| TransportError::parse_error("Invalid UTF-8 in message body"))
    }
}

#[derive(Debug, Clone)]
pub struct DapServerConfig {
    pub program: PathBuf,
    pub stop_on_entry: bool,
    pub use_arabic: bool,
}

impl Default for DapServerConfig {
    fn default() -> Self {
        Self {
            program: PathBuf::new(),
            stop_on_entry: false,
            use_arabic: false,
        }
    }
}

pub struct DapServer {
    adapter: DapAdapter,
    shutdown: Arc<AtomicBool>,
}

impl DapServer {
    pub fn new() -> Self {
        Self {
            adapter: DapAdapter::new(),
            shutdown: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn shutdown_handle(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.shutdown)
    }

    pub fn shutdown(&self) {
        self.shutdown.store(true, Ordering::SeqCst);
    }

    fn is_shutdown(&self) -> bool {
        self.shutdown.load(Ordering::SeqCst)
    }

    pub fn run_tcp(mut self, port: u16) -> TransportResult<()> {
        let addr = format!("127.0.0.1:{}", port);
        let listener = TcpListener::bind(&addr)?;

        println!(
            "DAP server listening on {} | خادم التصحيح يستمع على {}",
            addr, addr
        );

        // Accept one connection
        let (stream, peer_addr) = listener.accept()?;
        println!(
            "Client connected from {} | عميل متصل من {}",
            peer_addr, peer_addr
        );

        self.handle_connection_sync(stream)
    }

    fn handle_connection_sync(&mut self, stream: TcpStream) -> TransportResult<()> {
        let mut reader = BufReader::new(stream.try_clone()?);
        let mut writer = BufWriter::new(stream);

        loop {
            if self.is_shutdown() {
                break;
            }

            // Read request
            let message = match DapProtocol::read_message(&mut reader) {
                Ok(msg) => msg,
                Err(e) => {
                    if matches!(
                        e.message.as_str(),
                        "Connection closed" | "I/O error: unexpected end of file"
                    ) {
                        break;
                    }
                    return Err(e);
                }
            };

            // Handle request
            if let DapMessage::Request(request) = message {
                let response = self.adapter.handle_request(request);

                // Send response
                DapProtocol::write_message(&mut writer, &DapMessage::Response(response))?;

                // Send pending events
                for event in self.adapter.take_events() {
                    DapProtocol::write_message(&mut writer, &DapMessage::Event(event))?;
                }
            }
        }

        println!("DAP session ended | انتهت جلسة التصحيح");
        Ok(())
    }

    pub async fn run_tcp_async(mut self, port: u16) -> TransportResult<()> {
        let addr = format!("127.0.0.1:{}", port);
        let listener = TokioTcpListener::bind(&addr).await?;

        println!(
            "DAP server listening on {} | خادم التصحيح يستمع على {}",
            addr, addr
        );

        // Accept one connection
        let (stream, peer_addr) = listener.accept().await?;
        println!(
            "Client connected from {} | عميل متصل من {}",
            peer_addr, peer_addr
        );

        self.handle_connection_async(stream).await
    }

    async fn handle_connection_async(
        &mut self,
        stream: tokio::net::TcpStream,
    ) -> TransportResult<()> {
        let (read_half, write_half) = stream.into_split();
        let mut reader = TokioBufReader::new(read_half);
        let mut writer = tokio::io::BufWriter::new(write_half);

        loop {
            if self.is_shutdown() {
                break;
            }

            // Read request
            let message = match DapProtocolAsync::read_message(&mut reader).await {
                Ok(msg) => msg,
                Err(e) => {
                    if matches!(
                        e.message.as_str(),
                        "Connection closed" | "I/O error: unexpected end of file"
                    ) {
                        break;
                    }
                    return Err(e);
                }
            };

            // Handle request
            if let DapMessage::Request(request) = message {
                let response = self.adapter.handle_request(request);

                // Send response
                DapProtocolAsync::write_message(&mut writer, &DapMessage::Response(response))
                    .await?;

                // Send pending events
                for event in self.adapter.take_events() {
                    DapProtocolAsync::write_message(&mut writer, &DapMessage::Event(event)).await?;
                }
            }
        }

        println!("DAP session ended | انتهت جلسة التصحيح");
        Ok(())
    }

    pub fn run_stdio(mut self) -> TransportResult<()> {
        let stdin = io::stdin();
        let stdout = io::stdout();

        let mut reader = BufReader::new(stdin.lock());
        let mut writer = BufWriter::new(stdout.lock());

        loop {
            if self.is_shutdown() {
                break;
            }

            // Read request
            let message = match DapProtocol::read_message(&mut reader) {
                Ok(msg) => msg,
                Err(e) => {
                    if matches!(
                        e.message.as_str(),
                        "Connection closed" | "I/O error: unexpected end of file"
                    ) {
                        break;
                    }
                    return Err(e);
                }
            };

            // Handle request
            if let DapMessage::Request(request) = message {
                let response = self.adapter.handle_request(request);

                // Send response
                DapProtocol::write_message(&mut writer, &DapMessage::Response(response))?;

                // Send pending events
                for event in self.adapter.take_events() {
                    DapProtocol::write_message(&mut writer, &DapMessage::Event(event))?;
                }
            }
        }

        Ok(())
    }

    pub async fn run_stdio_async(mut self) -> TransportResult<()> {
        let stdin = tokio::io::stdin();
        let stdout = tokio::io::stdout();

        let mut reader = TokioBufReader::new(stdin);
        let mut writer = tokio::io::BufWriter::new(stdout);

        loop {
            if self.is_shutdown() {
                break;
            }

            // Read request
            let message = match DapProtocolAsync::read_message(&mut reader).await {
                Ok(msg) => msg,
                Err(e) => {
                    if matches!(
                        e.message.as_str(),
                        "Connection closed" | "I/O error: unexpected end of file"
                    ) {
                        break;
                    }
                    return Err(e);
                }
            };

            // Handle request
            if let DapMessage::Request(request) = message {
                let response = self.adapter.handle_request(request);

                // Send response
                DapProtocolAsync::write_message(&mut writer, &DapMessage::Response(response))
                    .await?;

                // Send pending events
                for event in self.adapter.take_events() {
                    DapProtocolAsync::write_message(&mut writer, &DapMessage::Event(event)).await?;
                }
            }
        }

        Ok(())
    }
}

impl Default for DapServer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_dap_message_serialize() {
        let request = DapRequest {
            seq: 1,
            msg_type: "request".to_string(),
            command: "initialize".to_string(),
            arguments: serde_json::json!({"clientID": "test"}),
        };

        let message = DapMessage::Request(request);
        let json = message.to_json().unwrap();

        assert!(json.contains("\"seq\":1"));
        assert!(json.contains("\"command\":\"initialize\""));
    }

    #[test]
    fn test_dap_message_deserialize() {
        let json = r#"{"seq":1,"type":"request","command":"initialize","arguments":{}}"#;
        let message = DapMessage::from_json(json).unwrap();

        if let DapMessage::Request(req) = message {
            assert_eq!(req.seq, 1);
            assert_eq!(req.command, "initialize");
        } else {
            panic!("Expected Request");
        }
    }

    #[test]
    fn test_protocol_read_headers() {
        let input = "Content-Length: 42\r\n\r\n";
        let mut reader = BufReader::new(Cursor::new(input));

        let length = DapProtocol::read_headers(&mut reader).unwrap();
        assert_eq!(length, 42);
    }

    #[test]
    fn test_protocol_read_headers_with_extra() {
        let input = "Content-Length: 100\r\nContent-Type: application/json\r\n\r\n";
        let mut reader = BufReader::new(Cursor::new(input));

        let length = DapProtocol::read_headers(&mut reader).unwrap();
        assert_eq!(length, 100);
    }

    #[test]
    fn test_protocol_read_message() {
        let json = r#"{"seq":1,"type":"request","command":"test","arguments":{}}"#;
        let input = format!("Content-Length: {}\r\n\r\n{}", json.len(), json);
        let mut reader = BufReader::new(Cursor::new(input));

        let message = DapProtocol::read_message(&mut reader).unwrap();

        if let DapMessage::Request(req) = message {
            assert_eq!(req.command, "test");
        } else {
            panic!("Expected Request");
        }
    }

    #[test]
    fn test_protocol_write_message() {
        let request = DapRequest {
            seq: 1,
            msg_type: "request".to_string(),
            command: "test".to_string(),
            arguments: serde_json::json!({}),
        };

        let message = DapMessage::Request(request);
        let mut output = Vec::new();

        DapProtocol::write_message(&mut output, &message).unwrap();

        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.starts_with("Content-Length:"));
        assert!(output_str.contains("\"command\":\"test\""));
    }

    #[test]
    fn test_transport_error_bilingual() {
        let err = TransportError::connection_closed();
        assert_eq!(err.message, "Connection closed");
        assert_eq!(err.message_ar, "تم إغلاق الاتصال");
    }

    #[test]
    fn test_protocol_roundtrip() {
        let original = DapRequest {
            seq: 42,
            msg_type: "request".to_string(),
            command: "setBreakpoints".to_string(),
            arguments: serde_json::json!({
                "source": {"path": "/test/file.trq"},
                "breakpoints": [{"line": 10}]
            }),
        };

        let message = DapMessage::Request(original);
        let mut buffer = Vec::new();

        DapProtocol::write_message(&mut buffer, &message).unwrap();

        let mut reader = BufReader::new(Cursor::new(buffer));
        let parsed = DapProtocol::read_message(&mut reader).unwrap();

        if let DapMessage::Request(req) = parsed {
            assert_eq!(req.seq, 42);
            assert_eq!(req.command, "setBreakpoints");
        } else {
            panic!("Expected Request");
        }
    }

    #[test]
    fn test_dap_server_creation() {
        let server = DapServer::new();
        assert!(!server.is_shutdown());

        server.shutdown();
        assert!(server.is_shutdown());
    }

    #[test]
    fn test_shutdown_handle() {
        let server = DapServer::new();
        let handle = server.shutdown_handle();

        assert!(!handle.load(Ordering::SeqCst));
        handle.store(true, Ordering::SeqCst);
        assert!(server.is_shutdown());
    }
}
