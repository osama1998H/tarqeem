//! Built-in function implementations for the interpreter.
//!
//! This module provides the interpreter's built-in functions including
//! I/O operations, math functions, type conversions, and utility functions.
//!
//! Note: Tarqeem is an Arabic-only programming language.
//! All built-in functions use Arabic names exclusively.

use std::io::{self, Write};

use super::{Interpreter, RuntimeError, RuntimeResult, Value};

impl Interpreter {
    pub(crate) fn is_builtin(&self, name: &str) -> bool {
        matches!(
            name,
            // I/O functions
            "اطبع"
                | "طباعة"
                | "اطبع_سطر"
                | "اطبع_خطأ"
                | "ادخل"
                | "ادخل_رسالة"
                | "ادخل_عدد"
                | "ادخل_عشري"
                // Type functions
                | "طول"
                | "نوع"
                | "عدد"
                | "عدد_عشري"
                | "نص"
                | "منطقي"
                // Math - basic
                | "مطلق"
                | "مطلق_عدد"
                | "قوة"
                | "قوة_عدد"
                | "جذر"
                | "جذر_تكعيبي"
                // Math - logarithms
                | "لوغاريتم"
                | "لوغ10"
                | "لوغاريتم10"
                | "لوغ2"
                | "أس"
                | "أسي"
                // Math - rounding
                | "أرضية"
                | "سقف"
                | "قرب"
                | "قرّب"
                | "تقريب"
                | "اقتطع"
                // Math - comparison
                | "أقل"
                | "أدنى"
                | "أقل_عدد"
                | "أكبر"
                | "أقصى"
                | "أكبر_عدد"
                | "حصر"
                | "حصر_عدد"
                | "علامة"
                // Math - number theory
                | "قاسم_مشترك"
                | "مضاعف_مشترك"
                | "عاملي"
                | "باقي"
                // Trigonometry
                | "جا"
                | "جيب"
                | "جتا"
                | "جيب_التمام"
                | "ظا"
                | "ظل"
                | "ظتا"
                | "ظل_التمام"
                | "قا"
                | "قاطع"
                | "قتا"
                | "قاطع_التمام"
                // Inverse trigonometry
                | "جا_عكسي"
                | "جيب_عكسي"
                | "جتا_عكسي"
                | "جيب_تمام_عكسي"
                | "ظا_عكسي"
                | "ظل_عكسي"
                | "ظا_عكسي2"
                | "ظل_عكسي2"
                // Hyperbolic
                | "جا_زائدي"
                | "جيب_زائدي"
                | "جتا_زائدي"
                | "جيب_تمام_زائدي"
                | "ظا_زائدي"
                | "ظل_زائدي"
                // Angle conversion
                | "الى_راديان"
                | "راديان"
                | "الى_درجات"
                | "درجات"
                // Random
                | "عشوائي"
                | "عشوائي_عدد"
                | "عشوائي_بين"
                | "عشوائي_عدد_بين"
                | "عشوائي_عشري"
                | "عشوائي_عشري_بين"
                | "عشوائي_منطقي"
                | "بذرة_عشوائية"
                | "بذرة_عشوائي"
                // Assertions and control
                | "تأكد"
                | "تأكد_رسالة"
                | "توقف"
                | "نم"
                | "وقت_الآن"
                | "وقت_أداء"
                // Internal conversion (used by runtime)
                | "عدد_لنص"
                | "عشري_لنص"
                | "منطقي_لنص"
                // Runtime function names (used by IR/codegen)
                | "trq_int_to_string"
                | "trq_float_to_string"
                | "trq_bool_to_string"
        )
    }

