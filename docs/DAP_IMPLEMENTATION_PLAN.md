# DAP (Debug Adapter Protocol) Full Implementation Plan

## Current State Analysis

### What Exists (95% Complete)

The Tarqeem debugger (`src/debug/`) is **substantially implemented**:

| Component | Status | Lines | Description |
|-----------|--------|-------|-------------|
| `adapter.rs` | 95% | 895 | DAP protocol types & request handlers |
| `commands.rs` | 100% | 498 | CLI command parser (Arabic/English) |
| `context.rs` | 100% | 630 | Breakpoints, watches, debug config |
| `interpreter.rs` | 100% | 1,612 | Debug-aware interpreter |
| `source_map.rs` | 100% | 416 | IR-to-source mapping |
| `state.rs` | 100% | 398 | Debug state machine |
| `tests.rs` | 100% | 505 | 57 unit tests |

### What's Missing

1. **DAP Server Mode** - TCP/stdio transport layer (TODO at `src/cli/commands.rs:512`)
2. **VS Code Extension** - `.vsix` package for IDE integration
3. **Launch Configuration** - `launch.json` schema for VS Code
4. **DWARF Debug Info** - Debug symbols in compiled binaries (future)

---

## Implementation Plan

### Phase 1: DAP Server Transport Layer

**Goal**: Implement TCP and stdio transports for DAP communication.

#### 1.1 Create DAP Server Module

**File**: `src/debug/server.rs` (~400 lines)

```rust
// Core components needed:
pub struct DapServer {
    adapter: DapAdapter,
    transport: Box<dyn DapTransport>,
}

pub trait DapTransport: Send {
    async fn read_message(&mut self) -> Result<DapMessage, TransportError>;
    async fn write_message(&mut self, message: &DapMessage) -> Result<(), TransportError>;
}

pub struct TcpTransport { /* TCP stream wrapper */ }
pub struct StdioTransport { /* stdin/stdout wrapper */ }
```

**Key Features**:
- Content-Length header parsing (DAP wire protocol)
- JSON message serialization/deserialization
- Async I/O with tokio
- Graceful connection handling

#### 1.2 DAP Wire Protocol

The DAP protocol uses HTTP-style headers:

```
Content-Length: 119\r\n
\r\n
{"seq":1,"type":"request","command":"initialize","arguments":{"clientID":"vscode"}}
```

**Implementation**:
```rust
async fn read_message(reader: &mut BufReader<R>) -> Result<DapMessage> {
    // 1. Read headers until blank line
    // 2. Parse Content-Length
    // 3. Read exactly that many bytes
    // 4. Deserialize JSON
}

async fn write_message(writer: &mut BufWriter<W>, msg: &DapMessage) -> Result<()> {
    let json = serde_json::to_string(msg)?;
    write!(writer, "Content-Length: {}\r\n\r\n{}", json.len(), json)?;
    writer.flush()?;
}
```

#### 1.3 Server Main Loop

```rust
impl DapServer {
    pub async fn run(&mut self) -> Result<()> {
        loop {
            // 1. Read request from transport
            let message = self.transport.read_message().await?;

            // 2. Handle request
            match message {
                DapMessage::Request(req) => {
                    let response = self.adapter.handle_request(req);
                    self.transport.write_message(&response.into()).await?;

                    // Send pending events
                    for event in self.adapter.take_events() {
                        self.transport.write_message(&event.into()).await?;
                    }
                }
                DapMessage::Event(_) => { /* Client shouldn't send events */ }
                DapMessage::Response(_) => { /* Ignore */ }
            }

            // 3. Check for disconnect
            if self.adapter.is_disconnected() {
                break;
            }
        }
        Ok(())
    }
}
```

#### 1.4 CLI Integration

**Update**: `src/cli/commands.rs` (line 511-519)

```rust
if let Some(port) = dap_port {
    // TCP mode
    let server = DapServer::new_tcp(port).await?;
    println!("DAP server listening on port {} / المصحح يستمع على المنفذ {}", port, port);
    server.run().await?;
} else if dap_stdio {
    // Stdio mode (for VS Code)
    let server = DapServer::new_stdio();
    server.run().await?;
} else {
    // Interactive CLI mode (existing)
}
```

