//! Debug command parsing and execution
//!
//! This module provides a command parser for the interactive debugger CLI.

use std::path::PathBuf;

use super::context::BreakpointId;
use super::state::StepMode;

#[derive(Debug, Clone, PartialEq)]
pub enum DebugCommand {
    Continue,

    StepOver,

    StepInto,

    StepOut,

    StepInstruction,

    Break {
        file: Option<PathBuf>,
        line: usize,
    },

    BreakIf {
        file: Option<PathBuf>,
        line: usize,
        condition: String,
    },

    DeleteBreakpoint {
        id: BreakpointId,
    },

    ClearBreakpoints,

    EnableBreakpoint {
        id: BreakpointId,
    },

    DisableBreakpoint {
        id: BreakpointId,
    },

    ListBreakpoints,

    Print {
        expression: String,
    },

    Locals,

    Globals,

    Backtrace,

    Where,

    Watch {
        expression: String,
    },

    Unwatch {
        id: u32,
    },

    ListWatches,

    List {
        lines: Option<usize>,
    },

    Help,

    Quit,

    Run,

    Restart,

    Set {
        variable: String,
        value: String,
    },

    Info {
        topic: String,
    },

    Unknown {
        command: String,
    },
}

pub struct DebugCommandParser;

impl DebugCommandParser {
    pub fn parse(input: &str) -> DebugCommand {
        let input = input.trim();

        if input.is_empty() {
            return DebugCommand::StepOver; // Default: step
        }

        let parts: Vec<&str> = input.splitn(2, ' ').collect();
        let cmd = parts[0].to_lowercase();
        let args = parts.get(1).map(|s| s.trim()).unwrap_or("");

        match cmd.as_str() {
            "c" | "continue" | "تابع" | "cont" => DebugCommand::Continue,

            "n" | "next" | "التالي" | "step" | "خطوة" => DebugCommand::StepOver,

            "s" | "stepi" | "into" | "داخل" => DebugCommand::StepInto,

            "o" | "out" | "finish" | "خارج" => DebugCommand::StepOut,

            "si" | "ni" | "inst" | "تعليمة" => DebugCommand::StepInstruction,

            "b" | "break" | "توقف" | "bp" => Self::parse_breakpoint(args),

            "d" | "delete" | "del" | "احذف" => {
                if let Ok(id) = args.parse::<u32>() {
                    DebugCommand::DeleteBreakpoint {
                        id: BreakpointId(id),
                    }
                } else {
                    DebugCommand::Unknown {
                        command: format!("delete {}", args),
                    }
                }
            }

            "clear" | "امسح" => DebugCommand::ClearBreakpoints,

            "enable" | "فعل" => {
                if let Ok(id) = args.parse::<u32>() {
                    DebugCommand::EnableBreakpoint {
                        id: BreakpointId(id),
                    }
                } else {
                    DebugCommand::Unknown {
                        command: format!("enable {}", args),
                    }
                }
            }

            "disable" | "عطل" => {
                if let Ok(id) = args.parse::<u32>() {
                    DebugCommand::DisableBreakpoint {
                        id: BreakpointId(id),
                    }
                } else {
                    DebugCommand::Unknown {
                        command: format!("disable {}", args),
                    }
                }
            }

            "bl" | "breakpoints" | "نقاط" => DebugCommand::ListBreakpoints,

            "p" | "print" | "اطبع" | "pr" => {
                if args.is_empty() {
                    DebugCommand::Unknown {
                        command: "print requires an expression".to_string(),
                    }
                } else {
                    DebugCommand::Print {
                        expression: args.to_string(),
                    }
                }
            }

            "l" | "locals" | "محليات" => DebugCommand::Locals,

            "g" | "globals" | "عالميات" => DebugCommand::Globals,

            "bt" | "backtrace" | "stack" | "مكدس" | "where" => DebugCommand::Backtrace,

            "w" | "loc" | "موقع" => DebugCommand::Where,

            "watch" | "راقب" => {
                if args.is_empty() {
                    DebugCommand::ListWatches
                } else {
                    DebugCommand::Watch {
                        expression: args.to_string(),
                    }
                }
            }

            "unwatch" | "الغ_مراقبة" => {
                if let Ok(id) = args.parse::<u32>() {
                    DebugCommand::Unwatch { id }
                } else {
                    DebugCommand::Unknown {
                        command: format!("unwatch {}", args),
                    }
                }
            }

            "list" | "سرد" | "src" => {
                let lines = args.parse::<usize>().ok();
                DebugCommand::List { lines }
            }

            "h" | "help" | "?" | "مساعدة" => DebugCommand::Help,

            "q" | "quit" | "exit" | "اخرج" => DebugCommand::Quit,

            "r" | "run" | "شغل" => DebugCommand::Run,

            "restart" | "اعد" => DebugCommand::Restart,

            "set" | "عين" => {
                let set_parts: Vec<&str> = args.splitn(2, '=').collect();
                if set_parts.len() == 2 {
                    DebugCommand::Set {
                        variable: set_parts[0].trim().to_string(),
                        value: set_parts[1].trim().to_string(),
                    }
                } else {
                    DebugCommand::Unknown {
                        command: format!("set {}", args),
                    }
                }
            }

            "i" | "info" | "معلومات" => DebugCommand::Info {
                topic: args.to_string(),
            },

            _ => DebugCommand::Unknown {
                command: input.to_string(),
            },
        }
    }

