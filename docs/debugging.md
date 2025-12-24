# Tarqeem Debugger (trqdbg)

دليل تصحيح البرامج في ترقيم

This document covers the debugging capabilities of the Tarqeem compiler, including CLI debugging and IDE integration via the Debug Adapter Protocol (DAP).

## Overview

The Tarqeem debugger provides:

- **Breakpoints**: Set breakpoints on source lines with optional conditions
- **Step Execution**: Step over, into, and out of function calls
- **Variable Inspection**: View local and global variables
- **Call Stack Tracing**: Navigate the call stack
- **Expression Evaluation**: Evaluate expressions at breakpoints
- **Watch Expressions**: Monitor variable values
- **DAP Support**: Full IDE integration via Debug Adapter Protocol

## CLI Debugging

### Starting the Debugger

```bash
# Start debugging a program
tarqeem debug program.trq

# With Arabic interface
tarqeem debug program.trq --arabic

# Stop at first line
tarqeem debug program.trq --stop-on-entry
```

### Interactive Commands

The debugger supports both English and Arabic commands:

| English | Arabic | Description |
|---------|--------|-------------|
| `break <line>` | `توقف <سطر>` | Set breakpoint at line |
| `break <line> if <cond>` | `توقف <سطر> إذا <شرط>` | Conditional breakpoint |
| `delete <id>` | `احذف <معرف>` | Remove breakpoint |
| `continue` | `تابع` | Continue execution |
| `step` | `خطوة` | Step into next line |
| `next` | `التالي` | Step over function calls |
| `out` | `خارج` | Step out of function |
| `print <expr>` | `اطبع <تعبير>` | Evaluate expression |
| `locals` | `محليات` | Show local variables |
| `globals` | `عالميات` | Show global variables |
| `stack` | `مكدس` | Show call stack |
| `watch <expr>` | `راقب <تعبير>` | Add watch expression |
| `list` | `عرض` | List breakpoints |
| `help` | `مساعدة` | Show help |
| `quit` | `اخرج` | Exit debugger |

### Example Session

```
$ tarqeem debug examples/حاسبة.trq
trqdbg> break 15
Breakpoint 1 set at line 15
نقطة توقف 1 عند السطر 15

trqdbg> continue
Hit breakpoint 1 at line 15
وصلت نقطة توقف 1 عند السطر 15

trqdbg> locals
  س: عدد = 10
  ص: عدد = 5

trqdbg> print س + ص
15

trqdbg> next
Stepped to line 16
انتقلت إلى السطر 16

trqdbg> continue
Program finished
انتهى البرنامج
```

## DAP Integration (IDE Debugging)

