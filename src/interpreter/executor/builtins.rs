//! Built-in function implementations for the interpreter.
//!
//! This module provides the interpreter's built-in functions including
//! I/O operations, math functions, type conversions, and utility functions.

use std::io::{self, Write};

use super::{Interpreter, RuntimeError, RuntimeResult, Value};

impl Interpreter {
    pub(crate) fn is_builtin(&self, name: &str) -> bool {
        matches!(
            name,
            "print"
                | "اطبع"
                | "println"
                | "طباعة"
                | "اطبع_سطر"
                | "input"
                | "ادخل"
                | "ادخل_رسالة"
                | "input_prompt"
                | "ادخل_عدد"
                | "input_int"
                | "ادخل_عشري"
                | "input_float"
                | "len"
                | "طول"
                | "length"
                | "type"
                | "نوع"
                | "typeof"
                | "int"
                | "عدد"
                | "float"
                | "عدد_عشري"
                | "str"
                | "نص"
                | "string"
                | "bool"
                | "منطقي"
                | "abs"
                | "مطلق"
                | "pow"
                | "قوة"
                | "sqrt"
                | "جذر"
                | "cbrt"
                | "جذر_تكعيبي"
                | "log"
                | "لوغاريتم"
                | "log10"
                | "لوغ10"
                | "لوغاريتم10"
                | "log2"
                | "لوغ2"
                | "exp"
                | "أس"
                | "أسي"
                | "floor"
                | "أرضية"
                | "ceil"
                | "سقف"
                | "round"
                | "قرب"
                | "تقريب"
                | "trunc"
                | "اقتطع"
                | "min"
                | "أقل"
                | "أدنى"
                | "max"
                | "أكبر"
                | "أقصى"
                | "clamp"
                | "حصر"
                | "sign"
                | "علامة"
                | "gcd"
                | "قاسم_مشترك"
                | "lcm"
                | "مضاعف_مشترك"
                | "factorial"
                | "عاملي"
                | "sin"
                | "جا"
                | "جيب"
                | "cos"
                | "جتا"
                | "جيب_التمام"
                | "tan"
                | "ظا"
                | "ظل"
                | "cot"
                | "ظتا"
                | "ظل_التمام"
                | "sec"
                | "قا"
                | "قاطع"
                | "csc"
                | "قتا"
                | "قاطع_التمام"
                | "asin"
                | "جا_عكسي"
                | "جيب_عكسي"
                | "acos"
                | "جتا_عكسي"
                | "جيب_تمام_عكسي"
                | "atan"
                | "ظا_عكسي"
                | "ظل_عكسي"
                | "atan2"
                | "ظا_عكسي2"
                | "ظل_عكسي2"
                | "sinh"
                | "جا_زائدي"
                | "جيب_زائدي"
                | "cosh"
                | "جتا_زائدي"
                | "جيب_تمام_زائدي"
                | "tanh"
                | "ظا_زائدي"
                | "ظل_زائدي"
                | "to_radians"
                | "الى_راديان"
                | "راديان"
                | "to_degrees"
                | "الى_درجات"
                | "درجات"
                | "random"
                | "عشوائي"
                | "random_int"
                | "random_range"
                | "عشوائي_بين"
                | "random_float"
                | "عشوائي_عشري"
                | "random_bool"
                | "عشوائي_منطقي"
                | "assert"
                | "تأكد"
                | "assert_msg"
                | "تأكد_رسالة"
                | "panic"
                | "توقف"
                | "sleep"
                | "نم"
                | "time_now"
                | "وقت_الآن"
                | "trq_int_to_string"
                | "trq_float_to_string"
                | "trq_bool_to_string"
        )
    }

