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
                | "ادخل"
                | "طول"
                | "نوع"
                | "عدد"
                | "عدد_عشري"
                | "نص"
                | "منطقي"
                | "جذر"
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
            //TODO: this need to convert to arabic مطلق
            "abs" => {
                let val = args
                    .first()
                    .ok_or_else(|| RuntimeError::invalid_operation("abs() تتطلب معامل واحد"))?;

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

            //TODO: this need to convert to arabic
            "sin" => {
                let val = args
                    .first()
                    .ok_or_else(|| RuntimeError::invalid_operation("sin() تتطلب معامل واحد"))?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(f.sin()))
            }

            //TODO: this need to convert to arabic
            "cos" => {
                let val = args
                    .first()
                    .ok_or_else(|| RuntimeError::invalid_operation("cos() تتطلب معامل واحد"))?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(f.cos()))
            }

            //TODO: this need to convert to arabic
            "tan" => {
                let val = args
                    .first()
                    .ok_or_else(|| RuntimeError::invalid_operation("tan() تتطلب معامل واحد"))?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(f.tan()))
            }

            //TODO: this need to convert to arabic
            "floor" => {
                let val = args
                    .first()
                    .ok_or_else(|| RuntimeError::invalid_operation("floor() تتطلب معامل واحد"))?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(f.floor()))
            }

            //TODO: this need to convert to arabic
            "ceil" => {
                let val = args
                    .first()
                    .ok_or_else(|| RuntimeError::invalid_operation("ceil() تتطلب معامل واحد"))?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(f.ceil()))
            }

            //TODO: this need to convert to arabic
            "round" => {
                let val = args
                    .first()
                    .ok_or_else(|| RuntimeError::invalid_operation("round() تتطلب معامل واحد"))?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(f.round()))
            }

            _ => Err(RuntimeError::undefined_function(name)),
        }
    }
}
