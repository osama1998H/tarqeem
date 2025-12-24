//! DAP Integration Tests
//!
//! Tests for the Debug Adapter Protocol server implementation.
//! These tests verify the complete DAP communication flow.
//!
//! اختبارات تكامل بروتوكول محول التصحيح

use std::io::{BufRead, BufReader, BufWriter, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use serde_json::json;

use tarqeem::debug::{DapEvent, DapMessage, DapProtocol, DapRequest, DapResponse, DapServer};

/// Helper to get the project root directory
fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Find an available port for testing
fn find_available_port() -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.local_addr().unwrap().port()
}

/// DAP test client for sending requests and receiving responses
struct DapTestClient {
    reader: BufReader<TcpStream>,
    writer: BufWriter<TcpStream>,
    seq: i64,
}

impl DapTestClient {
    /// Connect to a DAP server
    fn connect(port: u16) -> Result<Self, std::io::Error> {
        // Retry connection a few times as server may still be starting
        let mut attempts = 0;
        let stream = loop {
            match TcpStream::connect(format!("127.0.0.1:{}", port)) {
                Ok(s) => break s,
                Err(e) if attempts < 10 => {
                    attempts += 1;
                    thread::sleep(Duration::from_millis(100));
                    continue;
                }
                Err(e) => return Err(e),
            }
        };

        stream.set_read_timeout(Some(Duration::from_secs(5)))?;
        stream.set_write_timeout(Some(Duration::from_secs(5)))?;

        let reader = BufReader::new(stream.try_clone()?);
        let writer = BufWriter::new(stream);

        Ok(Self {
            reader,
            writer,
            seq: 0,
        })
    }

    /// Send a DAP request
    fn send_request(&mut self, command: &str, arguments: serde_json::Value) -> std::io::Result<()> {
        self.seq += 1;
        let request = DapRequest {
            seq: self.seq,
            msg_type: "request".to_string(),
            command: command.to_string(),
            arguments,
        };

        let message = DapMessage::Request(request);
        DapProtocol::write_message(&mut self.writer, &message)
            .map_err(|e| std::io::Error::other(e.message))
    }

    /// Read a DAP response
    fn read_response(&mut self) -> std::io::Result<DapResponse> {
        match DapProtocol::read_message(&mut self.reader) {
            Ok(DapMessage::Response(resp)) => Ok(resp),
            Ok(other) => Err(std::io::Error::other(format!(
                "Expected response, got {:?}",
                other
            ))),
            Err(e) => Err(std::io::Error::other(e.message)),
        }
    }

    /// Read a DAP event
    fn read_event(&mut self) -> std::io::Result<DapEvent> {
        match DapProtocol::read_message(&mut self.reader) {
            Ok(DapMessage::Event(evt)) => Ok(evt),
            Ok(other) => Err(std::io::Error::other(format!(
                "Expected event, got {:?}",
                other
            ))),
            Err(e) => Err(std::io::Error::other(e.message)),
        }
    }

    /// Read next message (could be response or event)
    fn read_message(&mut self) -> std::io::Result<DapMessage> {
        DapProtocol::read_message(&mut self.reader).map_err(|e| std::io::Error::other(e.message))
    }

    /// Send initialize request
    fn initialize(&mut self) -> std::io::Result<DapResponse> {
        self.send_request(
            "initialize",
            json!({
                "clientID": "test",
                "clientName": "DAP Test Client",
                "adapterID": "tarqeem",
                "locale": "ar"
            }),
        )?;
        self.read_response()
    }

    /// Send launch request
    fn launch(&mut self, program: &str, stop_on_entry: bool) -> std::io::Result<DapResponse> {
        self.send_request(
            "launch",
            json!({
                "program": program,
                "stopOnEntry": stop_on_entry
            }),
        )?;
        self.read_response()
    }

    /// Send setBreakpoints request
    fn set_breakpoints(
        &mut self,
        source_path: &str,
        lines: &[i64],
    ) -> std::io::Result<DapResponse> {
        let breakpoints: Vec<_> = lines.iter().map(|&line| json!({ "line": line })).collect();

        self.send_request(
            "setBreakpoints",
            json!({
                "source": { "path": source_path },
                "breakpoints": breakpoints
            }),
        )?;
        self.read_response()
    }

    /// Send configurationDone request
    fn configuration_done(&mut self) -> std::io::Result<DapResponse> {
        self.send_request("configurationDone", json!({}))?;
        self.read_response()
    }

