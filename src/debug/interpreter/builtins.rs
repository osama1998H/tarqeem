//! Built-in function implementations for the debug interpreter.

use std::io::{self, Write};

use crate::interpreter::{RuntimeError, RuntimeResult, Value};

use super::DebugInterpreter;

impl DebugInterpreter {
    pub(crate) fn is_builtin(&self, name: &str) -> bool {
        matches!(
            name,
            "print"
                | "اطبع"
                | "println"
                | "input"
                | "ادخل"
                | "len"
                | "طول"
                | "type"
                | "نوع"
                | "int"
                | "عدد"
                | "float"
                | "عدد_عشري"
                | "str"
                | "نص"
                | "bool"
                | "منطقي"
                | "abs"
                | "sqrt"
                | "جذر"
                | "sin"
                | "cos"
                | "tan"
                | "floor"
                | "ceil"
                | "round"
        )
    }

    pub(crate) fn call_builtin(&mut self, name: &str, args: Vec<Value>) -> RuntimeResult<Value> {
        match name {
            "print" | "اطبع" | "println" => {
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

            "input" | "ادخل" => {
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

            "len" | "طول" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "len() requires 1 argument",
                        "طول() تتطلب معامل واحد",
                    )
                })?;

                match val {
                    Value::Array(arr) => Ok(Value::Int(arr.borrow().len() as i64)),
                    Value::String(s) => Ok(Value::Int(s.chars().count() as i64)),
                    _ => Err(RuntimeError::type_error("array or string", val.type_name())),
                }
            }

            "type" | "نوع" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "type() requires 1 argument",
                        "نوع() تتطلب معامل واحد",
                    )
                })?;
                Ok(Value::string(val.type_name_ar()))
            }

            "int" | "عدد" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "int() requires 1 argument",
                        "عدد() تتطلب معامل واحد",
                    )
                })?;

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

            "float" | "عدد_عشري" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "float() requires 1 argument",
                        "عدد_عشري() تتطلب معامل واحد",
                    )
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

            "str" | "نص" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "str() requires 1 argument",
                        "نص() تتطلب معامل واحد",
                    )
                })?;
                Ok(Value::string(val.to_display_string()))
            }

            "bool" | "منطقي" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "bool() requires 1 argument",
                        "منطقي() تتطلب معامل واحد",
                    )
                })?;
                Ok(Value::Bool(val.is_truthy()))
            }

            "abs" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "abs() requires 1 argument",
                        "abs() تتطلب معامل واحد",
                    )
                })?;

                match val {
                    Value::Int(i) => Ok(Value::Int(i.abs())),
                    Value::Float(f) => Ok(Value::Float(f.abs())),
                    _ => Err(RuntimeError::type_error("numeric", val.type_name())),
                }
            }

            "sqrt" | "جذر" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "sqrt() requires 1 argument",
                        "جذر() تتطلب معامل واحد",
                    )
                })?;

                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(f.sqrt()))
            }

            "sin" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "sin() requires 1 argument",
                        "sin() تتطلب معامل واحد",
                    )
                })?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(f.sin()))
            }

            "cos" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "cos() requires 1 argument",
                        "cos() تتطلب معامل واحد",
                    )
                })?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(f.cos()))
            }

            "tan" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "tan() requires 1 argument",
                        "tan() تتطلب معامل واحد",
                    )
                })?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(f.tan()))
            }

            "floor" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "floor() requires 1 argument",
                        "floor() تتطلب معامل واحد",
                    )
                })?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(f.floor()))
            }

            "ceil" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "ceil() requires 1 argument",
                        "ceil() تتطلب معامل واحد",
                    )
                })?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(f.ceil()))
            }

            "round" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "round() requires 1 argument",
                        "round() تتطلب معامل واحد",
                    )
                })?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(f.round()))
            }

            _ => Err(RuntimeError::undefined_function(name)),
        }
    }
}
