//! Debug Adapter Protocol (DAP) implementation
//!
//! This module implements the Debug Adapter Protocol for VS Code integration.
//! DAP is a JSON-based protocol for communication between IDEs and debuggers.

use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::context::{Breakpoint, DebugContext};
use super::interpreter::{DebugInterpreter, StepResult};
use super::state::{DebugVariable, PauseReason, StackFrame, StepMode};

type Seq = i64;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DapRequest {
    pub seq: Seq,
    #[serde(rename = "type")]
    pub msg_type: String,
    pub command: String,
    #[serde(default)]
    pub arguments: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DapResponse {
    pub seq: Seq,
    #[serde(rename = "type")]
    pub msg_type: String,
    pub request_seq: Seq,
    pub success: bool,
    pub command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<serde_json::Value>,
}

impl DapResponse {
    pub fn success(request: &DapRequest, body: Option<serde_json::Value>) -> Self {
        Self {
            seq: 0, // Will be set by adapter
            msg_type: "response".to_string(),
            request_seq: request.seq,
            success: true,
            command: request.command.clone(),
            message: None,
            body,
        }
    }

    pub fn error(request: &DapRequest, message: String) -> Self {
        Self {
            seq: 0,
            msg_type: "response".to_string(),
            request_seq: request.seq,
            success: false,
            command: request.command.clone(),
            message: Some(message),
            body: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DapEvent {
    pub seq: Seq,
    #[serde(rename = "type")]
    pub msg_type: String,
    pub event: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<serde_json::Value>,
}

impl DapEvent {
    pub fn new(event: &str, body: Option<serde_json::Value>) -> Self {
        Self {
            seq: 0,
            msg_type: "event".to_string(),
            event: event.to_string(),
            body,
        }
    }

    pub fn initialized() -> Self {
        Self::new("initialized", None)
    }

    pub fn stopped(reason: &str, thread_id: i64, description: Option<String>) -> Self {
        let mut body = serde_json::json!({
            "reason": reason,
            "threadId": thread_id,
            "allThreadsStopped": true
        });

        if let Some(desc) = description {
            body["description"] = serde_json::Value::String(desc);
        }

        Self::new("stopped", Some(body))
    }

    pub fn continued(thread_id: i64) -> Self {
        Self::new(
            "continued",
            Some(serde_json::json!({
                "threadId": thread_id,
                "allThreadsContinued": true
            })),
        )
    }

    pub fn terminated() -> Self {
        Self::new("terminated", None)
    }

    pub fn exited(exit_code: i64) -> Self {
        Self::new(
            "exited",
            Some(serde_json::json!({
                "exitCode": exit_code
            })),
        )
    }

    pub fn output(category: &str, output: &str) -> Self {
        Self::new(
            "output",
            Some(serde_json::json!({
                "category": category,
                "output": output
            })),
        )
    }

    pub fn breakpoint(reason: &str, breakpoint: &DapBreakpoint) -> Self {
        Self::new(
            "breakpoint",
            Some(serde_json::json!({
                "reason": reason,
                "breakpoint": breakpoint
            })),
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DapBreakpoint {
    pub id: Option<i64>,
    pub verified: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<DapSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column: Option<i64>,
}

impl From<&Breakpoint> for DapBreakpoint {
    fn from(bp: &Breakpoint) -> Self {
        Self {
            id: Some(bp.id.0 as i64),
            verified: bp.verified,
            message: None,
            source: Some(DapSource {
                name: bp.file.file_name().map(|s| s.to_string_lossy().to_string()),
                path: Some(bp.file.to_string_lossy().to_string()),
            }),
            line: Some(bp.line as i64),
            column: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DapSource {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DapStackFrame {
    pub id: i64,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<DapSource>,
    pub line: i64,
    pub column: i64,
}

impl From<&StackFrame> for DapStackFrame {
    fn from(frame: &StackFrame) -> Self {
        let (source, line, column) = if let Some(loc) = &frame.location {
            (
                Some(DapSource {
                    name: loc
                        .file
                        .file_name()
                        .map(|s| s.to_string_lossy().to_string()),
                    path: Some(loc.file.to_string_lossy().to_string()),
                }),
                loc.line as i64,
                loc.column as i64,
            )
        } else {
            (None, 0, 0)
        };

        Self {
            id: frame.id as i64,
            name: frame.name.clone(),
            source,
            line,
            column,
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DapScope {
    pub name: String,
    #[serde(rename = "variablesReference")]
    pub variables_reference: i64,
    pub expensive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DapVariable {
    pub name: String,
    pub value: String,
    #[serde(rename = "type")]
    pub var_type: Option<String>,
    #[serde(rename = "variablesReference")]
    pub variables_reference: i64,
}

impl From<&DebugVariable> for DapVariable {
    fn from(var: &DebugVariable) -> Self {
        Self {
            name: var.name.clone(),
            value: var.value.clone(),
            var_type: Some(var.type_name.clone()),
            variables_reference: if var.children_count > 0 { 1 } else { 0 },
        }
    }
}

pub struct DapAdapter {
    interpreter: Option<DebugInterpreter>,
    seq: Seq,
    var_ref_counter: i64,
    var_refs: HashMap<i64, Vec<DebugVariable>>,
    initialized: bool,
    launched: bool,
    source_file: Option<PathBuf>,
    pending_events: Vec<DapEvent>,
    thread_id: i64,
}

impl Default for DapAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl DapAdapter {
    pub fn new() -> Self {
        Self {
            interpreter: None,
            seq: 1,
            var_ref_counter: 1,
            var_refs: HashMap::new(),
            initialized: false,
            launched: false,
            source_file: None,
            pending_events: Vec::new(),
            thread_id: 1,
        }
    }

    fn next_seq(&mut self) -> Seq {
        let seq = self.seq;
        self.seq += 1;
        seq
    }

    fn queue_event(&mut self, mut event: DapEvent) {
        event.seq = self.next_seq();
        self.pending_events.push(event);
    }

    pub fn take_events(&mut self) -> Vec<DapEvent> {
        std::mem::take(&mut self.pending_events)
    }

    pub fn handle_request(&mut self, request: DapRequest) -> DapResponse {
        let mut response = match request.command.as_str() {
            "initialize" => self.handle_initialize(&request),
            "launch" => self.handle_launch(&request),
            "attach" => self.handle_attach(&request),
            "disconnect" => self.handle_disconnect(&request),
            "terminate" => self.handle_terminate(&request),
            "configurationDone" => self.handle_configuration_done(&request),
            "setBreakpoints" => self.handle_set_breakpoints(&request),
            "setExceptionBreakpoints" => self.handle_set_exception_breakpoints(&request),
            "threads" => self.handle_threads(&request),
            "stackTrace" => self.handle_stack_trace(&request),
            "scopes" => self.handle_scopes(&request),
            "variables" => self.handle_variables(&request),
            "continue" => self.handle_continue(&request),
            "next" => self.handle_next(&request),
            "stepIn" => self.handle_step_in(&request),
            "stepOut" => self.handle_step_out(&request),
            "pause" => self.handle_pause(&request),
            "evaluate" => self.handle_evaluate(&request),
            "source" => self.handle_source(&request),
            "setVariable" => self.handle_set_variable(&request),
            // Memory Inspector extensions
            "tarqeem/memoryStats" => self.handle_memory_stats(&request),
            "tarqeem/heapAllocations" => self.handle_heap_allocations(&request),
            "tarqeem/memoryTimeline" => self.handle_memory_timeline(&request),
            _ => DapResponse::error(&request, format!("Unknown command: {}", request.command)),
        };

        response.seq = self.next_seq();
        response
    }

    fn handle_initialize(&mut self, request: &DapRequest) -> DapResponse {
        self.initialized = true;

        self.queue_event(DapEvent::initialized());

        DapResponse::success(
            request,
            Some(serde_json::json!({
                "supportsConfigurationDoneRequest": true,
                "supportsEvaluateForHovers": true,
                "supportsConditionalBreakpoints": true,
                "supportsHitConditionalBreakpoints": true,
                "supportsLogPoints": true,
                "supportsStepBack": false,
                "supportsRestartFrame": false,
                "supportsGotoTargetsRequest": false,
                "supportTerminateDebuggee": true,
                "supportsSetVariable": true,
                "supportsSetExpression": false,
                "supportsDelayedStackTraceLoading": false,
                "supportsLoadedSourcesRequest": false,
                "supportsExceptionInfoRequest": true,
                "supportsFunctionBreakpoints": false,
                "supportsDataBreakpoints": false,
                "supportsDisassembleRequest": false,
                "supportsReadMemoryRequest": false,
                "supportsWriteMemoryRequest": false,
            })),
        )
    }

    fn handle_launch(&mut self, request: &DapRequest) -> DapResponse {
        let program = request.arguments.get("program").and_then(|v| v.as_str());

        if program.is_none() {
            return DapResponse::error(request, "Program path is required".to_string());
        }

        let program_path = PathBuf::from(program.unwrap());
        self.source_file = Some(program_path.clone());

        let source = match std::fs::read_to_string(&program_path) {
            Ok(s) => s,
            Err(e) => return DapResponse::error(request, format!("Failed to read file: {}", e)),
        };

        let mut parser = crate::parser::Parser::new(&source);
        let ast = match parser.parse() {
            Ok(ast) => ast,
            Err(e) => return DapResponse::error(request, format!("خطأ في التحليل: {}", e.message)),
        };

        let mut analyzer = crate::semantic::Analyzer::new();
        if let Err(diagnostics) = analyzer.analyze(&ast) {
            let msg = diagnostics
                .iter()
                .map(|d| format!("{}: {}", d.span, d.message))
                .collect::<Vec<_>>()
                .join("\n");
            return DapResponse::error(request, format!("Semantic errors:\n{}", msg));
        }

        let ir_builder = crate::ir::IrBuilder::new("debug".to_string());
        let ir_module = match ir_builder.build(&ast) {
            Ok(m) => m,
            Err(e) => return DapResponse::error(request, format!("IR error: {}", e.message)),
        };

        let mut context = DebugContext::new();

        let stop_on_entry = request
            .arguments
            .get("stopOnEntry")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        context.config_mut().stop_on_entry = stop_on_entry;

        context
            .source_map_mut()
            .add_source(program_path.clone(), source);

        let mut interpreter = DebugInterpreter::new(ir_module, context);
        interpreter.set_source_file(program_path);

        if let Err(e) = interpreter.start() {
            return DapResponse::error(request, format!("Failed to start: {}", e.message));
        }

        self.interpreter = Some(interpreter);
        self.launched = true;

        if stop_on_entry {
            self.queue_event(DapEvent::stopped(
                "entry",
                self.thread_id,
                Some("Stopped at entry point".to_string()),
            ));
        }

        DapResponse::success(request, None)
    }

    fn handle_attach(&mut self, request: &DapRequest) -> DapResponse {
        DapResponse::error(request, "Attach not supported".to_string())
    }

    fn handle_disconnect(&mut self, request: &DapRequest) -> DapResponse {
        self.interpreter = None;
        self.launched = false;
        DapResponse::success(request, None)
    }

    fn handle_terminate(&mut self, request: &DapRequest) -> DapResponse {
        self.interpreter = None;
        self.launched = false;
        self.queue_event(DapEvent::terminated());
        DapResponse::success(request, None)
    }

    fn handle_configuration_done(&mut self, request: &DapRequest) -> DapResponse {
        if let Some(ref interpreter) = self.interpreter {
            if !interpreter.context().config().stop_on_entry {}
        }
        DapResponse::success(request, None)
    }

    fn handle_set_breakpoints(&mut self, request: &DapRequest) -> DapResponse {
        let source = request
            .arguments
            .get("source")
            .and_then(|s| s.get("path"))
            .and_then(|p| p.as_str());

        if source.is_none() {
            return DapResponse::error(request, "Source path required".to_string());
        }

        let source_path = PathBuf::from(source.unwrap());

        let breakpoints = request
            .arguments
            .get("breakpoints")
            .and_then(|b| b.as_array())
            .cloned()
            .unwrap_or_default();

        let mut result_breakpoints = Vec::new();

        if let Some(ref mut interpreter) = self.interpreter {
            let existing: Vec<_> = interpreter
                .context()
                .breakpoints()
                .filter(|bp| bp.file == source_path)
                .map(|bp| bp.id)
                .collect();

            for id in existing {
                let _ = interpreter.context_mut().remove_breakpoint(id);
            }

            for bp_json in breakpoints {
                let line = bp_json.get("line").and_then(|l| l.as_i64()).unwrap_or(0) as usize;

                let condition = bp_json
                    .get("condition")
                    .and_then(|c| c.as_str())
                    .map(|s| s.to_string());

                let result = if let Some(cond) = condition {
                    interpreter.context_mut().add_conditional_breakpoint(
                        source_path.clone(),
                        line,
                        cond,
                    )
                } else {
                    interpreter
                        .context_mut()
                        .add_breakpoint(source_path.clone(), line)
                };

                match result {
                    Ok(id) => {
                        let bp = interpreter.context().get_breakpoint(id).unwrap();
                        result_breakpoints.push(DapBreakpoint::from(bp));
                    }
                    Err(_) => {
                        result_breakpoints.push(DapBreakpoint {
                            id: None,
                            verified: false,
                            message: Some("Invalid breakpoint location".to_string()),
                            source: None,
                            line: Some(line as i64),
                            column: None,
                        });
                    }
                }
            }
        }

        DapResponse::success(
            request,
            Some(serde_json::json!({
                "breakpoints": result_breakpoints
            })),
        )
    }

    fn handle_set_exception_breakpoints(&mut self, request: &DapRequest) -> DapResponse {
        let filters = request
            .arguments
            .get("filters")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        let mut break_all = false;
        let mut break_uncaught = false;

        for filter in &filters {
            if let Some(filter_str) = filter.as_str() {
                match filter_str {
                    "all" | "raised" => break_all = true,
                    "uncaught" => break_uncaught = true,
                    _ => {}
                }
            }
        }

        if let Some(ref mut interpreter) = self.interpreter {
            interpreter
                .context_mut()
                .set_exception_breakpoints(break_all, break_uncaught);
        }

        DapResponse::success(request, Some(serde_json::json!({ "breakpoints": [] })))
    }

    fn handle_threads(&mut self, request: &DapRequest) -> DapResponse {
        DapResponse::success(
            request,
            Some(serde_json::json!({
                "threads": [{
                    "id": self.thread_id,
                    "name": "الخيط الرئيسي"
                }]
            })),
        )
    }

    fn handle_stack_trace(&mut self, request: &DapRequest) -> DapResponse {
        let frames: Vec<DapStackFrame> = if let Some(ref interpreter) = self.interpreter {
            interpreter
                .get_stack_trace()
                .iter()
                .map(DapStackFrame::from)
                .collect()
        } else {
            Vec::new()
        };

        DapResponse::success(
            request,
            Some(serde_json::json!({
                "stackFrames": frames,
                "totalFrames": frames.len()
            })),
        )
    }

    fn handle_scopes(&mut self, request: &DapRequest) -> DapResponse {
        let locals_ref = self.var_ref_counter;
        self.var_ref_counter += 1;
        let globals_ref = self.var_ref_counter;
        self.var_ref_counter += 1;

        if let Some(ref interpreter) = self.interpreter {
            self.var_refs.insert(locals_ref, interpreter.get_locals());
            self.var_refs.insert(globals_ref, interpreter.get_globals());
        }

        DapResponse::success(
            request,
            Some(serde_json::json!({
                "scopes": [
                    {
                        "name": "محليات",
                        "variablesReference": locals_ref,
                        "expensive": false
                    },
                    {
                        "name": "عالميات",
                        "variablesReference": globals_ref,
                        "expensive": false
                    }
                ]
            })),
        )
    }

    fn handle_variables(&mut self, request: &DapRequest) -> DapResponse {
        let var_ref = request
            .arguments
            .get("variablesReference")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        let variables: Vec<DapVariable> = self
            .var_refs
            .get(&var_ref)
            .map(|vars| vars.iter().map(DapVariable::from).collect())
            .unwrap_or_default();

        DapResponse::success(
            request,
            Some(serde_json::json!({
                "variables": variables
            })),
        )
    }

    fn handle_continue(&mut self, request: &DapRequest) -> DapResponse {
        if let Some(ref mut interpreter) = self.interpreter {
            match interpreter.continue_execution() {
                Ok(result) => {
                    self.handle_step_result(result);
                }
                Err(e) => {
                    self.queue_event(DapEvent::output(
                        "stderr",
                        &format!("Error: {}\n", e.message),
                    ));
                    self.queue_event(DapEvent::terminated());
                }
            }
        }

        DapResponse::success(
            request,
            Some(serde_json::json!({
                "allThreadsContinued": true
            })),
        )
    }

    fn handle_next(&mut self, request: &DapRequest) -> DapResponse {
        self.do_step(request, StepMode::Over)
    }

    fn handle_step_in(&mut self, request: &DapRequest) -> DapResponse {
        self.do_step(request, StepMode::Into)
    }

    fn handle_step_out(&mut self, request: &DapRequest) -> DapResponse {
        self.do_step(request, StepMode::Out)
    }

    fn do_step(&mut self, request: &DapRequest, mode: StepMode) -> DapResponse {
        if let Some(ref mut interpreter) = self.interpreter {
            match interpreter.step(mode) {
                Ok(result) => {
                    self.handle_step_result(result);
                }
                Err(e) => {
                    self.queue_event(DapEvent::output(
                        "stderr",
                        &format!("Error: {}\n", e.message),
                    ));
                    self.queue_event(DapEvent::terminated());
                }
            }
        }
        DapResponse::success(request, None)
    }

    fn handle_step_result(&mut self, result: StepResult) {
        match result {
            StepResult::Continue => {}
            StepResult::Paused(reason) => {
                let (dap_reason, description) = match reason {
                    PauseReason::Breakpoint { id } => {
                        ("breakpoint", Some(format!("Breakpoint {} hit", id.0)))
                    }
                    PauseReason::Step => ("step", None),
                    PauseReason::Entry => ("entry", Some("Stopped at entry".to_string())),
                    PauseReason::Exception { message } => {
                        ("exception", Some(format!("Exception: {}", message)))
                    }
                    PauseReason::UserRequest => ("pause", Some("Paused by user".to_string())),
                    PauseReason::DataBreakpoint { variable, .. } => (
                        "data breakpoint",
                        Some(format!("Variable '{}' changed", variable)),
                    ),
                };
                self.queue_event(DapEvent::stopped(dap_reason, self.thread_id, description));
            }
            StepResult::Returned(_) | StepResult::Completed(_) => {
                self.queue_event(DapEvent::exited(0));
                self.queue_event(DapEvent::terminated());
            }
            StepResult::Exception(exc) => {
                let msg = exc.to_display_string();
                self.queue_event(DapEvent::output("stderr", &format!("Exception: {}\n", msg)));
                self.queue_event(DapEvent::stopped(
                    "exception",
                    self.thread_id,
                    Some(format!("Exception: {}", msg)),
                ));
            }
        }
    }

    fn handle_pause(&mut self, request: &DapRequest) -> DapResponse {
        if let Some(ref mut interpreter) = self.interpreter {
            interpreter.request_pause();
            DapResponse::success(request, None)
        } else {
            DapResponse::error(request, "لا توجد جلسة تصحيح نشطة".to_string())
        }
    }

    fn handle_set_variable(&mut self, request: &DapRequest) -> DapResponse {
        let name = request
            .arguments
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let value = request
            .arguments
            .get("value")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if name.is_empty() {
            return DapResponse::error(request, "اسم المتغير مطلوب".to_string());
        }

        if let Some(ref mut interpreter) = self.interpreter {
            match interpreter.set_variable(name, value) {
                Ok(new_value) => DapResponse::success(
                    request,
                    Some(serde_json::json!({
                        "value": new_value.to_display_string(),
                        "type": new_value.type_name(),
                        "variablesReference": 0
                    })),
                ),
                Err(e) => DapResponse::error(request, e.message.clone()),
            }
        } else {
            DapResponse::error(request, "لا توجد جلسة تصحيح نشطة".to_string())
        }
    }

    fn handle_evaluate(&mut self, request: &DapRequest) -> DapResponse {
        let expression = request
            .arguments
            .get("expression")
            .and_then(|e| e.as_str())
            .unwrap_or("");

        if let Some(ref interpreter) = self.interpreter {
            match interpreter.evaluate(expression) {
                Ok(value) => DapResponse::success(
                    request,
                    Some(serde_json::json!({
                        "result": value.to_display_string(),
                        "type": value.type_name(),
                        "variablesReference": 0
                    })),
                ),
                Err(e) => DapResponse::error(request, e.message),
            }
        } else {
            DapResponse::error(request, "No active debug session".to_string())
        }
    }

    fn handle_source(&mut self, request: &DapRequest) -> DapResponse {
        if let Some(ref _interpreter) = self.interpreter {
            if let Some(ref source_file) = self.source_file {
                if let Ok(content) = std::fs::read_to_string(source_file) {
                    return DapResponse::success(
                        request,
                        Some(serde_json::json!({
                            "content": content
                        })),
                    );
                }
            }
        }
        DapResponse::error(request, "Source not available".to_string())
    }

    // ========================================================================
    // Memory Inspector DAP Extensions
    // ========================================================================

    /// Handle tarqeem/memoryStats request - returns memory usage statistics
    fn handle_memory_stats(&mut self, request: &DapRequest) -> DapResponse {
        if let Some(ref interpreter) = self.interpreter {
            // Collect heap allocations to compute statistics
            let allocations = interpreter.get_heap_allocations();

            let mut total_size: usize = 0;
            let mut string_size: usize = 0;
            let mut array_size: usize = 0;
            let mut object_size: usize = 0;

            for alloc in &allocations {
                total_size += alloc.size;
                match alloc.type_name.as_str() {
                    "string" | "نص" => string_size += alloc.size,
                    "array" | "مصفوفة" => array_size += alloc.size,
                    "object" | "كائن" => object_size += alloc.size,
                    _ => {}
                }
            }

            let regions = vec![
                serde_json::json!({
                    "name": "Strings",
                    "name_ar": "نصوص",
                    "allocated": string_size,
                    "peak": string_size,
                    "allocation_count": allocations.iter().filter(|a| a.type_name == "string" || a.type_name == "نص").count(),
                    "deallocation_count": 0
                }),
                serde_json::json!({
                    "name": "Arrays",
                    "name_ar": "مصفوفات",
                    "allocated": array_size,
                    "peak": array_size,
                    "allocation_count": allocations.iter().filter(|a| a.type_name == "array" || a.type_name == "مصفوفة").count(),
                    "deallocation_count": 0
                }),
                serde_json::json!({
                    "name": "Objects",
                    "name_ar": "كائنات",
                    "allocated": object_size,
                    "peak": object_size,
                    "allocation_count": allocations.iter().filter(|a| a.type_name == "object" || a.type_name == "كائن").count(),
                    "deallocation_count": 0
                }),
            ];

            DapResponse::success(
                request,
                Some(serde_json::json!({
                    "regions": regions,
                    "total_allocated": total_size,
                    "total_peak": total_size,
                    "live_allocations": allocations.len()
                })),
            )
        } else {
            DapResponse::error(request, "لا توجد جلسة تصحيح نشطة".to_string())
        }
    }

    /// Handle tarqeem/heapAllocations request - returns list of heap objects with ref counts
    fn handle_heap_allocations(&mut self, request: &DapRequest) -> DapResponse {
        let limit = request
            .arguments
            .get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(100) as usize;

        if let Some(ref interpreter) = self.interpreter {
            let allocations = interpreter.get_heap_allocations();

            let alloc_json: Vec<serde_json::Value> = allocations
                .into_iter()
                .take(limit)
                .map(|a| {
                    serde_json::json!({
                        "id": a.id,
                        "type_name": a.type_name,
                        "type_name_ar": a.type_name_ar,
                        "size": a.size,
                        "address": a.address,
                        "ref_count": a.ref_count,
                        "tag": a.tag,
                        "children": a.children
                    })
                })
                .collect();

            DapResponse::success(
                request,
                Some(serde_json::json!({
                    "allocations": alloc_json
                })),
            )
        } else {
            DapResponse::error(request, "لا توجد جلسة تصحيح نشطة".to_string())
        }
    }

    /// Handle tarqeem/memoryTimeline request - returns allocation events over time
    fn handle_memory_timeline(&mut self, request: &DapRequest) -> DapResponse {
        // Memory timeline tracking requires instrumentation during execution
        // For now, return an empty timeline - this can be enhanced later
        // with allocation/deallocation event tracking
        if self.interpreter.is_some() {
            DapResponse::success(
                request,
                Some(serde_json::json!({
                    "events": [],
                    "message": "تتبع الخط الزمني للذاكرة لم يُنفَّذ بعد"
                })),
            )
        } else {
            DapResponse::error(request, "لا توجد جلسة تصحيح نشطة".to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dap_event_creation() {
        let event = DapEvent::initialized();
        assert_eq!(event.event, "initialized");

        let event = DapEvent::stopped("breakpoint", 1, Some("Test".to_string()));
        assert_eq!(event.event, "stopped");
        assert!(event.body.is_some());
    }

    #[test]
    fn test_dap_response() {
        let request = DapRequest {
            seq: 1,
            msg_type: "request".to_string(),
            command: "test".to_string(),
            arguments: serde_json::Value::Null,
        };

        let response = DapResponse::success(&request, None);
        assert!(response.success);
        assert_eq!(response.request_seq, 1);

        let response = DapResponse::error(&request, "Error".to_string());
        assert!(!response.success);
        assert_eq!(response.message, Some("Error".to_string()));
    }

    #[test]
    fn test_dap_adapter_initialize() {
        let mut adapter = DapAdapter::new();

        let request = DapRequest {
            seq: 1,
            msg_type: "request".to_string(),
            command: "initialize".to_string(),
            arguments: serde_json::json!({}),
        };

        let response = adapter.handle_request(request);
        assert!(response.success);
        assert!(adapter.initialized);

        let events = adapter.take_events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event, "initialized");
    }

    #[test]
    fn test_dap_adapter_threads() {
        let mut adapter = DapAdapter::new();

        let request = DapRequest {
            seq: 1,
            msg_type: "request".to_string(),
            command: "threads".to_string(),
            arguments: serde_json::json!({}),
        };

        let response = adapter.handle_request(request);
        assert!(response.success);

        let body = response.body.unwrap();
        let threads = body.get("threads").unwrap().as_array().unwrap();
        assert_eq!(threads.len(), 1);
    }
}