    /// Send continue request
    fn continue_execution(&mut self, thread_id: i64) -> std::io::Result<DapResponse> {
        self.send_request("continue", json!({ "threadId": thread_id }))?;
        self.read_response()
    }

    /// Send next (step over) request
    fn next(&mut self, thread_id: i64) -> std::io::Result<DapResponse> {
        self.send_request("next", json!({ "threadId": thread_id }))?;
        self.read_response()
    }

    /// Send stepIn request
    fn step_in(&mut self, thread_id: i64) -> std::io::Result<DapResponse> {
        self.send_request("stepIn", json!({ "threadId": thread_id }))?;
        self.read_response()
    }

    /// Send stepOut request
    fn step_out(&mut self, thread_id: i64) -> std::io::Result<DapResponse> {
        self.send_request("stepOut", json!({ "threadId": thread_id }))?;
        self.read_response()
    }

    /// Send threads request
    fn threads(&mut self) -> std::io::Result<DapResponse> {
        self.send_request("threads", json!({}))?;
        self.read_response()
    }

    /// Send stackTrace request
    fn stack_trace(&mut self, thread_id: i64) -> std::io::Result<DapResponse> {
        self.send_request("stackTrace", json!({ "threadId": thread_id }))?;
        self.read_response()
    }

    /// Send scopes request
    fn scopes(&mut self, frame_id: i64) -> std::io::Result<DapResponse> {
        self.send_request("scopes", json!({ "frameId": frame_id }))?;
        self.read_response()
    }

    /// Send variables request
    fn variables(&mut self, variables_reference: i64) -> std::io::Result<DapResponse> {
        self.send_request(
            "variables",
            json!({ "variablesReference": variables_reference }),
        )?;
        self.read_response()
    }

    /// Send disconnect request
    fn disconnect(&mut self) -> std::io::Result<DapResponse> {
        self.send_request("disconnect", json!({}))?;
        self.read_response()
    }
}

/// Start a DAP server in a background thread
fn start_dap_server(port: u16) -> (thread::JoinHandle<()>, Arc<AtomicBool>) {
    let shutdown = Arc::new(AtomicBool::new(false));
    let shutdown_clone = Arc::clone(&shutdown);

    let handle = thread::spawn(move || {
        let server = DapServer::new();
        // Note: We can't easily stop the server from outside in sync mode
        // The server will stop when the client disconnects
        let _ = server.run_tcp(port);
    });

    (handle, shutdown)
}

// ============================================================================
// Test: DAP Server Lifecycle
// ============================================================================

#[test]
#[ignore] // Requires TCP networking - run with: cargo test --ignored
fn test_dap_server_lifecycle() {
    let port = find_available_port();

    // Start server in background
    let (server_handle, _shutdown) = start_dap_server(port);

    // Give server time to start
    thread::sleep(Duration::from_millis(200));

    // Connect client
    let mut client = DapTestClient::connect(port).expect("Failed to connect to DAP server");

    // 1. Initialize
    let init_response = client.initialize().expect("Initialize failed");
    assert!(init_response.success, "Initialize should succeed");

    // Verify capabilities in response
    if let Some(body) = &init_response.body {
        assert!(
            body.get("supportsConfigurationDoneRequest")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            "Should support configurationDone"
        );
        assert!(
            body.get("supportsSetVariable")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            "Should support setVariable"
        );
    }

    // Read initialized event
    let event = client
        .read_event()
        .expect("Should receive initialized event");
    assert_eq!(event.event, "initialized");

    // 2. Threads request
    let threads_response = client.threads().expect("Threads failed");
    assert!(threads_response.success, "Threads should succeed");

    // 3. Disconnect
    let disconnect_response = client.disconnect().expect("Disconnect failed");
    assert!(disconnect_response.success, "Disconnect should succeed");

    // Wait for server to finish
    let _ = server_handle.join();
}

// ============================================================================
// Test: DAP Initialize Response Capabilities
// ============================================================================

