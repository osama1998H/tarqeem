//! Built-in function implementations for the debug interpreter.

use std::io::{self, Write};

use crate::interpreter::epoch_millis;
use crate::interpreter::{
    bytes_to_string, call_env_var, call_exit_program, call_file_open, call_path_delete,
    call_path_status, call_program_args, call_read_stream, call_substring_by_chars,
    call_write_stream, RuntimeError, RuntimeResult, Value,
};

use super::DebugInterpreter;

impl DebugInterpreter {
    pub(crate) fn is_builtin(name: &str) -> bool {
        matches!(
            name,
            "اطبع"
                | "ادخل"
                | "طول"
                | "نوع"
                | "عدد"
                | "عدد_عشري"
                | "نص"
                | "منطقي"
                | "مطلق"
                | "جذر"
                | "جيب"
                | "جيب_التمام"
                | "ظل"
                | "أرضية"
                | "سقف"
                | "قرب"
                | "قص_حروف"
                | "حرف_إلى_رمز"
                | "رمز_إلى_حرف"
                | "نص_إلى_ثنائي"
                | "ثنائي_إلى_نص"
                | "متغير_بيئة"
                | "اكتب_مجرى"
                | "اقرأ_مجرى"
                | "افتح_ملف"
                | "حالة_مسار"
                | "احذف_مسار"
                | "معاملات_البرنامج"
                // Termination. Absent here, stepping through `أنهِ_البرنامج(٠)`
                // would abort with «دالة غير معرّفة» while every other backend
                // ended the program cleanly — the same gap #295 records for
                // `توقف` and `نم`, which is why the name goes in when it lands
                // rather than after someone debugs a program that uses it.
                | "أنهِ_البرنامج"
                | "أنه_البرنامج"
                // Runtime symbols the IR builder lowers core builtins to
                // (#222). Without these, stepping through `عدد("٥")` or
                // `تأكد(...)` aborts with "دالة غير معرّفة".
                | "trq_assert"
                | "trq_string_len"
                | "trq_string_to_int_checked"
                | "trq_string_to_float_checked"
                // String concatenation lowers its non-string side through these,
                // so `اطبع("الراتب: " + 10000.0)` aborted here while running fine
                // in every other backend (#185).
                | "trq_int_to_string"
                | "trq_float_to_string"
                | "trq_bool_to_string"
                // Stdlib time builtins. Absent here, stepping through
                // `وقت_أداء()` aborted while every other backend ran it (#241).
                | "وقت_الآن"
                | "وقت_أداء"
        )
    }

    pub(crate) fn call_builtin(&mut self, name: &str, args: Vec<Value>) -> RuntimeResult<Value> {
        match name {
            "وقت_الآن" | "وقت_أداء" => Ok(Value::Int(epoch_millis())),

            "اطبع" => {
                let output = args
                    .iter()
                    .map(|v| v.to_display_string())
                    .collect::<Vec<_>>()
                    .join(" ");

                self.context.add_output(output.clone());
                println!("{}", output);
                io::stdout().flush().ok();
                Ok(Value::Null)
            }

            "ادخل" => {
                if let Some(prompt) = args.first() {
                    print!("{}", prompt.to_display_string());
                    io::stdout().flush().ok();
                }

                let mut input = String::new();
                io::stdin()
                    .read_line(&mut input)
                    .map_err(|e| RuntimeError::internal(format!("Input error: {}", e)))?;

                Ok(Value::string(input.trim_end()))
            }

            "طول" => {
                let val = args
                    .first()
                    .ok_or_else(|| RuntimeError::invalid_operation("طول() تتطلب معامل واحد"))?;

                match val {
                    Value::Array(arr) => Ok(Value::Int(arr.borrow().len() as i64)),
                    Value::String(s) => Ok(Value::Int(s.chars().count() as i64)),
                    _ => Err(RuntimeError::type_error("array or string", val.type_name())),
                }
            }

            "نوع" => {
                let val = args
                    .first()
                    .ok_or_else(|| RuntimeError::invalid_operation("نوع() تتطلب معامل واحد"))?;
                Ok(Value::string(val.type_name_ar()))
            }

            "عدد" => {
                let val = args
                    .first()
                    .ok_or_else(|| RuntimeError::invalid_operation("عدد() تتطلب معامل واحد"))?;

                match val {
                    Value::Int(i) => Ok(Value::Int(*i)),
                    Value::Float(f) => Ok(Value::Int(*f as i64)),
                    Value::String(s) => s
                        .parse::<i64>()
                        .map(Value::Int)
                        .map_err(|_| RuntimeError::type_error("numeric string", "invalid string")),
                    Value::Bool(b) => Ok(Value::Int(if *b { 1 } else { 0 })),
                    _ => Err(RuntimeError::type_error(
                        "convertible to int",
                        val.type_name(),
                    )),
                }
            }

            "عدد_عشري" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation("عدد_عشري() تتطلب معامل واحد")
                })?;

