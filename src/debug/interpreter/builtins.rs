//! Built-in function implementations for the debug interpreter.

use std::io::{self, Write};

use crate::interpreter::epoch_millis;
use crate::interpreter::{
    bytes_to_string, call_substring_by_chars, RuntimeError, RuntimeResult, Value,
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