#[test]
#[ignore] // Requires TCP networking - run with: cargo test --ignored
fn test_dap_initialize_capabilities() {
    let port = find_available_port();

    let (server_handle, _) = start_dap_server(port);
    thread::sleep(Duration::from_millis(200));

    let mut client = DapTestClient::connect(port).expect("Failed to connect");

    let init_response = client.initialize().expect("Initialize failed");
    assert!(init_response.success);

    // Check all expected capabilities
    if let Some(body) = &init_response.body {
        // Core capabilities
        assert!(body.get("supportsConfigurationDoneRequest").is_some());
        assert!(body.get("supportsSetVariable").is_some());
        assert!(body.get("supportsStepBack").is_some());
        assert!(body.get("supportsRestartRequest").is_some());
        assert!(body.get("supportsExceptionInfoRequest").is_some());
        assert!(body.get("supportsFunctionBreakpoints").is_some());
        assert!(body.get("supportsConditionalBreakpoints").is_some());
        assert!(body.get("supportsHitConditionalBreakpoints").is_some());
        assert!(body.get("supportsEvaluateForHovers").is_some());
    }

    // Cleanup
    let _ = client.disconnect();
    let _ = server_handle.join();
}

// ============================================================================
// Test: DAP Protocol Message Format
// ============================================================================

#[test]
fn test_dap_protocol_message_format() {
    use std::io::Cursor;

    // Test write and read of a request message
    let request = DapRequest {
        seq: 1,
        msg_type: "request".to_string(),
        command: "initialize".to_string(),
        arguments: json!({"clientID": "test"}),
    };

    let message = DapMessage::Request(request);

    // Write to buffer
    let mut buffer = Vec::new();
    DapProtocol::write_message(&mut buffer, &message).expect("Write failed");

    // Verify Content-Length header is present
    let output = String::from_utf8_lossy(&buffer);
    assert!(
        output.contains("Content-Length:"),
        "Should contain Content-Length header"
    );
    assert!(
        output.contains("\r\n\r\n"),
        "Should have header/body separator"
    );

    // Read back
    let mut cursor = Cursor::new(buffer);
    let read_message = DapProtocol::read_message(&mut cursor).expect("Read failed");

    if let DapMessage::Request(req) = read_message {
        assert_eq!(req.seq, 1);
        assert_eq!(req.command, "initialize");
    } else {
        panic!("Expected Request message");
    }
}

// ============================================================================
// Test: DAP Event Serialization
// ============================================================================

#[test]
fn test_dap_event_serialization() {
    // Test stopped event
    let stopped_event = DapEvent::stopped("breakpoint", 1, Some("Hit breakpoint".to_string()));
    let json = serde_json::to_string(&stopped_event).expect("Serialization failed");

    assert!(json.contains("\"event\":\"stopped\""));
    assert!(json.contains("\"reason\":\"breakpoint\""));
    assert!(json.contains("\"threadId\":1"));

    // Test initialized event
    let init_event = DapEvent::initialized();
    let json = serde_json::to_string(&init_event).expect("Serialization failed");
    assert!(json.contains("\"event\":\"initialized\""));

    // Test terminated event
    let term_event = DapEvent::terminated();
    let json = serde_json::to_string(&term_event).expect("Serialization failed");
    assert!(json.contains("\"event\":\"terminated\""));
}

// ============================================================================
// Test: DAP Request/Response Matching
// ============================================================================

#[test]
#[ignore] // Requires TCP networking - run with: cargo test --ignored
fn test_dap_request_response_matching() {
    let port = find_available_port();

    let (server_handle, _) = start_dap_server(port);
    thread::sleep(Duration::from_millis(200));

    let mut client = DapTestClient::connect(port).expect("Failed to connect");

    // Send multiple requests and verify response seq numbers match
    client
        .send_request("initialize", json!({"clientID": "test"}))
        .unwrap();
    let resp1 = client.read_response().unwrap();
    assert_eq!(resp1.request_seq, 1, "Response should match request seq 1");
    assert_eq!(resp1.command, "initialize");

    // Read initialized event
    let _ = client.read_event();

    client.send_request("threads", json!({})).unwrap();
    let resp2 = client.read_response().unwrap();
    assert_eq!(resp2.request_seq, 2, "Response should match request seq 2");
    assert_eq!(resp2.command, "threads");

    client.send_request("disconnect", json!({})).unwrap();
    let resp3 = client.read_response().unwrap();
    assert_eq!(resp3.request_seq, 3, "Response should match request seq 3");
    assert_eq!(resp3.command, "disconnect");

    let _ = server_handle.join();
}

// ============================================================================
// Test: DAP Error Handling
// ============================================================================

