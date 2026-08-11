//! Merging of imported module ASTs into the main program AST.
//!
//! `IrBuilder::build` consumes exactly one `Ast`, and `build_stmt` returns `Ok`
//! for `Import` without emitting anything — so before this pass an imported
//! function was in the symbol table (`check` passed) but had no body in any
//! backend, and issue #182 surfaced at run time as `دالة غير معرّفة`.
//!
//! Merging here, ahead of IR, repairs the interpreter, the JIT and native
//! codegen at once, because all three consume the same IR `Module`. It also
//! leaves `IrBuilder::build`'s signature — and its ~20 non-pipeline callers —
//! untouched.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::error::codes::{ERR_DUPLICATE_EXPORT, ERR_REDEFINE_PRELUDE_CLASS};
use crate::error::{Diagnostic, Span};
use crate::parser::{Ast, ExportItems, Stmt, StmtKind};

use super::modules::ModuleLoader;
use super::prelude::PRELUDE_PATH;
use super::scope::normalize_name;

/// Stands in for the main file's path in diagnostics when the caller had none
/// (the REPL and in-memory callers analyze source with no file behind it).
const MAIN_FILE_LABEL: &str = "<الملف الرئيسي>";

/// The name `IrBuilder::build` (`src/ir/builder/mod.rs`) reads as the
/// Program-mode entry point. Matched byte-for-byte the way it is there — the
/// merge must drop exactly what would flip `has_user_main`, and nothing else.
/// The two cannot share a constant: `ir` sits after `semantic` in the pipeline.
const ENTRY_POINT_NAME: &str = "رئيسية";

/// What the merge does with one top-level statement of an imported module.
enum Disposition<'a> {
    /// Kept, already unwrapped out of any `صدّر`.
    Carry(&'a Stmt),
    /// Module metadata that emits no IR either way.
    Drop,
    /// Executable code at a module's top level. Dropped, and warned about.
    DropExecutable,
    /// A module's own `دالة رئيسية()`. Dropped, and warned about.
    DropEntryPoint,
}