    fn parse_breakpoint(args: &str) -> DebugCommand {
        if args.is_empty() {
            return DebugCommand::Unknown {
                command: "break requires a line number".to_string(),
            };
        }

        if let Some(if_pos) = args.find(" if ") {
            let line_part = &args[..if_pos];
            let condition = &args[if_pos + 4..];

            if let Some((file, line)) = Self::parse_location(line_part) {
                return DebugCommand::BreakIf {
                    file,
                    line,
                    condition: condition.to_string(),
                };
            }
        }

        if let Some((file, line)) = Self::parse_location(args) {
            DebugCommand::Break { file, line }
        } else {
            DebugCommand::Unknown {
                command: format!("Invalid breakpoint location: {}", args),
            }
        }
    }

    fn parse_location(s: &str) -> Option<(Option<PathBuf>, usize)> {
        let s = s.trim();

        if let Some(colon_pos) = s.rfind(':') {
            let file_part = &s[..colon_pos];
            let line_part = &s[colon_pos + 1..];

            if let Ok(line) = line_part.parse::<usize>() {
                return Some((Some(PathBuf::from(file_part)), line));
            }
        }

        if let Ok(line) = s.parse::<usize>() {
            return Some((None, line));
        }

        None
    }

    pub fn help_text() -> &'static str {
        r#"
Tarqeem Debugger Commands / أوامر مصحح ترقيم
============================================

Execution / التنفيذ:
  r, run, شغل              - Run the program / تشغيل البرنامج
  c, continue, تابع        - Continue execution / متابعة التنفيذ
  n, next, خطوة            - Step over (next line) / الخطوة التالية
  s, stepi, داخل           - Step into function / خطوة داخل الدالة
  o, out, خارج             - Step out of function / خروج من الدالة
  restart, اعد             - Restart program / إعادة تشغيل البرنامج

Breakpoints / نقاط التوقف:
  b, break <line>          - Set breakpoint / تعيين نقطة توقف
  b <file>:<line>          - Set breakpoint in file / نقطة توقف في ملف
  b <line> if <cond>       - Conditional breakpoint / نقطة توقف شرطية
  d, delete <id>           - Delete breakpoint / حذف نقطة توقف
  clear, امسح              - Clear all breakpoints / مسح كل النقاط
  enable <id>              - Enable breakpoint / تفعيل نقطة توقف
  disable <id>             - Disable breakpoint / تعطيل نقطة توقف
  bl, breakpoints          - List breakpoints / عرض نقاط التوقف

Inspection / الفحص:
  p, print <expr>          - Print expression / طباعة تعبير
  l, locals                - Show local variables / عرض المتغيرات المحلية
  g, globals               - Show global variables / عرض المتغيرات العالمية
  bt, backtrace            - Show call stack / عرض مكدس الاستدعاء
  w, where                 - Show current location / عرض الموقع الحالي
  list [n]                 - Show source code / عرض الكود المصدري

Watches / المراقبة:
  watch <expr>             - Add watch expression / إضافة تعبير مراقبة
  unwatch <id>             - Remove watch / إزالة مراقبة

Other / أخرى:
  h, help, ?               - Show this help / عرض المساعدة
  q, quit, اخرج            - Quit debugger / الخروج من المصحح
"#
    }
}