#[test]
#[ignore] // Requires TCP networking - run with: cargo test --ignored
fn test_dap_error_handling() {
    let port = find_available_port();

    let (server_handle, _) = start_dap_server(port);
    thread::sleep(Duration::from_millis(200));

    let mut client = DapTestClient::connect(port).expect("Failed to connect");

    // Initialize first
    let _ = client.initialize();
    let _ = client.read_event(); // initialized event

    // Send unknown command
    client
        .send_request("unknownCommand", json!({}))
        .expect("Send failed");
    let response = client.read_response().expect("Read failed");

    // Should get error response
    assert!(
        !response.success || response.message.is_some(),
        "Unknown command should fail or have error message"
    );

    // Cleanup
    let _ = client.disconnect();
    let _ = server_handle.join();
}

// ============================================================================
// Test: DAP Scopes and Variables Structure
// ============================================================================

#[test]
#[ignore] // Requires TCP networking - run with: cargo test --ignored
fn test_dap_scopes_variables_structure() {
    let port = find_available_port();

    let (server_handle, _) = start_dap_server(port);
    thread::sleep(Duration::from_millis(200));

    let mut client = DapTestClient::connect(port).expect("Failed to connect");

    // Initialize
    let _ = client.initialize();
    let _ = client.read_event();

    // Request scopes (even without a real program, should return valid structure)
    let scopes_response = client.scopes(0).expect("Scopes request failed");
    assert!(scopes_response.success, "Scopes should succeed");

    if let Some(body) = &scopes_response.body {
        // Should have scopes array
        assert!(body.get("scopes").is_some(), "Should have scopes array");
    }

    // Request variables
    let vars_response = client.variables(0).expect("Variables request failed");
    assert!(vars_response.success, "Variables should succeed");

    if let Some(body) = &vars_response.body {
        // Should have variables array
        assert!(
            body.get("variables").is_some(),
            "Should have variables array"
        );
    }

    // Cleanup
    let _ = client.disconnect();
    let _ = server_handle.join();
}

// ============================================================================
// Test: DAP Step Commands Response
// ============================================================================

#[test]
#[ignore] // Requires TCP networking - run with: cargo test --ignored
fn test_dap_step_commands_response() {
    let port = find_available_port();

    let (server_handle, _) = start_dap_server(port);
    thread::sleep(Duration::from_millis(200));

    let mut client = DapTestClient::connect(port).expect("Failed to connect");

    // Initialize
    let _ = client.initialize();
    let _ = client.read_event();

    // Test next command
    let next_response = client.next(1).expect("Next request failed");
    // Without a running program, this should return some response
    assert_eq!(next_response.command, "next");

    // Test stepIn command
    let step_in_response = client.step_in(1).expect("StepIn request failed");
    assert_eq!(step_in_response.command, "stepIn");

    // Test stepOut command
    let step_out_response = client.step_out(1).expect("StepOut request failed");
    assert_eq!(step_out_response.command, "stepOut");

    // Test continue command
    let continue_response = client
        .continue_execution(1)
        .expect("Continue request failed");
    assert_eq!(continue_response.command, "continue");

    // Cleanup
    let _ = client.disconnect();
    let _ = server_handle.join();
}

// ============================================================================
// Test: DAP Breakpoints Response Structure
// ============================================================================

#[test]
#[ignore] // Requires TCP networking - run with: cargo test --ignored
fn test_dap_breakpoints_response_structure() {
    let port = find_available_port();

    let (server_handle, _) = start_dap_server(port);
    thread::sleep(Duration::from_millis(200));

    let mut client = DapTestClient::connect(port).expect("Failed to connect");

    // Initialize
    let _ = client.initialize();
    let _ = client.read_event();

    // Set breakpoints
    let bp_response = client
        .set_breakpoints("/test/file.trq", &[5, 10, 15])
        .expect("SetBreakpoints failed");

    assert!(bp_response.success, "SetBreakpoints should succeed");

    if let Some(body) = &bp_response.body {
        // Should have breakpoints array
        let breakpoints = body.get("breakpoints").and_then(|b| b.as_array());
        assert!(breakpoints.is_some(), "Should have breakpoints array");

        if let Some(bps) = breakpoints {
            // Should have 3 breakpoints
            assert_eq!(bps.len(), 3, "Should have 3 breakpoints");

            // Each breakpoint should have id and verified fields
            for bp in bps {
                assert!(bp.get("id").is_some(), "Breakpoint should have id");
                assert!(
                    bp.get("verified").is_some(),
                    "Breakpoint should have verified"
                );
            }
        }
    }

    // Cleanup
    let _ = client.disconnect();
    let _ = server_handle.join();
}