---

### Phase 2: Enhanced DAP Features

#### 2.1 Running Execution (Background)

Currently, the debugger blocks on `continue`. For proper DAP:

```rust
// In adapter.rs, handle_continue should:
// 1. Start execution in background task
// 2. Return immediately
// 3. Send "continued" event
// 4. When stopped, send "stopped" event

pub async fn handle_continue_async(&mut self, request: &DapRequest) -> DapResponse {
    self.queue_event(DapEvent::continued(self.thread_id));

    // Spawn execution task
    let interpreter = self.interpreter.take();
    tokio::spawn(async move {
        // Run until breakpoint/completion
        // Send events via channel
    });

    DapResponse::success(request, Some(json!({"allThreadsContinued": true})))
}
```

#### 2.2 Pause Support

Implement async pause for running programs:

```rust
fn handle_pause(&mut self, request: &DapRequest) -> DapResponse {
    if let Some(ref interpreter) = self.interpreter {
        interpreter.request_pause();  // Set atomic flag
        DapResponse::success(request, None)
    } else {
        DapResponse::error(request, "No debug session")
    }
}
```

#### 2.3 SetVariable Support

Allow modifying variables during debug:

```rust
fn handle_set_variable(&mut self, request: &DapRequest) -> DapResponse {
    let name = request.arguments["name"].as_str()?;
    let value = request.arguments["value"].as_str()?;

    if let Some(ref mut interpreter) = self.interpreter {
        interpreter.set_variable(name, value)?;
        // Return new value
    }
}
```

#### 2.4 Exception Breakpoints

```rust
fn handle_set_exception_breakpoints(&mut self, request: &DapRequest) -> DapResponse {
    let filters = request.arguments["filters"].as_array()?;

    for filter in filters {
        match filter.as_str() {
            Some("all") => self.break_on_all_exceptions = true,
            Some("uncaught") => self.break_on_uncaught = true,
            _ => {}
        }
    }

    DapResponse::success(request, Some(json!({"breakpoints": []})))
}
```

---

### Phase 3: VS Code Extension

#### 3.1 Extension Structure

```
vscode-tarqeem-debug/
├── package.json           # Extension manifest
├── src/
│   └── extension.ts       # Extension entry
├── syntaxes/
│   └── tarqeem.tmLanguage.json  # Syntax highlighting
└── images/
    └── icon.png
```

#### 3.2 package.json

```json
{
  "name": "tarqeem-debug",
  "displayName": "Tarqeem Debugger",
  "description": "Debug support for Tarqeem (ترقيم) programming language",
  "version": "0.1.0",
  "publisher": "tarqeem",
  "engines": { "vscode": "^1.75.0" },
  "categories": ["Debuggers"],
  "activationEvents": ["onDebug"],
  "main": "./out/extension.js",
  "contributes": {
    "debuggers": [{
      "type": "tarqeem",
      "label": "Tarqeem / ترقيم",
      "program": "./out/debugAdapter.js",
      "runtime": "node",
      "languages": ["tarqeem"],
      "configurationAttributes": {
        "launch": {
          "required": ["program"],
          "properties": {
            "program": {
              "type": "string",
              "description": "Path to Tarqeem file / مسار ملف ترقيم"
            },
            "stopOnEntry": {
              "type": "boolean",
              "description": "Stop at first line / توقف عند أول سطر",
              "default": false
            },
            "args": {
              "type": "array",
              "description": "Command line arguments / معاملات سطر الأوامر"
            }
          }
        }
      },
      "initialConfigurations": [{
        "type": "tarqeem",
        "request": "launch",
        "name": "Debug Tarqeem / تصحيح ترقيم",
        "program": "${file}",
        "stopOnEntry": true
      }]
    }],
    "languages": [{
      "id": "tarqeem",
      "aliases": ["Tarqeem", "ترقيم"],
      "extensions": [".trq", ".ترقيم"],
      "configuration": "./language-configuration.json"
    }]
  }
}
```

