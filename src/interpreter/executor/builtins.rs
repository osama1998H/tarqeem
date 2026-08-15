//! Built-in function implementations for the interpreter.
//!
//! This module provides the interpreter's built-in functions including
//! I/O operations, math functions, type conversions, and utility functions.
//!
//! Note: Tarqeem is an Arabic-only programming language.
//! All built-in functions use Arabic names exclusively.

use std::fs;
use std::io::{self, BufRead, Read, Write};

use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use sha2::{Digest, Sha256};

use super::{Interpreter, RuntimeError, RuntimeResult, Value};

/// Convert a Value to a byte (0-255) with range validation.
fn value_to_byte(v: &Value) -> Result<u8, RuntimeError> {
    let i = v
        .as_int()
        .ok_or_else(|| RuntimeError::type_error("عدد", v.type_name()))?;
    if (0..=255).contains(&i) {
        Ok(i as u8)
    } else {
        Err(RuntimeError::invalid_operation(format!(
            "قيمة البايت يجب أن تكون 0-255، حصلنا على {}",
            i
        )))
    }
}

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
                // String functions
                | "نص_يحتوي"
                | "نص_يبدأ_بـ"
                | "نص_ينتهي_بـ"
                // Internal conversion (used by runtime)
                | "عدد_لنص"
                | "عشري_لنص"
                | "منطقي_لنص"
                // Runtime function names (used by IR/codegen)
                | "trq_int_to_string"
                | "trq_float_to_string"
                | "trq_bool_to_string"
                | "trq_assert"
                | "trq_string_len"
                | "trq_string_to_int_checked"
                | "trq_string_to_float_checked"
                // SHA-256 functions
                | "احسب_بصمة"
                | "بصمة_ملف"
                | "بصمة_ثنائي"
                | "طابق_بصمة"
                // Hex encoding functions
                | "إلى_ست_عشري"
                | "من_ست_عشري"
                | "ثنائي_إلى_ست_عشري"
                | "ست_عشري_إلى_ثنائي"
                // GZIP compression functions
                | "اضغط"
                | "فك_الضغط"
                | "اضغط_ثنائي"
                | "فك_ضغط_ثنائي"
                | "اضغط_ملف"
                | "فك_ضغط_ملف"
                // File I/O functions
                | "اقرأ_ملف"
                | "اكتب_ملف"
                | "اقرأ_سطر"
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
                    // Mirrors عدد, which already maps صحيح/خطأ to 1/0.
                    Value::Bool(b) => Ok(Value::Float(if *b { 1.0 } else { 0.0 })),
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

            "مطلق" | "مطلق_عدد" => {
                let val = args
                    .first()
                    .ok_or_else(|| RuntimeError::invalid_operation("مطلق() تتطلب معامل واحد"))?;

                match val {
                    Value::Int(i) => Ok(Value::Int(i.abs())),
                    Value::Float(f) => Ok(Value::Float(f.abs())),
                    _ => Err(RuntimeError::type_error("numeric", val.type_name())),
                }
            }

            "قوة" | "قوة_عدد" => {
                let base = args
                    .first()
                    .ok_or_else(|| RuntimeError::invalid_operation("قوة() تتطلب معاملين"))?;
                let exp = args
                    .get(1)
                    .ok_or_else(|| RuntimeError::invalid_operation("قوة() تتطلب معاملين"))?;

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
                let val = args
                    .first()
                    .ok_or_else(|| RuntimeError::invalid_operation("جذر() تتطلب معامل واحد"))?;

                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(f.sqrt()))
            }

            "جذر_تكعيبي" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation("جذر_تكعيبي() تتطلب معامل واحد")
                })?;

                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(f.cbrt()))
            }

            "لوغاريتم" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation("لوغاريتم() تتطلب معامل واحد")
                })?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(f.ln()))
            }

            "لوغ10" | "لوغاريتم10" => {
                let val = args
                    .first()
                    .ok_or_else(|| RuntimeError::invalid_operation("لوغ10() تتطلب معامل واحد"))?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(f.log10()))
            }

            "لوغ2" => {
                let val = args
                    .first()
                    .ok_or_else(|| RuntimeError::invalid_operation("لوغ2() تتطلب معامل واحد"))?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(f.log2()))
            }

            "أس" | "أسي" => {
                let val = args
                    .first()
                    .ok_or_else(|| RuntimeError::invalid_operation("أس() تتطلب معامل واحد"))?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(f.exp()))
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

            "قرب" | "قرّب" | "تقريب" => {
                let val = args
                    .first()
                    .ok_or_else(|| RuntimeError::invalid_operation("قرب() تتطلب معامل واحد"))?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(f.round()))
            }

            "اقتطع" => {
                let val = args
                    .first()
                    .ok_or_else(|| RuntimeError::invalid_operation("اقتطع() تتطلب معامل واحد"))?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(f.trunc()))
            }

            "أقل" | "أدنى" | "أقل_عدد" => {
                let a = args
                    .first()
                    .ok_or_else(|| RuntimeError::invalid_operation("أقل() تتطلب معاملين"))?;
                let b = args
                    .get(1)
                    .ok_or_else(|| RuntimeError::invalid_operation("أقل() تتطلب معاملين"))?;

                match (a, b) {
                    (Value::Int(x), Value::Int(y)) => Ok(Value::Int(*x.min(y))),
                    (Value::Float(x), Value::Float(y)) => Ok(Value::Float(x.min(*y))),
                    (Value::Int(x), Value::Float(y)) => Ok(Value::Float((*x as f64).min(*y))),
                    (Value::Float(x), Value::Int(y)) => Ok(Value::Float(x.min(*y as f64))),
                    _ => Err(RuntimeError::type_error("numeric", a.type_name())),
                }
            }

            "أكبر" | "أقصى" | "أكبر_عدد" => {
                let a = args
                    .first()
                    .ok_or_else(|| RuntimeError::invalid_operation("أكبر() تتطلب معاملين"))?;
                let b = args
                    .get(1)
                    .ok_or_else(|| RuntimeError::invalid_operation("أكبر() تتطلب معاملين"))?;

                match (a, b) {
                    (Value::Int(x), Value::Int(y)) => Ok(Value::Int(*x.max(y))),
                    (Value::Float(x), Value::Float(y)) => Ok(Value::Float(x.max(*y))),
                    (Value::Int(x), Value::Float(y)) => Ok(Value::Float((*x as f64).max(*y))),
                    (Value::Float(x), Value::Int(y)) => Ok(Value::Float(x.max(*y as f64))),
                    _ => Err(RuntimeError::type_error("numeric", a.type_name())),
                }
            }

            "حصر" | "حصر_عدد" => {
                let val = args
                    .first()
                    .ok_or_else(|| RuntimeError::invalid_operation("حصر() تتطلب ثلاثة معاملات"))?;
                let min_val = args
                    .get(1)
                    .ok_or_else(|| RuntimeError::invalid_operation("حصر() تتطلب ثلاثة معاملات"))?;
                let max_val = args
                    .get(2)
                    .ok_or_else(|| RuntimeError::invalid_operation("حصر() تتطلب ثلاثة معاملات"))?;

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
                let val = args
                    .first()
                    .ok_or_else(|| RuntimeError::invalid_operation("علامة() تتطلب معامل واحد"))?;

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
                let a = args
                    .first()
                    .ok_or_else(|| RuntimeError::invalid_operation("قاسم_مشترك() تتطلب معاملين"))?;
                let b = args
                    .get(1)
                    .ok_or_else(|| RuntimeError::invalid_operation("قاسم_مشترك() تتطلب معاملين"))?;

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
                    RuntimeError::invalid_operation("مضاعف_مشترك() تتطلب معاملين")
                })?;
                let b = args.get(1).ok_or_else(|| {
                    RuntimeError::invalid_operation("مضاعف_مشترك() تتطلب معاملين")
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
                let n = args
                    .first()
                    .ok_or_else(|| RuntimeError::invalid_operation("عاملي() تتطلب معامل واحد"))?;

                let n = n
                    .as_int()
                    .ok_or_else(|| RuntimeError::type_error("int", n.type_name()))?;

                if n < 0 {
                    return Err(RuntimeError::invalid_operation(
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
                let val = args
                    .first()
                    .ok_or_else(|| RuntimeError::invalid_operation("جيب() تتطلب معامل واحد"))?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(f.sin()))
            }

            "جتا" | "جيب_التمام" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation("جيب_التمام() تتطلب معامل واحد")
                })?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(f.cos()))
            }

            "ظا" | "ظل" => {
                let val = args
                    .first()
                    .ok_or_else(|| RuntimeError::invalid_operation("ظل() تتطلب معامل واحد"))?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(f.tan()))
            }

            "ظتا" | "ظل_التمام" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation("ظل_التمام() تتطلب معامل واحد")
                })?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(1.0 / f.tan()))
            }

            "قا" | "قاطع" => {
                let val = args
                    .first()
                    .ok_or_else(|| RuntimeError::invalid_operation("قاطع() تتطلب معامل واحد"))?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(1.0 / f.cos()))
            }

            "قتا" | "قاطع_التمام" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation("قاطع_التمام() تتطلب معامل واحد")
                })?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(1.0 / f.sin()))
            }

            "جا_عكسي" | "جيب_عكسي" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation("جيب_عكسي() تتطلب معامل واحد")
                })?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(f.asin()))
            }

            "جتا_عكسي" | "جيب_تمام_عكسي" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation("جيب_تمام_عكسي() تتطلب معامل واحد")
                })?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(f.acos()))
            }

            "ظا_عكسي" | "ظل_عكسي" => {
                let val = args
                    .first()
                    .ok_or_else(|| RuntimeError::invalid_operation("ظل_عكسي() تتطلب معامل واحد"))?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(f.atan()))
            }

            "ظا_عكسي2" | "ظل_عكسي2" => {
                let y = args
                    .first()
                    .ok_or_else(|| RuntimeError::invalid_operation("ظل_عكسي2() تتطلب معاملين"))?;
                let x = args
                    .get(1)
                    .ok_or_else(|| RuntimeError::invalid_operation("ظل_عكسي2() تتطلب معاملين"))?;

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
                    RuntimeError::invalid_operation("جيب_زائدي() تتطلب معامل واحد")
                })?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(f.sinh()))
            }

            "جتا_زائدي" | "جيب_تمام_زائدي" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation("جيب_تمام_زائدي() تتطلب معامل واحد")
                })?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(f.cosh()))
            }

            "ظا_زائدي" | "ظل_زائدي" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation("ظل_زائدي() تتطلب معامل واحد")
                })?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(f.tanh()))
            }

            "الى_راديان" | "راديان" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation("الى_راديان() تتطلب معامل واحد")
                })?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(f.to_radians()))
            }

            "الى_درجات" | "درجات" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation("الى_درجات() تتطلب معامل واحد")
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
                let min_val = args
                    .first()
                    .ok_or_else(|| RuntimeError::invalid_operation("عشوائي_بين() تتطلب معاملين"))?;
                let max_val = args
                    .get(1)
                    .ok_or_else(|| RuntimeError::invalid_operation("عشوائي_بين() تتطلب معاملين"))?;

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

            // `trq_assert` is the symbol the IR builder lowers both تأكد and
            // تأكد_رسالة to, with a null second argument standing for "no
            // message" — the same contract the native runtime implements.
            "تأكد" | "trq_assert" => {
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

            "trq_string_len" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation("trq_string_len() تتطلب معامل واحد")
                })?;
                match val {
                    Value::String(s) => Ok(Value::Int(s.chars().count() as i64)),
                    _ => Err(RuntimeError::type_error("نص", val.type_name())),
                }
            }

            // The checked parsers back عدد/عدد_عشري on a string. They reject an
            // unparsable value rather than yielding 0, so every backend agrees.
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

            "تأكد_رسالة" => {
                let cond = args
                    .first()
                    .ok_or_else(|| RuntimeError::invalid_operation("تأكد_رسالة() تتطلب معاملين"))?;
                let msg = args
                    .get(1)
                    .ok_or_else(|| RuntimeError::invalid_operation("تأكد_رسالة() تتطلب معاملين"))?;

                if !cond.is_truthy() {
                    let msg_str = msg.to_display_string();
                    return Err(RuntimeError::invalid_operation(format!(
                        "فشل التأكيد: {}",
                        msg_str
                    )));
                }
                Ok(Value::Null)
            }

            "توقف" => {
                let msg = args
                    .first()
                    .map(|v| v.to_display_string())
                    .unwrap_or_else(|| "توقف!".to_string());

                Err(RuntimeError::invalid_operation(format!("توقف: {}", msg)))
            }

            "نم" => {
                let ms = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation("نم() تتطلب معامل واحد (ميلي ثانية)")
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

            "نص_يحتوي" => {
                let haystack = args
                    .first()
                    .ok_or_else(|| RuntimeError::invalid_operation("نص_يحتوي() تتطلب معاملين"))?;
                let needle = args
                    .get(1)
                    .ok_or_else(|| RuntimeError::invalid_operation("نص_يحتوي() تتطلب معاملين"))?;

                match (haystack, needle) {
                    (Value::String(h), Value::String(n)) => Ok(Value::Bool(h.contains(n.as_str()))),
                    _ => Err(RuntimeError::type_error("نص", haystack.type_name())),
                }
            }

            "نص_يبدأ_بـ" => {
                let text = args
                    .first()
                    .ok_or_else(|| RuntimeError::invalid_operation("نص_يبدأ_بـ() تتطلب معاملين"))?;
                let prefix = args
                    .get(1)
                    .ok_or_else(|| RuntimeError::invalid_operation("نص_يبدأ_بـ() تتطلب معاملين"))?;

                match (text, prefix) {
                    (Value::String(t), Value::String(p)) => {
                        Ok(Value::Bool(t.starts_with(p.as_str())))
                    }
                    _ => Err(RuntimeError::type_error("نص", text.type_name())),
                }
            }

            "نص_ينتهي_بـ" => {
                let text = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation("نص_ينتهي_بـ() تتطلب معاملين")
                })?;
                let suffix = args.get(1).ok_or_else(|| {
                    RuntimeError::invalid_operation("نص_ينتهي_بـ() تتطلب معاملين")
                })?;

                match (text, suffix) {
                    (Value::String(t), Value::String(s)) => {
                        Ok(Value::Bool(t.ends_with(s.as_str())))
                    }
                    _ => Err(RuntimeError::type_error("نص", text.type_name())),
                }
            }

            "عدد_لنص" | "trq_int_to_string" => {
                let val = args
                    .first()
                    .ok_or_else(|| RuntimeError::invalid_operation("عدد_لنص() تتطلب معامل واحد"))?;
                match val {
                    Value::Int(n) => Ok(Value::string(n.to_string())),
                    Value::Float(f) => Ok(Value::string((*f as i64).to_string())),
                    _ => Err(RuntimeError::type_error("عدد", val.type_name())),
                }
            }

            "عشري_لنص" | "trq_float_to_string" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation("عشري_لنص() تتطلب معامل واحد")
                })?;
                match val {
                    Value::Float(f) => Ok(Value::string(f.to_string())),
                    Value::Int(n) => Ok(Value::string((*n as f64).to_string())),
                    _ => Err(RuntimeError::type_error("عدد_عشري", val.type_name())),
                }
            }

            "منطقي_لنص" | "trq_bool_to_string" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation("منطقي_لنص() تتطلب معامل واحد")
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

            // ============================================================
            // SHA-256 Functions (البصمة الرقمية)
            // ============================================================
            "احسب_بصمة" => {
                // SHA256 hash of a string
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation("احسب_بصمة() تتطلب معامل واحد")
                })?;

                let text = match val {
                    Value::String(s) => s.clone(),
                    _ => return Err(RuntimeError::type_error("نص", val.type_name())),
                };

                let mut hasher = Sha256::new();
                hasher.update(text.as_bytes());
                let result = hasher.finalize();
                Ok(Value::string(hex::encode(result)))
            }

            "بصمة_ملف" => {
                // SHA256 hash of a file
                let path_val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation("بصمة_ملف() تتطلب معامل واحد (مسار الملف)")
                })?;

                let path = match path_val {
                    Value::String(s) => s.clone(),
                    _ => return Err(RuntimeError::type_error("نص", path_val.type_name())),
                };

                let content = std::fs::read(path.as_str()).map_err(|e| {
                    RuntimeError::invalid_operation(format!("فشل قراءة الملف '{}': {}", path, e))
                })?;

                let mut hasher = Sha256::new();
                hasher.update(&content);
                let result = hasher.finalize();
                Ok(Value::string(hex::encode(result)))
            }

            "بصمة_ثنائي" => {
                // SHA256 hash of byte array
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation("بصمة_ثنائي() تتطلب معامل واحد (مصفوفة بايتات)")
                })?;

                let bytes: Vec<u8> = match val {
                    Value::Array(arr) => {
                        let arr = arr.borrow();
                        arr.iter()
                            .map(value_to_byte)
                            .collect::<Result<Vec<_>, _>>()?
                    }
                    _ => return Err(RuntimeError::type_error("مصفوفة", val.type_name())),
                };

                let mut hasher = Sha256::new();
                hasher.update(&bytes);
                let result = hasher.finalize();
                Ok(Value::string(hex::encode(result)))
            }

            "طابق_بصمة" => {
                // Compare two SHA256 hashes (constant-time comparison)
                let hash1 = args
                    .first()
                    .ok_or_else(|| RuntimeError::invalid_operation("طابق_بصمة() تتطلب معاملين"))?;
                let hash2 = args
                    .get(1)
                    .ok_or_else(|| RuntimeError::invalid_operation("طابق_بصمة() تتطلب معاملين"))?;

                let h1 = match hash1 {
                    Value::String(s) => s.clone(),
                    _ => return Err(RuntimeError::type_error("نص", hash1.type_name())),
                };
                let h2 = match hash2 {
                    Value::String(s) => s.clone(),
                    _ => return Err(RuntimeError::type_error("نص", hash2.type_name())),
                };

                // Constant-time comparison to prevent timing attacks
                let result = h1.len() == h2.len()
                    && h1
                        .bytes()
                        .zip(h2.bytes())
                        .fold(0u8, |acc, (a, b)| acc | (a ^ b))
                        == 0;

                Ok(Value::Bool(result))
            }

            // ============================================================
            // Hex Encoding Functions (الترميز الست عشري)
            // ============================================================
            "إلى_ست_عشري" => {
                // Hex encode a string
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation("إلى_ست_عشري() تتطلب معامل واحد")
                })?;

                let text = match val {
                    Value::String(s) => s.clone(),
                    _ => return Err(RuntimeError::type_error("نص", val.type_name())),
                };

                Ok(Value::string(hex::encode(text.as_bytes())))
            }

            "من_ست_عشري" => {
                // Hex decode to string
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation("من_ست_عشري() تتطلب معامل واحد")
                })?;

                let hex_str = match val {
                    Value::String(s) => s.clone(),
                    _ => return Err(RuntimeError::type_error("نص", val.type_name())),
                };

                let bytes = hex::decode(hex_str.as_bytes()).map_err(|e| {
                    RuntimeError::invalid_operation(format!("نص ست عشري غير صالح: {}", e))
                })?;

                let text = String::from_utf8(bytes).map_err(|e| {
                    RuntimeError::invalid_operation(format!(
                        "UTF-8 غير صالح في البيانات المفككة: {}",
                        e
                    ))
                })?;

                Ok(Value::string(text))
            }

            "ثنائي_إلى_ست_عشري" => {
                // Hex encode byte array
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation("ثنائي_إلى_ست_عشري() تتطلب معامل واحد")
                })?;

                let bytes: Vec<u8> = match val {
                    Value::Array(arr) => {
                        let arr = arr.borrow();
                        arr.iter()
                            .map(value_to_byte)
                            .collect::<Result<Vec<_>, _>>()?
                    }
                    _ => return Err(RuntimeError::type_error("مصفوفة", val.type_name())),
                };

                Ok(Value::string(hex::encode(&bytes)))
            }

            "ست_عشري_إلى_ثنائي" => {
                // Hex decode to byte array
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation("ست_عشري_إلى_ثنائي() تتطلب معامل واحد")
                })?;

                let hex_str = match val {
                    Value::String(s) => s.clone(),
                    _ => return Err(RuntimeError::type_error("نص", val.type_name())),
                };

                let bytes = hex::decode(hex_str.as_bytes()).map_err(|e| {
                    RuntimeError::invalid_operation(format!("نص ست عشري غير صالح: {}", e))
                })?;

                let values: Vec<Value> = bytes.into_iter().map(|b| Value::Int(b as i64)).collect();
                Ok(Value::array_from(values))
            }

            // ============================================================
            // GZIP Compression Functions (الضغط)
            // ============================================================
            "اضغط" => {
                // GZIP compress a string, returns byte array
                let val = args
                    .first()
                    .ok_or_else(|| RuntimeError::invalid_operation("اضغط() تتطلب معامل واحد"))?;

                let text = match val {
                    Value::String(s) => s.clone(),
                    _ => return Err(RuntimeError::type_error("نص", val.type_name())),
                };

                let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
                encoder
                    .write_all(text.as_bytes())
                    .map_err(|e| RuntimeError::invalid_operation(format!("فشل الضغط: {}", e)))?;
                let compressed = encoder
                    .finish()
                    .map_err(|e| RuntimeError::invalid_operation(format!("فشل الضغط: {}", e)))?;

                let values: Vec<Value> = compressed
                    .into_iter()
                    .map(|b| Value::Int(b as i64))
                    .collect();
                Ok(Value::array_from(values))
            }

            "فك_الضغط" => {
                // GZIP decompress byte array to string
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation("فك_الضغط() تتطلب معامل واحد")
                })?;

                let bytes: Vec<u8> = match val {
                    Value::Array(arr) => {
                        let arr = arr.borrow();
                        arr.iter()
                            .map(value_to_byte)
                            .collect::<Result<Vec<_>, _>>()?
                    }
                    _ => return Err(RuntimeError::type_error("مصفوفة", val.type_name())),
                };

                let mut decoder = GzDecoder::new(&bytes[..]);
                let mut decompressed = String::new();
                decoder
                    .read_to_string(&mut decompressed)
                    .map_err(|e| RuntimeError::invalid_operation(format!("فشل فك الضغط: {}", e)))?;

                Ok(Value::string(decompressed))
            }

            "اضغط_ثنائي" => {
                // GZIP compress byte array, returns byte array
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation("اضغط_ثنائي() تتطلب معامل واحد")
                })?;

                let bytes: Vec<u8> = match val {
                    Value::Array(arr) => {
                        let arr = arr.borrow();
                        arr.iter()
                            .map(value_to_byte)
                            .collect::<Result<Vec<_>, _>>()?
                    }
                    _ => return Err(RuntimeError::type_error("مصفوفة", val.type_name())),
                };

                let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
                encoder
                    .write_all(&bytes)
                    .map_err(|e| RuntimeError::invalid_operation(format!("فشل الضغط: {}", e)))?;
                let compressed = encoder
                    .finish()
                    .map_err(|e| RuntimeError::invalid_operation(format!("فشل الضغط: {}", e)))?;

                let values: Vec<Value> = compressed
                    .into_iter()
                    .map(|b| Value::Int(b as i64))
                    .collect();
                Ok(Value::array_from(values))
            }

            "فك_ضغط_ثنائي" => {
                // GZIP decompress byte array to byte array
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation("فك_ضغط_ثنائي() تتطلب معامل واحد")
                })?;

                let bytes: Vec<u8> = match val {
                    Value::Array(arr) => {
                        let arr = arr.borrow();
                        arr.iter()
                            .map(value_to_byte)
                            .collect::<Result<Vec<_>, _>>()?
                    }
                    _ => return Err(RuntimeError::type_error("مصفوفة", val.type_name())),
                };

                let mut decoder = GzDecoder::new(&bytes[..]);
                let mut decompressed = Vec::new();
                decoder
                    .read_to_end(&mut decompressed)
                    .map_err(|e| RuntimeError::invalid_operation(format!("فشل فك الضغط: {}", e)))?;

                let values: Vec<Value> = decompressed
                    .into_iter()
                    .map(|b| Value::Int(b as i64))
                    .collect();
                Ok(Value::array_from(values))
            }

            "اضغط_ملف" => {
                // GZIP compress a file
                let input_path = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation("اضغط_ملف() تتطلب معاملين (المدخل، المخرج)")
                })?;
                let output_path = args.get(1).ok_or_else(|| {
                    RuntimeError::invalid_operation("اضغط_ملف() تتطلب معاملين (المدخل، المخرج)")
                })?;

                let input = match input_path {
                    Value::String(s) => s.clone(),
                    _ => return Err(RuntimeError::type_error("نص", input_path.type_name())),
                };
                let output = match output_path {
                    Value::String(s) => s.clone(),
                    _ => return Err(RuntimeError::type_error("نص", output_path.type_name())),
                };

                let content = std::fs::read(input.as_str()).map_err(|e| {
                    RuntimeError::invalid_operation(format!("فشل قراءة الملف '{}': {}", input, e))
                })?;

                let output_file = std::fs::File::create(output.as_str()).map_err(|e| {
                    RuntimeError::invalid_operation(format!("فشل إنشاء الملف '{}': {}", output, e))
                })?;

                let mut encoder = GzEncoder::new(output_file, Compression::default());
                encoder
                    .write_all(&content)
                    .map_err(|e| RuntimeError::invalid_operation(format!("فشل الضغط: {}", e)))?;
                encoder
                    .finish()
                    .map_err(|e| RuntimeError::invalid_operation(format!("فشل الضغط: {}", e)))?;

                Ok(Value::Bool(true))
            }

            "فك_ضغط_ملف" => {
                // GZIP decompress a file
                let input_path = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation("فك_ضغط_ملف() تتطلب معاملين (المدخل، المخرج)")
                })?;
                let output_path = args.get(1).ok_or_else(|| {
                    RuntimeError::invalid_operation("فك_ضغط_ملف() تتطلب معاملين (المدخل، المخرج)")
                })?;

                let input = match input_path {
                    Value::String(s) => s.clone(),
                    _ => return Err(RuntimeError::type_error("نص", input_path.type_name())),
                };
                let output = match output_path {
                    Value::String(s) => s.clone(),
                    _ => return Err(RuntimeError::type_error("نص", output_path.type_name())),
                };

                let compressed = std::fs::read(input.as_str()).map_err(|e| {
                    RuntimeError::invalid_operation(format!("فشل قراءة الملف '{}': {}", input, e))
                })?;

                let mut decoder = GzDecoder::new(&compressed[..]);
                let mut decompressed = Vec::new();
                decoder
                    .read_to_end(&mut decompressed)
                    .map_err(|e| RuntimeError::invalid_operation(format!("فشل فك الضغط: {}", e)))?;

                std::fs::write(output.as_str(), &decompressed).map_err(|e| {
                    RuntimeError::invalid_operation(format!("فشل كتابة الملف '{}': {}", output, e))
                })?;

                Ok(Value::Bool(true))
            }

            "اقرأ_ملف" => {
                // Read file contents as string
                let path = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation("اقرأ_ملف() تتطلب معامل واحد (مسار الملف)")
                })?;

                let path_str = match path {
                    Value::String(s) => s.clone(),
                    _ => return Err(RuntimeError::type_error("نص", path.type_name())),
                };

                let content = fs::read_to_string(path_str.as_str()).map_err(|e| {
                    RuntimeError::invalid_operation(format!(
                        "فشل قراءة الملف '{}': {}",
                        path_str, e
                    ))
                })?;

                Ok(Value::string(content))
            }

            "اكتب_ملف" => {
                // Write string to file
                let path = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "اكتب_ملف() تتطلب معاملين (مسار الملف، المحتوى)",
                    )
                })?;
                let content = args.get(1).ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "اكتب_ملف() تتطلب معاملين (مسار الملف، المحتوى)",
                    )
                })?;

                let path_str = match path {
                    Value::String(s) => s.clone(),
                    _ => return Err(RuntimeError::type_error("نص", path.type_name())),
                };
                let content_str = match content {
                    Value::String(s) => s.clone(),
                    _ => return Err(RuntimeError::type_error("نص", content.type_name())),
                };

                fs::write(path_str.as_str(), content_str.as_str()).map_err(|e| {
                    RuntimeError::invalid_operation(format!(
                        "فشل كتابة الملف '{}': {}",
                        path_str, e
                    ))
                })?;

                Ok(Value::Bool(true))
            }

            "اقرأ_سطر" => {
                // Read line from stdin
                let stdin = io::stdin();
                let mut line = String::new();
                stdin.lock().read_line(&mut line).map_err(|e| {
                    RuntimeError::invalid_operation(format!("فشل قراءة السطر: {}", e))
                })?;

                // Remove trailing newline
                if line.ends_with('\n') {
                    line.pop();
                    if line.ends_with('\r') {
                        line.pop();
                    }
                }

                Ok(Value::string(line))
            }

            _ => Err(RuntimeError::undefined_function(name)),
        }
    }
}