// ============================================================================
// Test: DAP Exception Breakpoints
// ============================================================================

#[test]
#[ignore] // Requires TCP networking - run with: cargo test --ignored
fn test_dap_exception_breakpoints() {
    let port = find_available_port();

    let (server_handle, _) = start_dap_server(port);
    thread::sleep(Duration::from_millis(200));

    let mut client = DapTestClient::connect(port).expect("Failed to connect");

    // Initialize
    let _ = client.initialize();
    let _ = client.read_event();

    // Set exception breakpoints
    client
        .send_request(
            "setExceptionBreakpoints",
            json!({
                "filters": ["all", "uncaught"]
            }),
        )
        .expect("Send failed");

    let response = client.read_response().expect("Read failed");
    assert!(response.success, "SetExceptionBreakpoints should succeed");

    // Cleanup
    let _ = client.disconnect();
    let _ = server_handle.join();
}

// ============================================================================
// Test: DAP Stack Trace Response
// ============================================================================

#[test]
#[ignore] // Requires TCP networking - run with: cargo test --ignored
fn test_dap_stack_trace_response() {
    let port = find_available_port();

    let (server_handle, _) = start_dap_server(port);
    thread::sleep(Duration::from_millis(200));

    let mut client = DapTestClient::connect(port).expect("Failed to connect");

    // Initialize
    let _ = client.initialize();
    let _ = client.read_event();

    // Request stack trace
    let stack_response = client.stack_trace(1).expect("StackTrace request failed");
    assert!(stack_response.success, "StackTrace should succeed");

    if let Some(body) = &stack_response.body {
        // Should have stackFrames array
        assert!(body.get("stackFrames").is_some(), "Should have stackFrames");
        // Should have totalFrames
        assert!(body.get("totalFrames").is_some(), "Should have totalFrames");
    }

    // Cleanup
    let _ = client.disconnect();
    let _ = server_handle.join();
}

// ============================================================================
// Test: DAP Configuration Done
// ============================================================================

#[test]
#[ignore] // Requires TCP networking - run with: cargo test --ignored
fn test_dap_configuration_done() {
    let port = find_available_port();

    let (server_handle, _) = start_dap_server(port);
    thread::sleep(Duration::from_millis(200));

    let mut client = DapTestClient::connect(port).expect("Failed to connect");

    // Initialize
    let _ = client.initialize();
    let _ = client.read_event();

    // Configuration done
    let config_response = client
        .configuration_done()
        .expect("ConfigurationDone failed");
    assert!(config_response.success, "ConfigurationDone should succeed");

    // Cleanup
    let _ = client.disconnect();
    let _ = server_handle.join();
}

// ============================================================================
// Test: Multiple Sequential Requests
// ============================================================================

#[test]
#[ignore] // Requires TCP networking - run with: cargo test --ignored
fn test_dap_multiple_sequential_requests() {
    let port = find_available_port();

    let (server_handle, _) = start_dap_server(port);
    thread::sleep(Duration::from_millis(200));

    let mut client = DapTestClient::connect(port).expect("Failed to connect");

    // Perform a complete initialization sequence
    let init = client.initialize().expect("Initialize failed");
    assert!(init.success);

    let _ = client.read_event(); // initialized

    let threads = client.threads().expect("Threads failed");
    assert!(threads.success);

    let config = client
        .configuration_done()
        .expect("ConfigurationDone failed");
    assert!(config.success);

    // Set multiple breakpoints
    let bp1 = client
        .set_breakpoints("/test/a.trq", &[1, 2, 3])
        .expect("SetBreakpoints failed");
    assert!(bp1.success);

    let bp2 = client
        .set_breakpoints("/test/b.trq", &[10, 20])
        .expect("SetBreakpoints failed");
    assert!(bp2.success);

    // Get stack trace
    let stack = client.stack_trace(1).expect("StackTrace failed");
    assert!(stack.success);

    // Get scopes
    let scopes = client.scopes(0).expect("Scopes failed");
    assert!(scopes.success);

    // Disconnect
    let disconnect = client.disconnect().expect("Disconnect failed");
    assert!(disconnect.success);

    let _ = server_handle.join();
}