/// Build a single `Ast` containing every loaded module's declarations followed
/// by `main`'s statements.
///
/// `warnings` collects non-fatal diagnostics (dropped module-level executable
/// code); they cannot travel in the `Result`, whose `Err` arm is reserved for
/// fatal name collisions. Callers emit them and continue.
///
/// `main_path` lets the merge recognize main's own entry in the module cache. A
/// module that imports the main file back puts main there, and merging it would
/// duplicate every main declaration into a bogus collision.
pub fn link_program(
    main: &Ast,
    loader: &ModuleLoader,
    main_path: Option<&Path>,
    warnings: &mut Vec<Diagnostic>,
) -> Result<Ast, Vec<Diagnostic>> {
    // Single-file programs — and programs importing only stdlib, which the
    // loader never reads from disk — must pay one clone and nothing else.
    if loader.modules_in_load_order().next().is_none() {
        return Ok(main.clone());
    }

    let main_canonical = main_path.and_then(|path| path.canonicalize().ok());
    let main_display = main_path.unwrap_or_else(|| Path::new(MAIN_FILE_LABEL));
    let import_spans = main_import_spans(main, loader, main_path);

    let mut merged: Vec<Stmt> = Vec::new();
    let mut errors: Vec<Diagnostic> = Vec::new();
    let mut origins: HashMap<String, PathBuf> = HashMap::new();

    // Dependency-first order, so a module constant is declared — and, via
    // `globals_needing_init`, initialized inside `__global_init__` — before any
    // later module or main global that reads it.
    for module in loader.modules_in_load_order() {
        if main_canonical.as_deref() == Some(module.path.as_path()) {
            continue;
        }

        let collision_span = import_spans
            .get(&module.path)
            .copied()
            .unwrap_or_else(Span::default);

        let mut dropped_executables = 0usize;
        let mut dropped_entry_point = false;

        for stmt in &module.ast.statements {
            match disposition(stmt) {
                Disposition::Carry(carried) => {
                    if let Some(name) = declared_name(&carried.kind) {
                        record_origin(
                            &mut origins,
                            &mut errors,
                            name,
                            &module.path,
                            collision_span,
                        );
                    }
                    merged.push(carried.clone());
                }
                Disposition::Drop => {}
                Disposition::DropExecutable => dropped_executables += 1,
                Disposition::DropEntryPoint => dropped_entry_point = true,
            }
        }

        if dropped_entry_point {
            warnings.push(Diagnostic::warning(
                format!(
                    "تم تجاهل دالة 'رئيسية' الخاصة بالوحدة '{}'؛ نقطة دخول الوحدة \
                     لا معنى لها بعد الدمج، ونقطة دخول البرنامج هي نقطة دخول الملف \
                     الرئيسي وحده. / \
                     Ignored module '{}'s own 'رئيسية'; a module's entry point is \
                     meaningless once merged — the program's entry point is the \
                     main file's alone.",
                    module.path.display(),
                    module.path.display()
                ),
                collision_span,
            ));
        }

        if dropped_executables > 0 {
            warnings.push(Diagnostic::warning(
                format!(
                    "تم تجاهل {} جملة تنفيذية في المستوى الأعلى للوحدة '{}'؛ \
                     الوحدات المستوردة تُقرأ للتعريفات فقط ولا تُنفَّذ. / \
                     Ignored {} top-level executable statement(s) in module '{}'; \
                     imported modules contribute declarations only and are not executed.",
                    dropped_executables,
                    module.path.display(),
                    dropped_executables,
                    module.path.display()
                ),
                collision_span,
            ));
        }
    }

    // Main's statements are carried verbatim — `as_top_level_decl` in the IR
    // builder already sees through `صدّر`. Only name collection unwraps, so a
    // `صدّر دالة جمع` in main still collides with a module's `جمع`.
    for stmt in &main.statements {
        if let Some(name) = declared_name(&unwrap_exported_decl(stmt).kind) {
            record_origin(&mut origins, &mut errors, name, main_display, stmt.span);
        }
    }
    merged.extend(main.statements.iter().cloned());

    if !errors.is_empty() {
        return Err(errors);
    }

    Ok(Ast {
        statements: merged,
        bismillah_span: main.bismillah_span,
        alhamdulillah_span: main.alhamdulillah_span,
        // Main's file doc describes the linked program; an imported module's
        // describes that module, which no longer exists as a unit after the
        // merge — so those drop, exactly like their markers' spans.
        module_doc: main.module_doc.clone(),
    })
}

/// Map each imported module's canonical path to the `استورد` span in main that
/// pulled it in.
///
/// `Span` carries no file identity and `Diagnostic::emit` renders every span
/// against the main file's source, so a diagnostic about a module may only ever
/// be anchored to a main-file span — never to a span from the module itself.
fn main_import_spans(
    main: &Ast,
    loader: &ModuleLoader,
    main_path: Option<&Path>,
) -> HashMap<PathBuf, Span> {
    let mut spans = HashMap::new();

    let Some(from_path) = main_path else {
        return spans;
    };

    for stmt in &main.statements {
        if let StmtKind::Import { from, .. } = &stmt.kind {
            if let Some(target) = loader.resolve_path(from_path, from) {
                spans.entry(target).or_insert(stmt.span);
            }
        }
    }

    spans
}