The Tarqeem debugger implements the [Debug Adapter Protocol](https://microsoft.github.io/debug-adapter-protocol/) for IDE integration.

### Starting the DAP Server

```bash
# TCP mode (for remote debugging)
tarqeem debug program.trq --dap-port 4711

# Stdio mode (for VS Code integration)
tarqeem debug program.trq --dap-stdio
```

### Supported DAP Features

| Feature | Status |
|---------|--------|
| Initialize/Terminate | ✓ |
| Launch | ✓ |
| Attach | Planned |
| SetBreakpoints | ✓ |
| SetExceptionBreakpoints | ✓ |
| SetFunctionBreakpoints | ✓ |
| ConfigurationDone | ✓ |
| Continue/Pause | ✓ |
| Next/StepIn/StepOut | ✓ |
| Threads | ✓ |
| StackTrace | ✓ |
| Scopes | ✓ |
| Variables | ✓ |
| SetVariable | ✓ |
| Evaluate | ✓ |
| Disconnect | ✓ |

### DAP Capabilities

The debugger advertises these capabilities:

```json
{
  "supportsConfigurationDoneRequest": true,
  "supportsFunctionBreakpoints": true,
  "supportsConditionalBreakpoints": true,
  "supportsHitConditionalBreakpoints": true,
  "supportsEvaluateForHovers": true,
  "supportsStepBack": false,
  "supportsSetVariable": true,
  "supportsRestartFrame": false,
  "supportsGotoTargetsRequest": false,
  "supportsStepInTargetsRequest": false,
  "supportsCompletionsRequest": false,
  "supportsModulesRequest": false,
  "supportsExceptionOptions": true,
  "supportsValueFormattingOptions": false,
  "supportsExceptionInfoRequest": true,
  "supportTerminateDebuggee": true,
  "supportsRestartRequest": true
}
```

## VS Code Integration

### Manual Testing

You can test the DAP server manually with netcat:

```bash
# Terminal 1: Start DAP server
tarqeem debug test.trq --dap-port 4711

# Terminal 2: Connect and send DAP messages
nc localhost 4711
```

Send an initialize request:

```
Content-Length: 119

{"seq":1,"type":"request","command":"initialize","arguments":{"clientID":"test","adapterID":"tarqeem"}}
```

### Launch Configuration

For VS Code integration, use this launch configuration template:

```json
{
  "version": "0.2.0",
  "configurations": [
    {
      "type": "tarqeem",
      "request": "launch",
      "name": "Debug Tarqeem / تصحيح ترقيم",
      "program": "${file}",
      "stopOnEntry": true
    }
  ]
}
```

## Breakpoint Types

### Line Breakpoints

```
trqdbg> break 25
Breakpoint 1 set at line 25
```

### Conditional Breakpoints

```
trqdbg> break 25 if س > 10
Breakpoint 1 set at line 25 (condition: س > 10)
```

### Hit Count Breakpoints

```
trqdbg> break 25 hit 5
Breakpoint 1 set at line 25 (hits after 5 times)
```

### Exception Breakpoints

Via DAP setExceptionBreakpoints:

- `"all"` - Break on all exceptions
- `"uncaught"` - Break only on uncaught exceptions

## Variable Inspection

### Viewing Variables

```
trqdbg> locals
  اسم: نص = "أحمد"
  عمر: عدد = 25
  راتب: عدد_عشري = 5000.50

trqdbg> globals
  __version__: نص = "1.0.0"
```

### Modifying Variables

Via DAP setVariable:

```json
{
  "command": "setVariable",
  "arguments": {
    "variablesReference": 1,
    "name": "عمر",
    "value": "30"
  }
}
```

### Evaluating Expressions

```
trqdbg> print عمر * 12
300

trqdbg> print اسم + " عمره " + عمر
أحمد عمره 25
```

## Call Stack Navigation

```
trqdbg> stack
#0  احسب_الضريبة (راتب=5000) at حاسبة.trq:25
#1  اطبع_التقرير () at تقرير.trq:15
#2  رئيسية () at برنامج.trq:5
```

## Troubleshooting

### Common Issues

**Debugger won't start**
- Ensure the program compiles without errors first
- Check file path is correct

**Breakpoints not hitting**
- Verify the line number has executable code
- Check that source mapping is available

**DAP connection fails**
- Ensure the port is not in use
- Check firewall settings for TCP mode

### Debug Logging

Enable verbose output:

```bash
RUST_LOG=debug tarqeem debug program.trq
```

### DAP Message Tracing

For DAP debugging, monitor the wire protocol:

```bash
# On Linux
tarqeem debug program.trq --dap-port 4711 2>&1 | tee debug.log
```

## Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                    IDE (VS Code)                              │
│                    عميل التصحيح                               │
└──────────────────────────────────────────────────────────────┘
                              │
                              │ DAP Protocol (JSON-RPC)
                              │ Content-Length + JSON
                              ▼
┌──────────────────────────────────────────────────────────────┐
│                    DapServer                                  │
│                    خادم التصحيح                               │
│  - TCP transport (--dap-port)                                │
│  - Stdio transport (--dap-stdio)                             │
└──────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌──────────────────────────────────────────────────────────────┐
│                    DapAdapter                                 │
│                    معالج الطلبات                              │
│  - Request handling                                          │
│  - Response generation                                       │
│  - Event emission                                            │
└──────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌──────────────────────────────────────────────────────────────┐
│                    DebugInterpreter                           │
│                    المفسر التصحيحي                            │
│  - Step-by-step execution                                    │
│  - Breakpoint checking                                       │
│  - Variable inspection                                       │
└──────────────────────────────────────────────────────────────┘
```

## API Reference

### DapServer

```rust
// Create a new DAP server
let server = DapServer::new();

// Run on TCP port
server.run_tcp(4711)?;

// Run on stdio
server.run_stdio()?;
```

### DebugInterpreter

```rust
// Create debug interpreter
let mut interpreter = DebugInterpreter::new(module, source_map);

// Set breakpoint
interpreter.context().set_breakpoint(file, line);

// Step execution
let result = interpreter.step();

// Get local variables
let locals = interpreter.get_locals();
```

### DebugContext

```rust
// Create context
let mut ctx = DebugContext::new();

// Breakpoint management
let id = ctx.add_breakpoint(file, line);
ctx.set_breakpoint_condition(id, "س > 10");
ctx.toggle_breakpoint(id);
ctx.remove_breakpoint(id);

// Watch expressions
ctx.add_watch("س + ص");
```

## Future Enhancements

- **Remote Debugging**: Debug over network connections
- **Multi-threaded Support**: When async/await is implemented
- **DWARF Debug Info**: Debug symbols in compiled binaries
- **Hot Code Reload**: Update code without restarting
- **Conditional Watchpoints**: Break on expression change