    pub(crate) fn call_builtin(&mut self, name: &str, args: Vec<Value>) -> RuntimeResult<Value> {
        match name {
            "اطبع" | "طباعة" | "اطبع_سطر" | "اطبع_خطأ" => {
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

            "نوع" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "type() requires 1 argument",
                        "نوع() تتطلب معامل واحد",
                    )
                })?;
                Ok(Value::string(val.type_name_ar()))
            }

            "عدد" => {
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

            "عدد_عشري" => {
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

            "نص" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "str() requires 1 argument",
                        "نص() تتطلب معامل واحد",
                    )
                })?;
                Ok(Value::string(val.to_display_string()))
            }

            "منطقي" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "bool() requires 1 argument",
                        "منطقي() تتطلب معامل واحد",
                    )
                })?;
                Ok(Value::Bool(val.is_truthy()))
            }

            "مطلق" | "مطلق_عدد" => {
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

            "قوة" | "قوة_عدد" => {
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

            "جذر" => {
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

            "جذر_تكعيبي" => {
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

            "لوغاريتم" => {
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

            "لوغ10" | "لوغاريتم10" => {
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

            "لوغ2" => {
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

            "أس" | "أسي" => {
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

            "أرضية" => {
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

            "سقف" => {
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

            "قرب" | "قرّب" | "تقريب" => {
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

            "اقتطع" => {
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

            "أقل" | "أدنى" | "أقل_عدد" => {
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

            "أكبر" | "أقصى" | "أكبر_عدد" => {
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

            "حصر" | "حصر_عدد" => {
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

            "علامة" => {
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

            "قاسم_مشترك" => {
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

            "مضاعف_مشترك" => {
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

            "عاملي" => {
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

            "جا" | "جيب" => {
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

            "جتا" | "جيب_التمام" => {
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

            "ظا" | "ظل" => {
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

            "ظتا" | "ظل_التمام" => {
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

            "قا" | "قاطع" => {
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

            "قتا" | "قاطع_التمام" => {
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

            "جا_عكسي" | "جيب_عكسي" => {
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

            "جتا_عكسي" | "جيب_تمام_عكسي" => {
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

            "ظا_عكسي" | "ظل_عكسي" => {
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

            "ظا_عكسي2" | "ظل_عكسي2" => {
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

            "جا_زائدي" | "جيب_زائدي" => {
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

            "جتا_زائدي" | "جيب_تمام_زائدي" => {
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

            "ظا_زائدي" | "ظل_زائدي" => {
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

            "الى_راديان" | "راديان" => {
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

            "الى_درجات" | "درجات" => {
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

            "عشوائي" | "عشوائي_عدد" => {
                use std::time::{SystemTime, UNIX_EPOCH};
                let seed = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_nanos() as u64)
                    .unwrap_or(12345);
                let random = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                Ok(Value::Int((random % (i64::MAX as u64 + 1)) as i64))
            }

            "عشوائي_بين" | "عشوائي_عدد_بين" => {
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

            "عشوائي_عشري" | "عشوائي_عشري_بين" => {
                use std::time::{SystemTime, UNIX_EPOCH};
                let seed = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_nanos() as u64)
                    .unwrap_or(12345);
                let random = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                let result = (random as f64) / (u64::MAX as f64);
                Ok(Value::Float(result))
            }

            "عشوائي_منطقي" => {
                use std::time::{SystemTime, UNIX_EPOCH};
                let seed = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_nanos() as u64)
                    .unwrap_or(12345);
                let random = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                Ok(Value::Bool(random.is_multiple_of(2)))
            }

            "تأكد" => {
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

            "تأكد_رسالة" => {
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

            "توقف" => {
                let msg = args
                    .first()
                    .map(|v| v.to_display_string())
                    .unwrap_or_else(|| "Panic!".to_string());

                Err(RuntimeError::invalid_operation(
                    format!("Panic: {}", msg),
                    format!("توقف: {}", msg),
                ))
            }

            "نم" => {
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

            "وقت_الآن" | "وقت_أداء" => {
                use std::time::{SystemTime, UNIX_EPOCH};
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_millis() as i64)
                    .unwrap_or(0);
                Ok(Value::Int(now))
            }

            "ادخل_رسالة" => {
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

            "ادخل_عدد" => {
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

            "ادخل_عشري" => {
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

            "عدد_لنص" | "trq_int_to_string" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "عدد_لنص() requires 1 argument",
                        "عدد_لنص() تتطلب معامل واحد",
                    )
                })?;
                match val {
                    Value::Int(n) => Ok(Value::string(n.to_string())),
                    Value::Float(f) => Ok(Value::string((*f as i64).to_string())),
                    _ => Err(RuntimeError::type_error("عدد", val.type_name())),
                }
            }

            "عشري_لنص" | "trq_float_to_string" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "عشري_لنص() requires 1 argument",
                        "عشري_لنص() تتطلب معامل واحد",
                    )
                })?;
                match val {
                    Value::Float(f) => Ok(Value::string(f.to_string())),
                    Value::Int(n) => Ok(Value::string((*n as f64).to_string())),
                    _ => Err(RuntimeError::type_error("عدد_عشري", val.type_name())),
                }
            }

            "منطقي_لنص" | "trq_bool_to_string" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "منطقي_لنص() requires 1 argument",
                        "منطقي_لنص() تتطلب معامل واحد",
                    )
                })?;
                match val {
                    Value::Bool(b) => Ok(Value::string(if *b {
                        "صحيح".to_string()
                    } else {
                        "خطأ".to_string()
                    })),
                    _ => Err(RuntimeError::type_error("منطقي", val.type_name())),
                }
            }

            _ => Err(RuntimeError::undefined_function(name)),
        }
    }
}