fn disposition(stmt: &Stmt) -> Disposition<'_> {
    // Unwrapped first so the merged list is uniform bare declarations — every
    // IR pass then sees the same shape regardless of which file a declaration
    // came from — and so that `صدّر دالة رئيسية()` is judged by what it
    // declares, not by the `صدّر` around it.
    let decl = unwrap_exported_decl(stmt);

    match &decl.kind {
        // A library that also runs standalone declares its own entry point.
        // Carrying it would flip `has_user_main` for the merged AST and make
        // the IR builder reject a perfectly valid script-mode main with ت٠٢٠١,
        // naming two constructs that are not even in the same file.
        StmtKind::FuncDecl { name, .. } if name == ENTRY_POINT_NAME => Disposition::DropEntryPoint,

        StmtKind::FuncDecl { .. }
        | StmtKind::ClassDecl { .. }
        | StmtKind::InterfaceDecl { .. }
        | StmtKind::EnumDecl { .. }
        | StmtKind::VarDecl { .. } => Disposition::Carry(decl),

        StmtKind::Import { .. } | StmtKind::Export(..) => Disposition::Drop,

        // A strict no-regression: module top-level code has never run, because
        // imports were always dropped at IR. Running it now would need a
        // module-initialization ordering model.
        //
        // Dropping it — like dropping the entry point above — also keeps the
        // IR builder's two entry-point predicates answering the same for the
        // merged AST as for main alone: `has_top_level_executable` here,
        // `has_user_main` there. Either one flipped by an imported file turns a
        // valid main into ت٠٢٠١.
        _ => Disposition::DropExecutable,
    }
}

/// See through a `صدّر` to the declaration it wraps.
///
/// Shared with the analyzer's module passes so that "what counts as a
/// declaration" cannot drift between the types registered for checking and the
/// statements merged for IR.
pub(super) fn unwrap_exported_decl(stmt: &Stmt) -> &Stmt {
    match &stmt.kind {
        StmtKind::Export(ExportItems::Declaration(inner)) => inner,
        _ => stmt,
    }
}

fn declared_name(kind: &StmtKind) -> Option<&str> {
    match kind {
        StmtKind::FuncDecl { name, .. }
        | StmtKind::ClassDecl { name, .. }
        | StmtKind::InterfaceDecl { name, .. }
        | StmtKind::EnumDecl { name, .. }
        | StmtKind::VarDecl { name, .. } => Some(name),
        _ => None,
    }
}

