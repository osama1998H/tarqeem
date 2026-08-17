//! The implicit prelude: declarations every program has without importing them.
//!
//! Today that is the base exception class `استثناء` (LANGUAGE_SPEC §11.1).
//! `ارمِ` accepts only `استثناء` or a subclass — `Analyzer::is_error_type` —
//! so until this existed *nothing* was throwable and the whole of §11 was dead
//! code (issue #181). It could not simply be added to `stdlib/أخطاء`: that
//! file declares the hierarchy under the name `خطأ`, which is the boolean-false
//! keyword and cannot be parsed as a class name.
//!
//! The prelude is injected as a synthetic entry in the module cache rather than
//! registered by hand, because `ClassResolver` alone is not enough: `جديد
//! استثناء(…)` needs an object layout and a constructor body in the `Ast` that
//! `IrBuilder::build` consumes. Going through the cache reuses the whole
//! module pipeline built for issue #182 — `register_module_types` and
//! `add_module_type_members` register the class ahead of `build_vtables`, and
//! `link_program` merges the declaration into the program AST — so the
//! interpreter, the JIT and native codegen all get it from one insertion.
//!
//! The source is embedded rather than read from `stdlib/`: the LSP and DAP
//! have no stdlib search path (issue #230), and a prelude that can go missing
//! at run time would take `ارمِ` down with it.

use std::path::PathBuf;

use crate::parser::{Ast, Parser};

/// Stands in for a file path in the module cache. Deliberately not a valid
/// filename, so it can never collide with a real module and reads as
/// non-user-code wherever a module path is displayed.
pub(crate) const PRELUDE_PATH: &str = "<تمهيد ترقيم>";

/// The base exception class name, as `is_error_type` and the catch-parameter
/// type both spell it. Re-exported from `crate::semantic` because the IR
/// builder needs the same name to type a catch parameter.
pub const EXCEPTION_CLASS: &str = "استثناء";

/// The field holding the reason an exception was thrown. The interpreter reads
/// it to report an uncaught exception, so it must stay in step with
/// `PRELUDE_SOURCE` — hence a shared constant rather than a literal at each use.
pub const EXCEPTION_MESSAGE_FIELD: &str = "رسالة";

/// Single constructor: `ClassInfo::constructor` is one `Option<MethodInfo>`, so
/// the two-constructor form in LANGUAGE_SPEC §11.1 is not expressible until
/// Tarqeem has overloading. `رسالة_عربية` is left out for the same reason —
/// with one constructor it could only ever duplicate `رسالة`.
const PRELUDE_SOURCE: &str = r#"بسم_الله

/// صنف الاستثناء الأساسي: أصل كل ما يصح رميه بـ «ارمِ».
///
/// يورّثه المستخدم ليعرّف أنواع استثناءات خاصة به:
///     صنف استثناء_قسمة يرث استثناء {}
صنف استثناء {
    /// نص يوصف سبب الاستثناء، ويُقرأ في «التقط».
    عام رسالة: نص

    منشئ(رسالة: نص) {
        هذا.رسالة = رسالة
    }
}

الحمد_لله
"#;

/// Parse the prelude. `Err` is unreachable for a correct `PRELUDE_SOURCE` and
/// is covered by `test_prelude_source_parses`; callers skip injection rather
/// than fail the user's compile over a compiler-internal defect.
pub(crate) fn prelude_ast() -> Result<(PathBuf, String, Ast), String> {
    let source = PRELUDE_SOURCE.to_string();
    let ast = Parser::new(&source).parse().map_err(|e| e.message)?;
    Ok((PathBuf::from(PRELUDE_PATH), source, ast))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::StmtKind;

    #[test]
    fn test_prelude_source_parses() {
        let (path, _, ast) = prelude_ast().expect("يجب أن يُحلَّل مصدر التمهيد");
        assert_eq!(path, PathBuf::from(PRELUDE_PATH));
        assert!(!ast.statements.is_empty());
    }

    #[test]
    fn test_prelude_declares_exception_class_with_message_field() {
        let (_, _, ast) = prelude_ast().expect("يجب أن يُحلَّل مصدر التمهيد");

        let found = ast.statements.iter().any(|stmt| {
            matches!(&stmt.kind, StmtKind::ClassDecl { name, .. } if name == EXCEPTION_CLASS)
        });

        assert!(found, "التمهيد يجب أن يعرّف الصنف '{}'", EXCEPTION_CLASS);
    }

    /// The prelude must not contain executable top-level code: `link_program`
    /// drops it and warns, which would fire on every single compile.
    #[test]
    fn test_prelude_has_only_declarations() {
        let (_, _, ast) = prelude_ast().expect("يجب أن يُحلَّل مصدر التمهيد");

        for stmt in &ast.statements {
            assert!(
                matches!(
                    &stmt.kind,
                    StmtKind::ClassDecl { .. }
                        | StmtKind::InterfaceDecl { .. }
                        | StmtKind::FuncDecl { .. }
                        | StmtKind::EnumDecl { .. }
                ),
                "جملة تنفيذية في التمهيد: {:?}",
                stmt.kind
            );
        }
    }
}