#### 3.3 Debug Adapter (TypeScript)

```typescript
// debugAdapter.ts - Spawns tarqeem with DAP mode
import { spawn } from 'child_process';

const tarqeem = spawn('tarqeem', ['debug', '--dap-stdio', program]);

// Pipe DAP messages between VS Code and tarqeem
process.stdin.pipe(tarqeem.stdin);
tarqeem.stdout.pipe(process.stdout);
```

---

### Phase 4: Testing & Documentation

#### 4.1 Integration Tests

**File**: `tests/dap_integration_tests.rs`

```rust
#[tokio::test]
async fn test_dap_server_lifecycle() {
    // 1. Start DAP server
    // 2. Send initialize
    // 3. Send launch
    // 4. Set breakpoints
    // 5. Continue
    // 6. Verify stopped event
    // 7. Disconnect
}

#[tokio::test]
async fn test_dap_breakpoint_hit() {
    // Test breakpoint triggers stop event
}

#[tokio::test]
async fn test_dap_step_operations() {
    // Test next, stepIn, stepOut
}
```

#### 4.2 Manual Test Protocol

1. Start server: `tarqeem debug test.trq --dap-port 4711`
2. Connect with netcat: `nc localhost 4711`
3. Send initialize request
4. Verify capabilities response
5. Test each DAP command

#### 4.3 Documentation

- Update `README.md` with debugger usage
- Create `docs/debugging.md` with:
  - CLI debugging tutorial
  - VS Code setup guide
  - DAP protocol reference
  - Troubleshooting guide

---

## Implementation Order

| Step | Task | Effort | Files |
|------|------|--------|-------|
| 1 | Create `server.rs` with transport trait | 2h | `src/debug/server.rs` |
| 2 | Implement `TcpTransport` | 1h | `src/debug/server.rs` |
| 3 | Implement `StdioTransport` | 30m | `src/debug/server.rs` |
| 4 | Wire protocol (Content-Length) | 1h | `src/debug/server.rs` |
| 5 | Server main loop | 1h | `src/debug/server.rs` |
| 6 | Update CLI for DAP mode | 30m | `src/cli/commands.rs` |
| 7 | Add async execution support | 2h | `src/debug/adapter.rs` |
| 8 | Add pause support | 1h | `src/debug/interpreter.rs` |
| 9 | Integration tests | 2h | `tests/dap_tests.rs` |
| 10 | VS Code extension scaffold | 2h | `vscode-tarqeem-debug/` |
| 11 | Documentation | 1h | `docs/debugging.md` |

**Total Estimated Effort**: ~14 hours

---

## Dependencies

Already available in `Cargo.toml`:
- `tokio` (async runtime) ✓
- `serde_json` (JSON serialization) ✓
- `serde` (derive macros) ✓

May need to add:
- None - all dependencies are present

---

## Risk Assessment

| Risk | Mitigation |
|------|------------|
| Async execution complexity | Start with synchronous blocking, add async later |
| VS Code extension packaging | Use yeoman generator, follow official docs |
| Wire protocol parsing | Extensive unit tests for edge cases |
| Arabic text in DAP messages | Ensure UTF-8 handling throughout |

---

## Success Criteria

1. `tarqeem debug file.trq --dap-port 4711` starts TCP server
2. `tarqeem debug file.trq --dap-stdio` works with VS Code
3. VS Code can:
   - Set breakpoints
   - Step through code
   - Inspect variables
   - See call stack
   - Evaluate expressions
4. All existing CLI debugger tests pass
5. New DAP server integration tests pass

---

## Future Enhancements (Out of Scope)

1. **DWARF Debug Info**: Generate debug symbols for compiled binaries
2. **Attach Mode**: Attach to running Tarqeem processes
3. **Multi-threaded Debugging**: When async/await is implemented
4. **Remote Debugging**: Debug over network connections
5. **Conditional Watchpoints**: Break on expression change
6. **Hot Code Reload**: Update code without restarting