impl DebugCommand {
    pub fn as_step_mode(&self) -> Option<StepMode> {
        match self {
            DebugCommand::StepOver => Some(StepMode::Over),
            DebugCommand::StepInto => Some(StepMode::Into),
            DebugCommand::StepOut => Some(StepMode::Out),
            DebugCommand::StepInstruction => Some(StepMode::Instruction),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_continue() {
        assert_eq!(DebugCommandParser::parse("c"), DebugCommand::Continue);
        assert_eq!(
            DebugCommandParser::parse("continue"),
            DebugCommand::Continue
        );
        assert_eq!(DebugCommandParser::parse("تابع"), DebugCommand::Continue);
    }

    #[test]
    fn test_parse_step() {
        assert_eq!(DebugCommandParser::parse("n"), DebugCommand::StepOver);
        assert_eq!(DebugCommandParser::parse("next"), DebugCommand::StepOver);
        assert_eq!(DebugCommandParser::parse("s"), DebugCommand::StepInto);
        assert_eq!(DebugCommandParser::parse("o"), DebugCommand::StepOut);
    }

    #[test]
    fn test_parse_breakpoint() {
        assert_eq!(
            DebugCommandParser::parse("b 10"),
            DebugCommand::Break {
                file: None,
                line: 10
            }
        );

        assert_eq!(
            DebugCommandParser::parse("break test.ترقيم:20"),
            DebugCommand::Break {
                file: Some(PathBuf::from("test.ترقيم")),
                line: 20
            }
        );
    }

    #[test]
    fn test_parse_conditional_breakpoint() {
        assert_eq!(
            DebugCommandParser::parse("b 10 if x > 5"),
            DebugCommand::BreakIf {
                file: None,
                line: 10,
                condition: "x > 5".to_string()
            }
        );
    }

    #[test]
    fn test_parse_print() {
        assert_eq!(
            DebugCommandParser::parse("p x"),
            DebugCommand::Print {
                expression: "x".to_string()
            }
        );

        assert_eq!(
            DebugCommandParser::parse("print myVar + 1"),
            DebugCommand::Print {
                expression: "myVar + 1".to_string()
            }
        );
    }

    #[test]
    fn test_parse_delete() {
        assert_eq!(
            DebugCommandParser::parse("d 1"),
            DebugCommand::DeleteBreakpoint {
                id: BreakpointId(1)
            }
        );
    }

    #[test]
    fn test_parse_locals_globals() {
        assert_eq!(DebugCommandParser::parse("l"), DebugCommand::Locals);
        assert_eq!(DebugCommandParser::parse("locals"), DebugCommand::Locals);
        assert_eq!(DebugCommandParser::parse("g"), DebugCommand::Globals);
    }

    #[test]
    fn test_parse_backtrace() {
        assert_eq!(DebugCommandParser::parse("bt"), DebugCommand::Backtrace);
        assert_eq!(DebugCommandParser::parse("stack"), DebugCommand::Backtrace);
        assert_eq!(DebugCommandParser::parse("مكدس"), DebugCommand::Backtrace);
    }

    #[test]
    fn test_parse_help_quit() {
        assert_eq!(DebugCommandParser::parse("h"), DebugCommand::Help);
        assert_eq!(DebugCommandParser::parse("?"), DebugCommand::Help);
        assert_eq!(DebugCommandParser::parse("q"), DebugCommand::Quit);
        assert_eq!(DebugCommandParser::parse("اخرج"), DebugCommand::Quit);
    }

    #[test]
    fn test_empty_input() {
        assert_eq!(DebugCommandParser::parse(""), DebugCommand::StepOver);
    }

    #[test]
    fn test_arabic_commands() {
        assert_eq!(DebugCommandParser::parse("خطوة"), DebugCommand::StepOver);
        assert_eq!(DebugCommandParser::parse("محليات"), DebugCommand::Locals);
        assert_eq!(DebugCommandParser::parse("مساعدة"), DebugCommand::Help);
    }
}