/// Claim `name` for `owner`, or report a collision against whoever claimed it
/// first.
///
/// Names are NFC-normalized because two Arabic identifiers may be equal yet
/// differ byte-wise; the merged program would then carry two declarations that
/// every later stage considers one.
fn record_origin(
    origins: &mut HashMap<String, PathBuf>,
    errors: &mut Vec<Diagnostic>,
    name: &str,
    owner: &Path,
    span: Span,
) {
    let key = normalize_name(name);

    match origins.get(&key) {
        // A clash with the implicit prelude is not a module problem and must not
        // be reported as one: the user imported nothing, and `<تمهيد ترقيم>` is
        // not a path they can open. Name the real cause instead (issue #181).
        Some(first_owner) if first_owner == Path::new(PRELUDE_PATH) => errors.push(
            Diagnostic::error(
                format!(
                    "الاسم '{}' محجوز لصنف الاستثناء الأساسي المعرَّف تلقائياً؛ \
                     اختر اسماً آخر، أو ورّثه: صنف اسمك يرث {}. / \
                     The name '{}' is reserved for the built-in base exception \
                     class; pick another name, or inherit from it instead.",
                    name, name, name
                ),
                span,
            )
            .with_code(ERR_REDEFINE_PRELUDE_CLASS.to_string()),
        ),
        Some(first_owner) => errors.push(
            Diagnostic::error(
                format!(
                    "تعريف علوي مكرر '{}' عند دمج الوحدات: معرَّف في '{}' وفي '{}'. / \
                     Duplicate top-level definition '{}' while merging modules: \
                     defined in '{}' and in '{}'.",
                    name,
                    first_owner.display(),
                    owner.display(),
                    name,
                    first_owner.display(),
                    owner.display()
                ),
                span,
            )
            .with_code(ERR_DUPLICATE_EXPORT.to_string()),
        ),
        None => {
            origins.insert(key, owner.to_path_buf());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Parser;
    use std::io::Write;
    use tempfile::TempDir;

    /// The parser rejects any file lacking the بسم_الله / الحمد_لله markers, so
    /// every fixture — on disk or in memory — must go through one of these.
    fn wrap(body: &str) -> String {
        format!("بسم_الله\n{}\nالحمد_لله", body)
    }

    fn create_module(dir: &Path, name: &str, body: &str) -> PathBuf {
        let path = dir.join(name);
        let mut file = std::fs::File::create(&path).unwrap();
        file.write_all(wrap(body).as_bytes()).unwrap();
        path.canonicalize().unwrap()
    }

    fn parse_main(body: &str) -> Ast {
        Parser::new(&wrap(body)).parse().expect("main must parse")
    }

    fn loader_with(modules: &[&Path]) -> ModuleLoader {
        let mut loader = ModuleLoader::new();
        for path in modules {
            loader
                .load_module(path, Span::empty())
                .expect("fixture module must load");
        }
        loader
    }

    fn names(ast: &Ast) -> Vec<&str> {
        ast.statements
            .iter()
            .filter_map(|stmt| declared_name(&unwrap_exported_decl(stmt).kind))
            .collect()
    }

    #[test]
    fn test_exported_declaration_is_unwrapped() {
        let dir = TempDir::new().unwrap();
        let lib = create_module(
            dir.path(),
            "أدوات.ترقيم",
            "صدّر دالة جمع(أ: عدد، ب: عدد) -> عدد { أرجع أ + ب }",
        );
        let loader = loader_with(&[&lib]);

        let main = parse_main("استورد { جمع } من \"./أدوات\"\nاطبع(جمع(2، 3))");
        let mut warnings = Vec::new();
        let linked = link_program(&main, &loader, None, &mut warnings).unwrap();

        assert!(
            matches!(&linked.statements[0].kind, StmtKind::FuncDecl { name, .. } if name == "جمع"),
            "expected a bare FuncDecl, got {:?}",
            linked.statements[0].kind
        );
    }

    #[test]
    fn test_imports_and_non_declaration_exports_are_dropped() {
        let dir = TempDir::new().unwrap();
        create_module(dir.path(), "قاعدة.ترقيم", "صدّر ثابت س = 1");
        let lib = create_module(
            dir.path(),
            "أدوات.ترقيم",
            "استورد { س } من \"./قاعدة\"\nدالة ضعف(أ: عدد) -> عدد { أرجع أ * 2 }\nصدّر { ضعف }",
        );
        let loader = loader_with(&[&lib]);

        let main = parse_main("استورد { ضعف } من \"./أدوات\"\nاطبع(ضعف(2))");
        let mut warnings = Vec::new();
        let linked = link_program(&main, &loader, None, &mut warnings).unwrap();

        assert!(
            !linked
                .statements
                .iter()
                .take(linked.statements.len() - main.statements.len())
                .any(|stmt| matches!(stmt.kind, StmtKind::Import { .. } | StmtKind::Export(..))),
            "module Import/Export statements must not survive the merge"
        );
        assert!(names(&linked).contains(&"ضعف"));
        assert!(names(&linked).contains(&"س"));
    }

    #[test]
    fn test_module_top_level_executable_is_dropped_with_warning() {
        let dir = TempDir::new().unwrap();
        let lib = create_module(
            dir.path(),
            "أدوات.ترقيم",
            "صدّر دالة ضعف(أ: عدد) -> عدد { أرجع أ * 2 }\nاطبع(\"تحميل\")",
        );
        let loader = loader_with(&[&lib]);

        let main = parse_main("استورد { ضعف } من \"./أدوات\"\nاطبع(ضعف(2))");
        let mut warnings = Vec::new();
        let linked = link_program(&main, &loader, None, &mut warnings).unwrap();

        // The module's اطبع is gone; main's remains.
        let expr_stmts = linked
            .statements
            .iter()
            .filter(|stmt| matches!(stmt.kind, StmtKind::Expr(_)))
            .count();
        assert_eq!(
            expr_stmts, 1,
            "only main's executable statement may survive"
        );

        assert_eq!(
            warnings.len(),
            1,
            "one warning per module, got {:?}",
            warnings
        );
        assert!(
            warnings[0].message.contains("أدوات.ترقيم"),
            "warning must name the module: {}",
            warnings[0].message
        );
    }

    // A library that also runs standalone keeps its own رئيسية. Carrying it
    // flipped `has_user_main` for the merged AST, and a script-mode main then
    // died with ت٠٢٠١ over a دالة رئيسية it never wrote.
    #[test]
    fn test_module_entry_point_is_dropped_with_warning() {
        for module_body in [
            "صدّر دالة ضاعف(س: عدد) -> عدد { أرجع س * 2 }\nدالة رئيسية() { اطبع(\"وحدة\") }",
            "صدّر دالة ضاعف(س: عدد) -> عدد { أرجع س * 2 }\nصدّر دالة رئيسية() { اطبع(\"وحدة\") }",
        ] {
            let dir = TempDir::new().unwrap();
            let lib = create_module(dir.path(), "أدوات.ترقيم", module_body);
            let loader = loader_with(&[&lib]);

            let main = parse_main("استورد { ضاعف } من \"./أدوات\"\nاطبع(ضاعف(21))");
            let mut warnings = Vec::new();
            let linked = link_program(&main, &loader, None, &mut warnings).unwrap();

            assert_eq!(
                names(&linked),
                vec!["ضاعف"],
                "the module's رئيسية must not reach the merged AST: {}",
                module_body
            );
            assert_eq!(
                warnings.len(),
                1,
                "expected one warning, got {:?}",
                warnings
            );
            assert!(
                warnings[0].message.contains("رئيسية")
                    && warnings[0].message.contains("أدوات.ترقيم"),
                "the warning must name both the entry point and its module: {}",
                warnings[0].message
            );
        }
    }

    // The drop happens before the name is claimed, so main keeps its own
    // رئيسية instead of colliding with the module's.
    #[test]
    fn test_module_entry_point_does_not_collide_with_main_entry_point() {
        let dir = TempDir::new().unwrap();
        let lib = create_module(
            dir.path(),
            "أدوات.ترقيم",
            "صدّر دالة ضاعف(س: عدد) -> عدد { أرجع س * 2 }\nدالة رئيسية() { اطبع(\"وحدة\") }",
        );
        let loader = loader_with(&[&lib]);

        let main_path = dir.path().join("رئيسي.ترقيم");
        let main = parse_main("استورد { ضاعف } من \"./أدوات\"\nدالة رئيسية() { اطبع(ضاعف(21)) }");
        let mut warnings = Vec::new();
        let linked = link_program(&main, &loader, Some(&main_path), &mut warnings)
            .expect("a module's own entry point must not collide with main's");

        assert_eq!(names(&linked), vec!["ضاعف", "رئيسية"]);
    }

    #[test]
    fn test_duplicate_definition_is_an_error() {
        let dir = TempDir::new().unwrap();
        let lib = create_module(
            dir.path(),
            "أدوات.ترقيم",
            "صدّر دالة جمع(أ: عدد) -> عدد { أرجع أ }",
        );
        let loader = loader_with(&[&lib]);

        let main_path = dir.path().join("رئيسي.ترقيم");
        let main = parse_main("استورد { جمع } من \"./أدوات\"\nدالة جمع(أ: عدد) -> عدد { أرجع أ }");
        let mut warnings = Vec::new();
        let errors = link_program(&main, &loader, Some(&main_path), &mut warnings)
            .expect_err("a redefinition of an imported name must be fatal");

        assert_eq!(errors.len(), 1);
        assert_eq!(
            errors[0].code.as_deref(),
            Some(ERR_DUPLICATE_EXPORT.to_string().as_str())
        );
        assert!(
            errors[0].message.contains("أدوات.ترقيم") && errors[0].message.contains("رئيسي.ترقيم"),
            "both file paths must appear, since Span carries no file identity: {}",
            errors[0].message
        );
    }

    #[test]
    fn test_module_declarations_precede_main_declarations() {
        let dir = TempDir::new().unwrap();
        let lib = create_module(dir.path(), "أدوات.ترقيم", "صدّر ثابت الإصدار = \"1.0\"");
        let loader = loader_with(&[&lib]);

        let main = parse_main("استورد { الإصدار } من \"./أدوات\"\nثابت الاسم = \"ترقيم\"");
        let mut warnings = Vec::new();
        let linked = link_program(&main, &loader, None, &mut warnings).unwrap();

        assert_eq!(names(&linked), vec!["الإصدار", "الاسم"]);
    }

    #[test]
    fn test_transitive_modules_are_merged_dependency_first() {
        let dir = TempDir::new().unwrap();
        create_module(dir.path(), "ج.ترقيم", "صدّر دالة ثلاثة() -> عدد { أرجع 3 }");
        let b = create_module(
            dir.path(),
            "ب.ترقيم",
            "استورد { ثلاثة } من \"./ج\"\nصدّر دالة اثنان() -> عدد { أرجع 2 }",
        );
        let loader = loader_with(&[&b]);

        let main = parse_main("استورد { اثنان } من \"./ب\"\nاطبع(اثنان())");
        let mut warnings = Vec::new();
        let linked = link_program(&main, &loader, None, &mut warnings).unwrap();

        assert_eq!(names(&linked), vec!["ثلاثة", "اثنان"]);
    }

    #[test]
    fn test_main_present_in_cache_is_not_merged_twice() {
        let dir = TempDir::new().unwrap();
        // ب imports back into رئيسي, which caches رئيسي under its own path.
        let main_path = create_module(
            dir.path(),
            "رئيسي.ترقيم",
            "استورد { اثنان } من \"./ب\"\nدالة واحد() -> عدد { أرجع 1 }",
        );
        let b = create_module(
            dir.path(),
            "ب.ترقيم",
            "استورد { واحد } من \"./رئيسي\"\nصدّر دالة اثنان() -> عدد { أرجع 2 }",
        );

        let mut loader = ModuleLoader::new();
        let _ = loader.load_module(&b, Span::empty());
        assert!(
            loader.is_loaded(&main_path),
            "precondition: the back-import must have cached main"
        );

        let main = parse_main("استورد { اثنان } من \"./ب\"\nدالة واحد() -> عدد { أرجع 1 }");
        let mut warnings = Vec::new();
        let linked = link_program(&main, &loader, Some(&main_path), &mut warnings)
            .expect("main's own cache entry must be skipped, not treated as a collision");

        assert_eq!(names(&linked), vec!["اثنان", "واحد"]);
    }

    #[test]
    fn test_empty_cache_preserves_file_markers() {
        let loader = ModuleLoader::new();
        let main = parse_main("اطبع(\"مرحبا\")");
        let mut warnings = Vec::new();
        let linked = link_program(&main, &loader, None, &mut warnings).unwrap();

        assert!(linked.has_file_markers());
        assert_eq!(linked.bismillah_span, main.bismillah_span);
        assert_eq!(linked.alhamdulillah_span, main.alhamdulillah_span);
        assert_eq!(linked.statements.len(), main.statements.len());
        assert!(warnings.is_empty());
    }

    /// The linked program is one program, so it keeps *main's* file doc comment.
    /// An imported module's header describes a unit that no longer exists after
    /// the merge, and is dropped like its markers' spans.
    #[test]
    fn test_main_module_doc_survives_linking() {
        let loader = ModuleLoader::new();
        let main = Parser::new("بسم_الله\n/// وثيقة الملف\n\n// ملاحظة\nاطبع(\"مرحبا\")\nالحمد_لله")
            .parse()
            .expect("main must parse");
        assert_eq!(main.module_doc.as_deref(), Some("وثيقة الملف"));

        let mut warnings = Vec::new();
        let linked = link_program(&main, &loader, None, &mut warnings).unwrap();

        assert_eq!(linked.module_doc, main.module_doc);
    }
}
