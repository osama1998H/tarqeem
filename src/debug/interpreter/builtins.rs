//! Built-in function implementations for the debug interpreter.

use std::io::{self, Write};

use crate::interpreter::{RuntimeError, RuntimeResult, Value};

use super::DebugInterpreter;

impl DebugInterpreter {
    pub(crate) fn is_builtin(&self, name: &str) -> bool {
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
        )
    }

    pub(crate) fn call_builtin(&mut self, name: &str, args: Vec<Value>) -> RuntimeResult<Value> {
        match name {
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