    pub(crate) fn call_builtin(&mut self, name: &str, args: Vec<Value>) -> RuntimeResult<Value> {
        match name {
            "print" | "اطبع" | "println" | "طباعة" | "اطبع_سطر" => {
                let output = args
                    .iter()
                    .map(|v| v.to_display_string())
                    .collect::<Vec<_>>()
                    .join(" ");

                if self.capture_output {
                    self.output.push(output);
                } else {
                    println!("{}", output);
                    io::stdout().flush().ok();
                }
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

            "len" | "طول" | "length" => {
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

            "type" | "نوع" | "typeof" => {
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

            "str" | "نص" | "string" => {
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

            "abs" | "مطلق" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "abs() requires 1 argument",
                        "مطلق() تتطلب معامل واحد",
                    )
                })?;

                match val {
                    Value::Int(i) => Ok(Value::Int(i.abs())),
                    Value::Float(f) => Ok(Value::Float(f.abs())),
                    _ => Err(RuntimeError::type_error("numeric", val.type_name())),
                }
            }

            "pow" | "قوة" => {
                let base = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "pow() requires 2 arguments",
                        "قوة() تتطلب معاملين",
                    )
                })?;
                let exp = args.get(1).ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "pow() requires 2 arguments",
                        "قوة() تتطلب معاملين",
                    )
                })?;

                match (base, exp) {
                    (Value::Int(b), Value::Int(e)) if *e >= 0 => Ok(Value::Int(b.pow(*e as u32))),
                    (Value::Int(b), Value::Int(e)) => Ok(Value::Float((*b as f64).powf(*e as f64))),
                    _ => {
                        let b = base
                            .as_float()
                            .ok_or_else(|| RuntimeError::type_error("numeric", base.type_name()))?;
                        let e = exp
                            .as_float()
                            .ok_or_else(|| RuntimeError::type_error("numeric", exp.type_name()))?;
                        Ok(Value::Float(b.powf(e)))
                    }
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

            "cbrt" | "جذر_تكعيبي" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "cbrt() requires 1 argument",
                        "جذر_تكعيبي() تتطلب معامل واحد",
                    )
                })?;

                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(f.cbrt()))
            }

            "log" | "لوغاريتم" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "log() requires 1 argument",
                        "لوغاريتم() تتطلب معامل واحد",
                    )
                })?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(f.ln()))
            }

            "log10" | "لوغ10" | "لوغاريتم10" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "log10() requires 1 argument",
                        "لوغ10() تتطلب معامل واحد",
                    )
                })?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(f.log10()))
            }

            "log2" | "لوغ2" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "log2() requires 1 argument",
                        "لوغ2() تتطلب معامل واحد",
                    )
                })?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(f.log2()))
            }

            "exp" | "أس" | "أسي" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "exp() requires 1 argument",
                        "أس() تتطلب معامل واحد",
                    )
                })?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(f.exp()))
            }

            "floor" | "أرضية" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "floor() requires 1 argument",
                        "أرضية() تتطلب معامل واحد",
                    )
                })?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(f.floor()))
            }

            "ceil" | "سقف" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "ceil() requires 1 argument",
                        "سقف() تتطلب معامل واحد",
                    )
                })?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(f.ceil()))
            }

            "round" | "قرب" | "تقريب" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "round() requires 1 argument",
                        "قرب() تتطلب معامل واحد",
                    )
                })?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(f.round()))
            }

            "trunc" | "اقتطع" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "trunc() requires 1 argument",
                        "اقتطع() تتطلب معامل واحد",
                    )
                })?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(f.trunc()))
            }

            "min" | "أقل" | "أدنى" => {
                let a = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "min() requires 2 arguments",
                        "أقل() تتطلب معاملين",
                    )
                })?;
                let b = args.get(1).ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "min() requires 2 arguments",
                        "أقل() تتطلب معاملين",
                    )
                })?;

                match (a, b) {
                    (Value::Int(x), Value::Int(y)) => Ok(Value::Int(*x.min(y))),
                    (Value::Float(x), Value::Float(y)) => Ok(Value::Float(x.min(*y))),
                    (Value::Int(x), Value::Float(y)) => Ok(Value::Float((*x as f64).min(*y))),
                    (Value::Float(x), Value::Int(y)) => Ok(Value::Float(x.min(*y as f64))),
                    _ => Err(RuntimeError::type_error("numeric", a.type_name())),
                }
            }

            "max" | "أكبر" | "أقصى" => {
                let a = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "max() requires 2 arguments",
                        "أكبر() تتطلب معاملين",
                    )
                })?;
                let b = args.get(1).ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "max() requires 2 arguments",
                        "أكبر() تتطلب معاملين",
                    )
                })?;

                match (a, b) {
                    (Value::Int(x), Value::Int(y)) => Ok(Value::Int(*x.max(y))),
                    (Value::Float(x), Value::Float(y)) => Ok(Value::Float(x.max(*y))),
                    (Value::Int(x), Value::Float(y)) => Ok(Value::Float((*x as f64).max(*y))),
                    (Value::Float(x), Value::Int(y)) => Ok(Value::Float(x.max(*y as f64))),
                    _ => Err(RuntimeError::type_error("numeric", a.type_name())),
                }
            }

            "clamp" | "حصر" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "clamp() requires 3 arguments",
                        "حصر() تتطلب ثلاثة معاملات",
                    )
                })?;
                let min_val = args.get(1).ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "clamp() requires 3 arguments",
                        "حصر() تتطلب ثلاثة معاملات",
                    )
                })?;
                let max_val = args.get(2).ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "clamp() requires 3 arguments",
                        "حصر() تتطلب ثلاثة معاملات",
                    )
                })?;

                match (val, min_val, max_val) {
                    (Value::Int(v), Value::Int(mn), Value::Int(mx)) => {
                        Ok(Value::Int(*v.max(mn).min(mx)))
                    }
                    _ => {
                        let v = val
                            .as_float()
                            .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                        let mn = min_val.as_float().ok_or_else(|| {
                            RuntimeError::type_error("numeric", min_val.type_name())
                        })?;
                        let mx = max_val.as_float().ok_or_else(|| {
                            RuntimeError::type_error("numeric", max_val.type_name())
                        })?;
                        Ok(Value::Float(v.max(mn).min(mx)))
                    }
                }
            }

            "sign" | "علامة" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "sign() requires 1 argument",
                        "علامة() تتطلب معامل واحد",
                    )
                })?;

                match val {
                    Value::Int(i) => Ok(Value::Int(i.signum())),
                    Value::Float(f) => {
                        if f.is_nan() {
                            Ok(Value::Float(f64::NAN))
                        } else if *f > 0.0 {
                            Ok(Value::Float(1.0))
                        } else if *f < 0.0 {
                            Ok(Value::Float(-1.0))
                        } else {
                            Ok(Value::Float(0.0))
                        }
                    }
                    _ => Err(RuntimeError::type_error("numeric", val.type_name())),
                }
            }

            "gcd" | "قاسم_مشترك" => {
                let a = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "gcd() requires 2 arguments",
                        "قاسم_مشترك() تتطلب معاملين",
                    )
                })?;
                let b = args.get(1).ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "gcd() requires 2 arguments",
                        "قاسم_مشترك() تتطلب معاملين",
                    )
                })?;

                let x = a
                    .as_int()
                    .ok_or_else(|| RuntimeError::type_error("int", a.type_name()))?;
                let y = b
                    .as_int()
                    .ok_or_else(|| RuntimeError::type_error("int", b.type_name()))?;

                fn gcd(mut a: i64, mut b: i64) -> i64 {
                    a = a.abs();
                    b = b.abs();
                    while b != 0 {
                        let t = b;
                        b = a % b;
                        a = t;
                    }
                    a
                }

                Ok(Value::Int(gcd(x, y)))
            }

            "lcm" | "مضاعف_مشترك" => {
                let a = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "lcm() requires 2 arguments",
                        "مضاعف_مشترك() تتطلب معاملين",
                    )
                })?;
                let b = args.get(1).ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "lcm() requires 2 arguments",
                        "مضاعف_مشترك() تتطلب معاملين",
                    )
                })?;

                let x = a
                    .as_int()
                    .ok_or_else(|| RuntimeError::type_error("int", a.type_name()))?;
                let y = b
                    .as_int()
                    .ok_or_else(|| RuntimeError::type_error("int", b.type_name()))?;

                fn gcd(mut a: i64, mut b: i64) -> i64 {
                    a = a.abs();
                    b = b.abs();
                    while b != 0 {
                        let t = b;
                        b = a % b;
                        a = t;
                    }
                    a
                }

                if x == 0 || y == 0 {
                    Ok(Value::Int(0))
                } else {
                    Ok(Value::Int((x.abs() / gcd(x, y)) * y.abs()))
                }
            }

            "factorial" | "عاملي" => {
                let n = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "factorial() requires 1 argument",
                        "عاملي() تتطلب معامل واحد",
                    )
                })?;

                let n = n
                    .as_int()
                    .ok_or_else(|| RuntimeError::type_error("int", n.type_name()))?;

                if n < 0 {
                    return Err(RuntimeError::invalid_operation(
                        "factorial() requires non-negative argument",
                        "عاملي() تتطلب عدد غير سالب",
                    ));
                }

                let mut result: i64 = 1;
                for i in 2..=n {
                    result = result.saturating_mul(i);
                }
                Ok(Value::Int(result))
            }

            "sin" | "جا" | "جيب" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "sin() requires 1 argument",
                        "جيب() تتطلب معامل واحد",
                    )
                })?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(f.sin()))
            }

            "cos" | "جتا" | "جيب_التمام" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "cos() requires 1 argument",
                        "جيب_التمام() تتطلب معامل واحد",
                    )
                })?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(f.cos()))
            }

            "tan" | "ظا" | "ظل" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "tan() requires 1 argument",
                        "ظل() تتطلب معامل واحد",
                    )
                })?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(f.tan()))
            }

            "cot" | "ظتا" | "ظل_التمام" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "cot() requires 1 argument",
                        "ظل_التمام() تتطلب معامل واحد",
                    )
                })?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(1.0 / f.tan()))
            }

            "sec" | "قا" | "قاطع" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "sec() requires 1 argument",
                        "قاطع() تتطلب معامل واحد",
                    )
                })?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(1.0 / f.cos()))
            }

            "csc" | "قتا" | "قاطع_التمام" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "csc() requires 1 argument",
                        "قاطع_التمام() تتطلب معامل واحد",
                    )
                })?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(1.0 / f.sin()))
            }

            "asin" | "جا_عكسي" | "جيب_عكسي" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "asin() requires 1 argument",
                        "جيب_عكسي() تتطلب معامل واحد",
                    )
                })?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(f.asin()))
            }

            "acos" | "جتا_عكسي" | "جيب_تمام_عكسي" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "acos() requires 1 argument",
                        "جيب_تمام_عكسي() تتطلب معامل واحد",
                    )
                })?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(f.acos()))
            }

            "atan" | "ظا_عكسي" | "ظل_عكسي" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "atan() requires 1 argument",
                        "ظل_عكسي() تتطلب معامل واحد",
                    )
                })?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(f.atan()))
            }

            "atan2" | "ظا_عكسي2" | "ظل_عكسي2" => {
                let y = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "atan2() requires 2 arguments",
                        "ظل_عكسي2() تتطلب معاملين",
                    )
                })?;
                let x = args.get(1).ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "atan2() requires 2 arguments",
                        "ظل_عكسي2() تتطلب معاملين",
                    )
                })?;

                let y = y
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", y.type_name()))?;
                let x = x
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", x.type_name()))?;

                Ok(Value::Float(y.atan2(x)))
            }

            "sinh" | "جا_زائدي" | "جيب_زائدي" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "sinh() requires 1 argument",
                        "جيب_زائدي() تتطلب معامل واحد",
                    )
                })?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(f.sinh()))
            }

            "cosh" | "جتا_زائدي" | "جيب_تمام_زائدي" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "cosh() requires 1 argument",
                        "جيب_تمام_زائدي() تتطلب معامل واحد",
                    )
                })?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(f.cosh()))
            }

            "tanh" | "ظا_زائدي" | "ظل_زائدي" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "tanh() requires 1 argument",
                        "ظل_زائدي() تتطلب معامل واحد",
                    )
                })?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(f.tanh()))
            }

            "to_radians" | "الى_راديان" | "راديان" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "to_radians() requires 1 argument",
                        "الى_راديان() تتطلب معامل واحد",
                    )
                })?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(f.to_radians()))
            }

            "to_degrees" | "الى_درجات" | "درجات" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "to_degrees() requires 1 argument",
                        "الى_درجات() تتطلب معامل واحد",
                    )
                })?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(f.to_degrees()))
            }

            "random" | "عشوائي" | "random_int" => {
                use std::time::{SystemTime, UNIX_EPOCH};
                let seed = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_nanos() as u64)
                    .unwrap_or(12345);
                let random = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                Ok(Value::Int((random % (i64::MAX as u64 + 1)) as i64))
            }

            "random_range" | "عشوائي_بين" => {
                let min_val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "random_range() requires 2 arguments",
                        "عشوائي_بين() تتطلب معاملين",
                    )
                })?;
                let max_val = args.get(1).ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "random_range() requires 2 arguments",
                        "عشوائي_بين() تتطلب معاملين",
                    )
                })?;

                let min = min_val
                    .as_int()
                    .ok_or_else(|| RuntimeError::type_error("int", min_val.type_name()))?;
                let max = max_val
                    .as_int()
                    .ok_or_else(|| RuntimeError::type_error("int", max_val.type_name()))?;

                use std::time::{SystemTime, UNIX_EPOCH};
                let seed = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_nanos() as u64)
                    .unwrap_or(12345);
                let random = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                let range = (max - min + 1) as u64;
                let result = min + (random % range) as i64;
                Ok(Value::Int(result))
            }

            "random_float" | "عشوائي_عشري" => {
                use std::time::{SystemTime, UNIX_EPOCH};
                let seed = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_nanos() as u64)
                    .unwrap_or(12345);
                let random = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                let result = (random as f64) / (u64::MAX as f64);
                Ok(Value::Float(result))
            }

            "random_bool" | "عشوائي_منطقي" => {
                use std::time::{SystemTime, UNIX_EPOCH};
                let seed = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_nanos() as u64)
                    .unwrap_or(12345);
                let random = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                Ok(Value::Bool(random.is_multiple_of(2)))
            }

            "assert" | "تأكد" => {
                let cond = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "assert() requires 1 argument",
                        "تأكد() تتطلب معامل واحد",
                    )
                })?;

                if !cond.is_truthy() {
                    return Err(RuntimeError::invalid_operation(
                        "Assertion failed",
                        "فشل التأكيد",
                    ));
                }
                Ok(Value::Null)
            }

            "assert_msg" | "تأكد_رسالة" => {
                let cond = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "assert_msg() requires 2 arguments",
                        "تأكد_رسالة() تتطلب معاملين",
                    )
                })?;
                let msg = args.get(1).ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "assert_msg() requires 2 arguments",
                        "تأكد_رسالة() تتطلب معاملين",
                    )
                })?;

                if !cond.is_truthy() {
                    let msg_str = msg.to_display_string();
                    return Err(RuntimeError::invalid_operation(
                        format!("Assertion failed: {}", msg_str),
                        format!("فشل التأكيد: {}", msg_str),
                    ));
                }
                Ok(Value::Null)
            }

            "panic" | "توقف" => {
                let msg = args
                    .first()
                    .map(|v| v.to_display_string())
                    .unwrap_or_else(|| "Panic!".to_string());

                Err(RuntimeError::invalid_operation(
                    format!("Panic: {}", msg),
                    format!("توقف: {}", msg),
                ))
            }

            "sleep" | "نم" => {
                let ms = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "sleep() requires 1 argument (milliseconds)",
                        "نم() تتطلب معامل واحد (ميلي ثانية)",
                    )
                })?;

                let ms = ms
                    .as_int()
                    .ok_or_else(|| RuntimeError::type_error("int", ms.type_name()))?;

                if ms > 0 {
                    std::thread::sleep(std::time::Duration::from_millis(ms as u64));
                }
                Ok(Value::Null)
            }

            "time_now" | "وقت_الآن" => {
                use std::time::{SystemTime, UNIX_EPOCH};
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_millis() as i64)
                    .unwrap_or(0);
                Ok(Value::Int(now))
            }

            "ادخل_رسالة" | "input_prompt" => {
                let prompt = args
                    .first()
                    .map(|v| v.to_display_string())
                    .unwrap_or_default();

                print!("{}", prompt);
                io::stdout().flush().ok();

                let mut input = String::new();
                io::stdin()
                    .read_line(&mut input)
                    .map_err(|e| RuntimeError::internal(format!("Input error: {}", e)))?;

                Ok(Value::string(input.trim_end()))
            }

            "ادخل_عدد" | "input_int" => {
                let mut input = String::new();
                io::stdin()
                    .read_line(&mut input)
                    .map_err(|e| RuntimeError::internal(format!("Input error: {}", e)))?;

                input
                    .trim()
                    .parse::<i64>()
                    .map(Value::Int)
                    .map_err(|_| RuntimeError::type_error("integer input", "invalid input"))
            }

            "ادخل_عشري" | "input_float" => {
                let mut input = String::new();
                io::stdin()
                    .read_line(&mut input)
                    .map_err(|e| RuntimeError::internal(format!("Input error: {}", e)))?;

                input
                    .trim()
                    .parse::<f64>()
                    .map(Value::Float)
                    .map_err(|_| RuntimeError::type_error("float input", "invalid input"))
            }

            "trq_int_to_string" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "trq_int_to_string requires 1 argument",
                        "trq_int_to_string تتطلب معامل واحد",
                    )
                })?;
                match val {
                    Value::Int(n) => Ok(Value::string(n.to_string())),
                    Value::Float(f) => Ok(Value::string((*f as i64).to_string())),
                    _ => Err(RuntimeError::type_error("int", val.type_name())),
                }
            }

            "trq_float_to_string" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "trq_float_to_string requires 1 argument",
                        "trq_float_to_string تتطلب معامل واحد",
                    )
                })?;
                match val {
                    Value::Float(f) => Ok(Value::string(f.to_string())),
                    Value::Int(n) => Ok(Value::string((*n as f64).to_string())),
                    _ => Err(RuntimeError::type_error("float", val.type_name())),
                }
            }

            "trq_bool_to_string" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "trq_bool_to_string requires 1 argument",
                        "trq_bool_to_string تتطلب معامل واحد",
                    )
                })?;
                match val {
                    Value::Bool(b) => Ok(Value::string(if *b {
                        "صحيح".to_string()
                    } else {
                        "خطأ".to_string()
                    })),
                    _ => Err(RuntimeError::type_error("bool", val.type_name())),
                }
            }

            _ => Err(RuntimeError::undefined_function(name)),
        }
    }
}