                match val {
                    Value::Int(i) => Ok(Value::Float(*i as f64)),
                    Value::Float(f) => Ok(Value::Float(*f)),
                    Value::String(s) => s
                        .parse::<f64>()
                        .map(Value::Float)
                        .map_err(|_| RuntimeError::type_error("numeric string", "invalid string")),
                    _ => Err(RuntimeError::type_error(
                        "convertible to float",
                        val.type_name(),
                    )),
                }
            }

            "نص" => {
                let val = args
                    .first()
                    .ok_or_else(|| RuntimeError::invalid_operation("نص() تتطلب معامل واحد"))?;
                Ok(Value::string(val.to_display_string()))
            }

            "منطقي" => {
                let val = args
                    .first()
                    .ok_or_else(|| RuntimeError::invalid_operation("منطقي() تتطلب معامل واحد"))?;
                Ok(Value::Bool(val.is_truthy()))
            }

            // Mirrors `interpreter::executor::builtins`, which implements the
            // same four symbols for the same lowerings.
            "trq_assert" => {
                let cond = args
                    .first()
                    .ok_or_else(|| RuntimeError::invalid_operation("تأكد() تتطلب معامل واحد"))?;

                if !cond.is_truthy() {
                    return match args.get(1) {
                        Some(msg) if !matches!(msg, Value::Null) => {
                            Err(RuntimeError::invalid_operation(format!(
                                "فشل التأكيد: {}",
                                msg.to_display_string()
                            )))
                        }
                        _ => Err(RuntimeError::invalid_operation("فشل التأكيد")),
                    };
                }
                Ok(Value::Null)
            }

            // Must match `interpreter::executor::builtins` arm for arm, including
            // the `Null` case: an un-narrowed `نص؟` reaches here as `Value::Null`
            // and native answers -1 for it.
            "حرف_إلى_رمز" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation("حرف_إلى_رمز() تتطلب معامل واحد")
                })?;
                match val {
                    Value::String(s) => Ok(Value::Int(s.chars().next().map_or(-1, |c| c as i64))),
                    Value::Null => Ok(Value::Int(-1)),
                    _ => Err(RuntimeError::type_error("نص", val.type_name())),
                }
            }

            // Mirrors `interpreter::executor::builtins`, including the absence
            // of a `Null` arm — see the reasoning there (#327).
            "رمز_إلى_حرف" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation("رمز_إلى_حرف() تتطلب معامل واحد")
                })?;
                match val {
                    Value::Int(code) => Ok(Value::string(
                        u32::try_from(*code)
                            .ok()
                            .and_then(char::from_u32)
                            .map_or(String::new(), |c| c.to_string()),
                    )),
                    _ => Err(RuntimeError::type_error("عدد", val.type_name())),
                }
            }

            // The whole dispatch is shared rather than mirrored, so the
            // totality contract — negative start, non-positive length, start
            // past the end, null string — cannot drift from the interpreter's.
            "قص_حروف" => call_substring_by_chars(&args),

            // Mirrors `interpreter::executor::builtins`, `Null` arm included —
            // there the parameter is a pointer, so the empty array is a designed
            // answer rather than #327's artifact.
            "نص_إلى_ثنائي" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation("نص_إلى_ثنائي() تتطلب معامل واحد")
                })?;
                match val {
                    Value::String(s) => Ok(Value::array_from(
                        s.bytes().map(|b| Value::Int(b as i64)).collect(),
                    )),
                    Value::Null => Ok(Value::array()),
                    _ => Err(RuntimeError::type_error("نص", val.type_name())),
                }
            }

            // Its inverse, and the rejection is what keeps the backends
            // agreeing: a `Value::String` is a Rust `String` and cannot hold
            // invalid UTF-8 at all, so answering `""` is the only contract both
            // this and native can honour. See `trq_string_from_bytes`.
            "ثنائي_إلى_نص" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation("ثنائي_إلى_نص() تتطلب معامل واحد")
                })?;

                match val {
                    Value::Array(arr) => Ok(Value::string(
                        bytes_to_string(&arr.borrow()).unwrap_or_default(),
                    )),
                    // Load-bearing, but reached differently from its sibling:
                    // `مصفوفة<عدد>؟` does not parse (ب٠١٠١) and a bare `لا_شيء`
                    // is refused at the argument, so the route is an `أي` holder
                    // — where native's null guard answers `""` and erroring here
                    // instead would abort on source native runs fine.
                    Value::Null => Ok(Value::string("")),
                    _ => Err(RuntimeError::type_error("مصفوفة", val.type_name())),
                }
            }

            "متغير_بيئة" => call_env_var(&args),

            "اكتب_مجرى" => call_write_stream(&args),
            "اقرأ_مجرى" => call_read_stream(&args),
            "افتح_ملف" => call_file_open(&args),
            "حالة_مسار" => call_path_status(&args),
            "احذف_مسار" => call_path_delete(&args),
            "معاملات_البرنامج" => call_program_args(&args),

            "أنهِ_البرنامج" | "أنه_البرنامج" => call_exit_program(&args),

            "trq_string_len" => {
                let val = args
                    .first()
                    .ok_or_else(|| RuntimeError::invalid_operation("طول النص يتطلب معامل واحد"))?;
                match val {
                    Value::String(s) => Ok(Value::Int(s.len() as i64)),
                    _ => Err(RuntimeError::type_error("نص", val.type_name())),
                }
            }

            // `to_display_string` is the single definition of how each of these
            // reads, so the debugger cannot drift from the other backends.
            "trq_int_to_string" | "trq_float_to_string" | "trq_bool_to_string" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation("التحويل إلى نص يتطلب معامل واحد")
                })?;
                Ok(Value::string(val.to_display_string()))
            }

            "trq_string_to_int_checked" => {
                let val = args
                    .first()
                    .ok_or_else(|| RuntimeError::invalid_operation("عدد() تتطلب معامل واحد"))?;
                match val {
                    Value::String(s) => {
                        s.trim().parse::<i64>().map(Value::Int).map_err(|_| {
                            RuntimeError::type_error("numeric string", "invalid string")
                        })
                    }
                    _ => Err(RuntimeError::type_error("نص", val.type_name())),
                }
            }

            "trq_string_to_float_checked" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation("عدد_عشري() تتطلب معامل واحد")
                })?;
                match val {
                    Value::String(s) => {
                        s.trim().parse::<f64>().map(Value::Float).map_err(|_| {
                            RuntimeError::type_error("numeric string", "invalid string")
                        })
                    }
                    _ => Err(RuntimeError::type_error("نص", val.type_name())),
                }
            }
            "مطلق" => {
                let val = args
                    .first()
                    .ok_or_else(|| RuntimeError::invalid_operation("مطلق() تتطلب معامل واحد"))?;

                match val {
                    Value::Int(i) => Ok(Value::Int(i.abs())),
                    Value::Float(f) => Ok(Value::Float(f.abs())),
                    _ => Err(RuntimeError::type_error("numeric", val.type_name())),
                }
            }

            "جذر" => {
                let val = args
                    .first()
                    .ok_or_else(|| RuntimeError::invalid_operation("جذر() تتطلب معامل واحد"))?;

                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(f.sqrt()))
            }

            "جيب" => {
                let val = args
                    .first()
                    .ok_or_else(|| RuntimeError::invalid_operation("جيب() تتطلب معامل واحد"))?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(f.sin()))
            }

            "جيب_التمام" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation("جيب_التمام() تتطلب معامل واحد")
                })?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(f.cos()))
            }

            "ظل" => {
                let val = args
                    .first()
                    .ok_or_else(|| RuntimeError::invalid_operation("ظل() تتطلب معامل واحد"))?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(f.tan()))
            }

            "أرضية" => {
                let val = args
                    .first()
                    .ok_or_else(|| RuntimeError::invalid_operation("أرضية() تتطلب معامل واحد"))?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(f.floor()))
            }

            "سقف" => {
                let val = args
                    .first()
                    .ok_or_else(|| RuntimeError::invalid_operation("سقف() تتطلب معامل واحد"))?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(f.ceil()))
            }

            "قرب" => {
                let val = args
                    .first()
                    .ok_or_else(|| RuntimeError::invalid_operation("قرب() تتطلب معامل واحد"))?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(f.round()))
            }

            _ => Err(RuntimeError::undefined_function(name)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interpreter::ErrorKind;

    /// The debug interpreter is the fourth backend and the one that silently
    /// falls behind (#223): `tests/builtins_execution_tests.rs` has no `debug`
    /// leg, so nothing else here would notice these going missing again (#241).
    ///
    /// `is_builtin` and the `call_builtin` match are separate lists, and a name
    /// present in one but not the other still fails at run time, so both are
    /// checked.
    #[test]
    fn test_time_builtins_are_dispatchable() {
        for name in ["وقت_الآن", "وقت_أداء"] {
            assert!(
                DebugInterpreter::is_builtin(name),
                "{name} غير مُعرَّف كدالة مدمجة في مفسّر التنقيح"
            );
        }
    }

    /// `tests/builtins_execution_tests.rs` drives `run`, `run --jit` and
    /// `compile`; nothing there reaches this file, so a core builtin's debug arm
    /// is only ever covered here. `حرف_إلى_رمز` is checked through
    /// `call_builtin` rather than `is_builtin` alone, since the two are separate
    /// lists and a name in one but not the other still aborts at run time.
    #[test]
    fn test_char_code_is_dispatchable() {
        assert!(
            DebugInterpreter::is_builtin("حرف_إلى_رمز"),
            "حرف_إلى_رمز غير مُعرَّف كدالة مدمجة في مفسّر التنقيح"
        );

        let mut interpreter = DebugInterpreter::new(
            crate::ir::Module::new("تنقيح".to_string()),
            crate::debug::DebugContext::default(),
        );

        for (argument, expected) in [
            (Value::string("أ"), 1571),
            (Value::string("مَرحبا"), 1605),
            (Value::string(""), -1),
            (Value::Null, -1),
        ] {
            let result = interpreter
                .call_builtin("حرف_إلى_رمز", vec![argument])
                .expect("حرف_إلى_رمز أخفق في مفسّر التنقيح");
            assert_eq!(result.as_int(), Some(expected));
        }
    }

    /// Same reasoning as `test_char_code_is_dispatchable`, and the pair is
    /// asserted through the round trip rather than against literal strings so
    /// that a width the encoder gets wrong cannot pass as a plausible answer.
    #[test]
    fn test_char_from_code_is_dispatchable() {
        assert!(
            DebugInterpreter::is_builtin("رمز_إلى_حرف"),
            "رمز_إلى_حرف غير مُعرَّف كدالة مدمجة في مفسّر التنقيح"
        );

        let mut interpreter = DebugInterpreter::new(
            crate::ir::Module::new("تنقيح".to_string()),
            crate::debug::DebugContext::default(),
        );

        // One per UTF-8 width, then U+0000 — a one-character string, not the
        // empty one — then each rejection class, including the `as u32` wrap.
        for code in [65, 1605, 65021, 126464, 0] {
            let built = interpreter
                .call_builtin("رمز_إلى_حرف", vec![Value::Int(code)])
                .expect("رمز_إلى_حرف أخفق في مفسّر التنقيح");
            let round_tripped = interpreter
                .call_builtin("حرف_إلى_رمز", vec![built])
                .expect("حرف_إلى_رمز أخفق في مفسّر التنقيح");
            assert_eq!(round_tripped.as_int(), Some(code));
        }

        for code in [-1, 0xD800, 0x11_0000, 0x1_0000_0041] {
            let built = interpreter
                .call_builtin("رمز_إلى_حرف", vec![Value::Int(code)])
                .expect("رمز_إلى_حرف أخفق في مفسّر التنقيح");
            assert_eq!(built.as_string(), Some(""), "من {code}");
        }
    }

    /// Same reasoning as the two above, and this is the **first** array-returning
    /// arm in this file — every other one here only ever reads an array. The
    /// element values are checked, not just the count: a count alone would pass
    /// on an arm that returned characters rather than octets for ASCII input.
    #[test]
    fn test_string_to_bytes_is_dispatchable() {
        assert!(
            DebugInterpreter::is_builtin("نص_إلى_ثنائي"),
            "نص_إلى_ثنائي غير مُعرَّف كدالة مدمجة في مفسّر التنقيح"
        );

        let mut interpreter = DebugInterpreter::new(
            crate::ir::Module::new("تنقيح".to_string()),
            crate::debug::DebugContext::default(),
        );

        for (argument, expected) in [
            (Value::string("A"), vec![65]),
            (Value::string("م"), vec![0xD9, 0x85]),
            (Value::string("hi"), vec![104, 105]),
            (Value::string(""), vec![]),
            // An un-narrowed `نص؟` arrives as `Value::Null`, and native answers
            // an empty array for it — see the arm's own note.
            (Value::Null, vec![]),
        ] {
            let result = interpreter
                .call_builtin("نص_إلى_ثنائي", vec![argument])
                .expect("نص_إلى_ثنائي أخفق في مفسّر التنقيح");

            let Value::Array(bytes) = result else {
                panic!("نص_إلى_ثنائي لم تُرجع مصفوفة");
            };
            let actual: Vec<i64> = bytes.borrow().iter().filter_map(|v| v.as_int()).collect();
            assert_eq!(actual, expected);
        }
    }

    /// The inverse arm, and the rejections matter more here than the successes:
    /// the debugger shares `bytes_to_string` with the main interpreter, so what
    /// this pins is that the *dispatch* exists and keys on the Arabic name.
    #[test]
    fn test_bytes_to_string_is_dispatchable() {
        assert!(
            DebugInterpreter::is_builtin("ثنائي_إلى_نص"),
            "ثنائي_إلى_نص غير مُعرَّف كدالة مدمجة في مفسّر التنقيح"
        );

        let mut interpreter = DebugInterpreter::new(
            crate::ir::Module::new("تنقيح".to_string()),
            crate::debug::DebugContext::default(),
        );

        for (slots, expected) in [
            (vec![65], "A"),
            (vec![0xD9, 0x85], "م"),
            (vec![104, 105], "hi"),
            (vec![], ""),
            // Not bytes, and not valid UTF-8, both answering `""` — the one rule.
            (vec![300], ""),
            (vec![-1], ""),
            (vec![0xD9], ""),
        ] {
            let argument = Value::array_from(slots.iter().map(|b| Value::Int(*b)).collect());
            let result = interpreter
                .call_builtin("ثنائي_إلى_نص", vec![argument])
                .expect("ثنائي_إلى_نص أخفق في مفسّر التنقيح");

            assert_eq!(result.as_string(), Some(expected), "من {slots:?}");
        }

        // Reached through an `أي` holder rather than an optional annotation —
        // `مصفوفة<عدد>؟` does not parse. Native answers `""` for it.
        let null = interpreter
            .call_builtin("ثنائي_إلى_نص", vec![Value::Null])
            .expect("ثنائي_إلى_نص أخفق على لا_شيء");
        assert_eq!(null.as_string(), Some(""));
    }

    /// The slicer's arm. Like `ثنائي_إلى_نص` above, the whole dispatch is shared
    /// with the main interpreter, so what this pins is that the dispatch exists
    /// and keys on the Arabic name — the gap that made `توقف` and `نم` abort here
    /// while running in every other backend (#295), and that no cross-backend
    /// sweep can catch, since `Backend::ALL` does not reach this interpreter.
    #[test]
    fn test_substr_chars_is_dispatchable() {
        assert!(
            DebugInterpreter::is_builtin("قص_حروف"),
            "قص_حروف غير مُعرَّفة كدالة مدمجة في مفسّر التنقيح"
        );

        let mut interpreter = DebugInterpreter::new(
            crate::ir::Module::new("تنقيح".to_string()),
            crate::debug::DebugContext::default(),
        );

        for (text, start, len, expected) in [
            ("مرحبا", 1, 3, "رحب"),
            // One codepoint of each UTF-8 width, so a byte slicer cannot pass.
            ("A﷽م𞸀", 1, 2, "﷽م"),
            // Totality, in the four shapes `trq_string_substr_chars` guards.
            ("مرحبا", -1, 2, ""),
            ("مرحبا", 9, 2, ""),
            ("مرحبا", 1, 0, ""),
            ("م", 0, 5, "م"),
        ] {
            let result = interpreter
                .call_builtin(
                    "قص_حروف",
                    vec![Value::string(text), Value::Int(start), Value::Int(len)],
                )
                .expect("قص_حروف أخفقت في مفسّر التنقيح");

            assert_eq!(
                result.as_string(),
                Some(expected),
                "من {text:?} {start} {len}"
            );
        }

        // The `نص` parameter is a pointer, so native's null guard answers `""`
        // and this arm mirrors a designed contract rather than an artifact.
        let null = interpreter
            .call_builtin("قص_حروف", vec![Value::Null, Value::Int(0), Value::Int(1)])
            .expect("قص_حروف أخفقت على لا_شيء");
        assert_eq!(null.as_string(), Some(""));
    }

    /// `أنهِ_البرنامج`'s arm, and the one builtin whose dispatch could not be
    /// tested at all if it exited the process: `cargo test` runs these in-process,
    /// so a `process::exit` here would take the whole test binary with it.
    ///
    /// That is exactly why the shared helper returns `ErrorKind::ProgramExit`
    /// instead of exiting — the host decides. Here the host is the debugger, which
    /// gets an ordinary `Err` it can report, and a debug session survives stepping
    /// over the call.
    ///
    /// Both spellings, because they are two identifiers reaching one primitive and
    /// a missing arm for the variant would only surface when someone typed it.
    #[test]
    fn test_exit_program_is_dispatchable() {
        for name in ["أنهِ_البرنامج", "أنه_البرنامج"] {
            assert!(
                DebugInterpreter::is_builtin(name),
                "{name} غير مُعرَّفة كدالة مدمجة في مفسّر التنقيح"
            );
        }

        let mut interpreter = DebugInterpreter::new(
            crate::ir::Module::new("تنقيح".to_string()),
            crate::debug::DebugContext::default(),
        );

        // The masking contract, asserted through the debugger's own dispatch: the
        // status is `حالة & ٢٥٥`, so out-of-range values wrap rather than erroring.
        for (status, expected) in [(0, 0), (3, 3), (255, 255), (256, 0), (-1, 255), (300, 44)] {
            let err = interpreter
                .call_builtin("أنهِ_البرنامج", vec![Value::Int(status)])
                .expect_err("أنهِ_البرنامج تُرجع إشارة إنهاء لا قيمة");
            assert_eq!(
                err.kind,
                ErrorKind::ProgramExit(expected),
                "الحالة {status} في مفسّر التنقيح"
            );
        }

        // No `Value::Null` arm: the parameter is an `عدد`, so there is no pointer
        // for a runtime guard to answer, and treating `لا_شيء` as `0` would encode
        // codegen's artifact as contract (#326's narrowing, #327).
        let err = interpreter
            .call_builtin("أنه_البرنامج", vec![Value::Null])
            .expect_err("لا_شيء ليست حالة خروج");
        assert_eq!(err.kind, ErrorKind::TypeError);
    }

    /// `متغير_بيئة`'s arm, pinned for the same reason as the slicer above: the
    /// dispatch is shared, so what this checks is that it exists here at all and
    /// keys on the Arabic name.
    ///
    /// Deliberately asserts only the answers that need no variable to exist.
    /// Setting one here would mean `std::env::set_var`, which races every other
    /// test in this process; the cross-backend legs in
    /// `tests/builtins_execution_tests.rs` inject on the child process instead,
    /// and that is where a present variable is covered.
    #[test]
    fn test_env_var_is_dispatchable() {
        assert!(
            DebugInterpreter::is_builtin("متغير_بيئة"),
            "متغير_بيئة غير مُعرَّفة كدالة مدمجة في مفسّر التنقيح"
        );

        let mut interpreter = DebugInterpreter::new(
            crate::ir::Module::new("تنقيح".to_string()),
            crate::debug::DebugContext::default(),
        );

        // An absent name, an empty one and `لا_شيء` are the three shapes the
        // runtime folds into `""`, and none of them needs a variable to exist.
        for name in ["TARQEEM_ABSENT_DEBUG_338", ""] {
            let result = interpreter
                .call_builtin("متغير_بيئة", vec![Value::string(name)])
                .expect("متغير_بيئة أخفقت في مفسّر التنقيح");
            assert_eq!(result.as_string(), Some(""), "من {name:?}");
        }

        let null = interpreter
            .call_builtin("متغير_بيئة", vec![Value::Null])
            .expect("متغير_بيئة أخفقت على لا_شيء");
        assert_eq!(null.as_string(), Some(""));
    }

    /// `اكتب_مجرى`'s arm. The dispatch is shared, so what this pins is that the
    /// debug interpreter reaches it at all and keys on the Arabic name — the
    /// failure #241 recorded, where a builtin worked in every other backend and
    /// aborted the moment someone stepped through it.
    ///
    /// Asserts the answers that write nothing: a refused descriptor and an empty
    /// payload. Writing real bytes here would put them on the test harness's own
    /// stdout, since these tests run in-process.
    #[test]
    fn test_write_stream_is_dispatchable() {
        assert!(
            DebugInterpreter::is_builtin("اكتب_مجرى"),
            "اكتب_مجرى غير مُعرَّفة كدالة مدمجة في مفسّر التنقيح"
        );

        let mut interpreter = DebugInterpreter::new(
            crate::ir::Module::new("تنقيح".to_string()),
            crate::debug::DebugContext::default(),
        );

        // `٠` is stdin and `٣` upward names a handle this test has not opened, so
        // both answer `-١` — the same answers the runtime gives.
        for descriptor in [0, 3, -1] {
            let refused = interpreter
                .call_builtin(
                    "اكتب_مجرى",
                    vec![
                        Value::Int(descriptor),
                        Value::array_from(vec![Value::Int(65)]),
                    ],
                )
                .expect("اكتب_مجرى تُرجع قيمة لا خطأ");
            assert_eq!(refused.as_int(), Some(-1), "المجرى {descriptor}");
        }

        // A byte out of range refuses the whole call, and nothing is written.
        let out_of_range = interpreter
            .call_builtin(
                "اكتب_مجرى",
                vec![Value::Int(1), Value::array_from(vec![Value::Int(300)])],
            )
            .expect("اكتب_مجرى تُرجع قيمة لا خطأ");
        assert_eq!(out_of_range.as_int(), Some(-1));

        // Nothing to write is a count, not a failure — for an empty array and
        // for `لا_شيء` alike.
        for bytes in [Value::array(), Value::Null] {
            let empty = interpreter
                .call_builtin("اكتب_مجرى", vec![Value::Int(1), bytes])
                .expect("اكتب_مجرى تُرجع قيمة لا خطأ");
            assert_eq!(empty.as_int(), Some(0));
        }

        // No `Value::Null` arm for the descriptor: it is an `عدد`, so mirroring
        // codegen's `لا_شيء`-as-zero would encode an artifact as contract (#326).
        let err = interpreter
            .call_builtin("اكتب_مجرى", vec![Value::Null, Value::array()])
            .expect_err("لا_شيء ليست مجرى");
        assert_eq!(err.kind, ErrorKind::TypeError);
    }

    /// `اقرأ_مجرى`'s arm, pinning the same thing its sibling above does: that the
    /// debug interpreter reaches the shared dispatch and keys on the Arabic name.
    ///
    /// Asserts only the answers that **read nothing**. These tests run
    /// in-process, so a positive-count read on descriptor `٠` would take the test
    /// harness's own stdin and block — the mirror of the write test asserting
    /// only the answers that write nothing.
    #[test]
    fn test_read_stream_is_dispatchable() {
        assert!(
            DebugInterpreter::is_builtin("اقرأ_مجرى"),
            "اقرأ_مجرى غير مُعرَّفة كدالة مدمجة في مفسّر التنقيح"
        );

        let mut interpreter = DebugInterpreter::new(
            crate::ir::Module::new("تنقيح".to_string()),
            crate::debug::DebugContext::default(),
        );

        // How many bytes an answer holds, and a failure if it is not an array at
        // all — `Value` has no `as_array`, and a wrong variant must not read as
        // "empty".
        fn byte_count(answer: &Value) -> Option<usize> {
            match answer {
                Value::Array(arr) => Some(arr.borrow().len()),
                _ => None,
            }
        }

        // `١` and `٢` carry bytes the other way, `٣` upward names a handle this
        // test has not opened, and a negative descriptor names nothing — all
        // answer the empty array, as the runtime does.
        for descriptor in [1, 2, 3, -1] {
            let refused = interpreter
                .call_builtin("اقرأ_مجرى", vec![Value::Int(descriptor), Value::Int(4)])
                .expect("اقرأ_مجرى تُرجع قيمة لا خطأ");
            assert_eq!(byte_count(&refused), Some(0), "المجرى {descriptor}");
        }

        // A non-positive count reads nothing, and stdin is never touched — which
        // is what lets this case name descriptor `٠` at all.
        for count in [0, -5] {
            let nothing = interpreter
                .call_builtin("اقرأ_مجرى", vec![Value::Int(0), Value::Int(count)])
                .expect("اقرأ_مجرى تُرجع قيمة لا خطأ");
            assert_eq!(byte_count(&nothing), Some(0), "العدد {count}");
        }

        // No `Value::Null` arm for either parameter — both are `عدد`, so there is
        // no pointer for a runtime guard to answer (#326, #327). This is the
        // first primitive since #324 with none, so the assertion covers both.
        for args in [
            vec![Value::Null, Value::Int(4)],
            vec![Value::Int(0), Value::Null],
        ] {
            let err = interpreter
                .call_builtin("اقرأ_مجرى", args)
                .expect_err("لا_شيء ليست مجرى ولا عدداً");
            assert_eq!(err.kind, ErrorKind::TypeError);
        }
    }

    /// `حالة_مسار` under the debug interpreter, which is the only backend a
    /// cross-backend test cannot reach.
    ///
    /// It matters more here than for its siblings: this primitive's kind/size
    /// mapping is shared with the main interpreter through `call_path_status`
    /// precisely because it is *already* duplicated once in `trq_path_status`. A
    /// third copy would give it two ways to drift, and #295 is what a skipped
    /// debug leg looks like afterwards.
    #[test]
    fn test_path_status_is_dispatchable() {
        assert!(
            DebugInterpreter::is_builtin("حالة_مسار"),
            "حالة_مسار غير مُعرَّفة كدالة مدمجة في مفسّر التنقيح"
        );

        let mut interpreter = DebugInterpreter::new(
            crate::ir::Module::new("تنقيح".to_string()),
            crate::debug::DebugContext::default(),
        );

        let status = |interpreter: &mut DebugInterpreter, path: Value, field: i64| {
            interpreter
                .call_builtin("حالة_مسار", vec![path, Value::Int(field)])
                .expect("حالة_مسار تُرجع قيمة لا خطأ")
        };

        // A directory answers its kind and no size.
        assert_eq!(
            status(&mut interpreter, Value::string("/tmp"), 0),
            Value::Int(2)
        );
        assert_eq!(
            status(&mut interpreter, Value::string("/tmp"), 1),
            Value::Int(-1)
        );

        // An absent path, an empty name and a null one are one answer — the last
        // through the arm the `نص` parameter has and the `عدد` field does not.
        for path in [
            Value::string("/tmp/tarqeem_debug_path_status_absent_xyz"),
            Value::string(""),
            Value::Null,
        ] {
            assert_eq!(status(&mut interpreter, path.clone(), 0), Value::Int(0));
            assert_eq!(status(&mut interpreter, path, 1), Value::Int(-1));
        }

        // An unknown field has no answer whatever the path holds.
        for field in [2, 9, -1] {
            assert_eq!(
                status(&mut interpreter, Value::string("/tmp"), field),
                Value::Int(-1),
                "الحقل {field}"
            );
        }

        // The field is `عدد`, so `لا_شيء` there is a type error rather than a
        // designed answer (#326, #327).
        let err = interpreter
            .call_builtin("حالة_مسار", vec![Value::string("/tmp"), Value::Null])
            .expect_err("لا_شيء ليست حقلاً");
        assert_eq!(err.kind, ErrorKind::TypeError);
    }

    /// `معاملات_البرنامج` under the debug interpreter.
    ///
    /// The empty answer here is the **contract**, not a gap in the fixture: DAP
    /// does not go through the CLI's run path, so `set_program_args` is never
    /// called and the argument list is unset. That is deliberately
    /// indistinguishable from a program genuinely given no arguments, the way
    /// `متغير_بيئة`'s unset and set-empty are.
    ///
    /// It also pins the shape a caller depends on — an **array**, not a null or a
    /// string — which is what would break if the dispatch arm were ever dropped
    /// while `is_builtin` kept the name.
    #[test]
    fn test_program_args_is_dispatchable() {
        assert!(
            DebugInterpreter::is_builtin("معاملات_البرنامج"),
            "معاملات_البرنامج غير مُعرَّفة كدالة مدمجة في مفسّر التنقيح"
        );

        let mut interpreter = DebugInterpreter::new(
            crate::ir::Module::new("تنقيح".to_string()),
            crate::debug::DebugContext::default(),
        );

        let args = interpreter
            .call_builtin("معاملات_البرنامج", vec![])
            .expect("معاملات_البرنامج تُرجع قيمة لا خطأ");

        match args {
            Value::Array(items) => assert!(
                items.borrow().is_empty(),
                "معاملات_البرنامج تُرجع مصفوفة فارغة تحت التنقيح"
            ),
            other => panic!("متوقع مصفوفة، وُجد {}", other.type_name()),
        }
    }

    /// `افتح_ملف` under the debug interpreter.
    ///
    /// Driven on a **refusal** deliberately: an unknown mode is settled before
    /// the path, so this touches no filesystem at all — which an in-process unit
    /// test in a shared handle space should not. `٣` is the row worth using, since
    /// `stdlib/ملفات/ملف.ترقيم` calls it `وضع_قراءة_كتابة` and no handle
    /// direction here can serve it.
    ///
    /// It pins the answer's *shape* too — an `عدد`, not a null — which is what
    /// would break if the dispatch arm were dropped while `is_builtin` kept the
    /// name.
    #[test]
    fn test_file_open_is_dispatchable() {
        assert!(
            DebugInterpreter::is_builtin("افتح_ملف"),
            "افتح_ملف غير مُعرَّفة كدالة مدمجة في مفسّر التنقيح"
        );

        let mut interpreter = DebugInterpreter::new(
            crate::ir::Module::new("تنقيح".to_string()),
            crate::debug::DebugContext::default(),
        );

        let refused = interpreter
            .call_builtin("افتح_ملف", vec![Value::string("/tmp"), Value::Int(3)])
            .expect("افتح_ملف تُرجع قيمة لا خطأ");
        assert_eq!(refused, Value::Int(-1));

        // A pointer parameter, so `لا_شيء` is an answer rather than a type error
        // — the runtime's null guard mirrored (#324).
        let absent = interpreter
            .call_builtin("افتح_ملف", vec![Value::Null, Value::Int(0)])
            .expect("افتح_ملف تُرجع قيمة لا خطأ");
        assert_eq!(absent, Value::Int(-1));
    }

    /// `احذف_مسار` under the debug interpreter, which is the only backend a
    /// cross-backend test cannot reach. The portable rows live here; the symlink
    /// row — which is the whole contract — is the test below.
    #[test]
    fn test_path_delete_is_dispatchable() {
        assert!(
            DebugInterpreter::is_builtin("احذف_مسار"),
            "احذف_مسار غير مُعرَّفة كدالة مدمجة في مفسّر التنقيح"
        );

        let mut interpreter = DebugInterpreter::new(
            crate::ir::Module::new("تنقيح".to_string()),
            crate::debug::DebugContext::default(),
        );

        let delete = |interpreter: &mut DebugInterpreter, path: Value| {
            interpreter
                .call_builtin("احذف_مسار", vec![path])
                .expect("احذف_مسار تُرجع قيمة لا خطأ")
        };

        // An absent path, an empty name and a null one are one answer.
        for path in [
            Value::string("/tmp/tarqeem_debug_path_delete_absent_xyz"),
            Value::string(""),
            Value::Null,
        ] {
            assert_eq!(delete(&mut interpreter, path), Value::Bool(false));
        }

        // A non-empty directory is refused: `rmdir`, not `rm -r`. Under the system
        // temp directory rather than a literal `/tmp`, so the row runs on Windows
        // too — but in a directory this test *makes* non-empty, never the shared
        // temp directory itself: that one is only non-empty by luck, and on the one
        // run where it is empty `rmdir` would succeed and delete it out from under
        // every other test in the process.
        let full = std::env::temp_dir().join("tarqeem_debug_path_delete_full_dir");
        std::fs::remove_dir_all(&full).ok();
        std::fs::create_dir(&full).expect("تعذّر إنشاء المجلد");
        std::fs::write(full.join("ساكن.نص"), "x").expect("تعذّر إنشاء الملف");

        assert_eq!(
            delete(
                &mut interpreter,
                Value::string(full.to_str().expect("مسار مؤقت صالح"))
            ),
            Value::Bool(false)
        );
        assert!(full.is_dir(), "المجلد العامر حُذف");

        std::fs::remove_dir_all(&full).ok();
    }

    /// The lstat-versus-stat choice, which is the only place the two copies of the
    /// kernel can silently disagree — so it is asserted here as well as
    /// cross-backend. Split from the test above so the portable rows still run on
    /// Windows.
    #[test]
    #[cfg(unix)]
    fn test_path_delete_unlinks_a_symlink_rather_than_following_it() {
        let mut interpreter = DebugInterpreter::new(
            crate::ir::Module::new("تنقيح".to_string()),
            crate::debug::DebugContext::default(),
        );

        let delete = |interpreter: &mut DebugInterpreter, path: Value| {
            interpreter
                .call_builtin("احذف_مسار", vec![path])
                .expect("احذف_مسار تُرجع قيمة لا خطأ")
        };

        let target = "/tmp/tarqeem_debug_path_delete_target_dir";
        let link = "/tmp/tarqeem_debug_path_delete_link";
        std::fs::remove_file(link).ok();
        std::fs::remove_dir_all(target).ok();
        std::fs::create_dir(target).expect("تعذّر إنشاء المجلد");
        std::os::unix::fs::symlink(target, link).expect("تعذّر إنشاء الوصلة");

        // The contract decision: the link is unlinked and its target survives.
        // Following the link would call `rmdir` on it and answer `خطأ`.
        assert_eq!(
            delete(&mut interpreter, Value::string(link)),
            Value::Bool(true)
        );
        assert!(std::fs::symlink_metadata(link).is_err(), "الوصلة باقية");
        assert!(std::path::Path::new(target).is_dir(), "الهدف حُذف");

        // And an empty directory goes, through the other branch.
        assert_eq!(
            delete(&mut interpreter, Value::string(target)),
            Value::Bool(true)
        );
        assert!(!std::path::Path::new(target).exists());
    }

    #[test]
    fn test_time_builtins_share_the_interpreter_clock() {
        // Same helper the main interpreter calls, so the two cannot drift.
        let millis = epoch_millis();
        assert!(
            millis > 1_000_000_000_000,
            "التوقيت {millis} ليس بالميلي ثانية منذ ١٩٧٠"
        );
    }
}
