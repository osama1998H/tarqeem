//! Scope and symbol table management

use super::types::Type;
use crate::error::Span;
use indexmap::IndexMap;
use std::collections::HashSet;
use unicode_normalization::UnicodeNormalization;

pub(crate) fn normalize_name(name: &str) -> String {
    name.nfc().collect()
}

#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub ty: Type,
    pub mutable: bool,
    pub defined: bool,
    /// Whether the symbol has been accessed/used in the code.
    /// Used for unused variable/function warnings.
    pub used: bool,
    /// The source location where this symbol was declared.
    /// Used for warning messages to point to the declaration site.
    /// Built-in symbols use `Span::default()`.
    pub span: Span,
}

impl Symbol {
    pub fn new(name: impl Into<String>, kind: SymbolKind, ty: Type, span: Span) -> Self {
        Self {
            name: name.into(),
            kind,
            ty,
            mutable: true,
            defined: true,
            used: false,
            span,
        }
    }

    pub fn variable(name: impl Into<String>, ty: Type, mutable: bool, span: Span) -> Self {
        Self {
            name: name.into(),
            kind: SymbolKind::Variable,
            ty,
            mutable,
            defined: true,
            used: false,
            span,
        }
    }

    pub fn function(
        name: impl Into<String>,
        params: Vec<Type>,
        return_type: Type,
        span: Span,
    ) -> Self {
        Self {
            name: name.into(),
            kind: SymbolKind::Function,
            ty: Type::Function {
                params,
                return_type: Box::new(return_type),
            },
            mutable: false,
            defined: true,
            used: false,
            span,
        }
    }

    pub fn class(name: impl Into<String>, span: Span) -> Self {
        let name = name.into();
        Self {
            name: name.clone(),
            kind: SymbolKind::Class,
            ty: Type::Class(name),
            mutable: false,
            defined: true,
            used: false,
            span,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SymbolKind {
    Variable,
    Function,
    Class,
    Interface,
    Enum,
    Parameter,
    Import,
}

#[derive(Debug)]
pub struct Scope {
    symbols: IndexMap<String, Symbol>,
    /// The subset of `symbols` registered by `register_builtins`. Built-ins are
    /// the last tier of the lookup order, so a user declaration displaces one
    /// rather than colliding with it (#262). Not inferable from
    /// `span == Span::default()` — `هذا` shares that span and must stay fixed.
    builtin_names: HashSet<String>,
    parent: Option<Box<Scope>>,
    kind: ScopeKind,
    return_type: Option<Type>,
    /// Return-expression types collected while analyzing a `Function`/
    /// `Lambda` scope's own body, so a block-bodied lambda's return type can
    /// be inferred from its `أرجع` statements (declared functions never read
    /// this back; only lambdas lack a declared return type to check against).
    inferred_returns: Vec<Type>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScopeKind {
    Global,
    Function,
    Block,
    Class,
    Loop,
    /// An arrow lambda's body. Distinct from `Function` so capture detection
    /// (issue #180) can tell "crossed into a nested lambda" apart from
    /// "crossed into a nested declared function" while both still count as
    /// function boundaries for `أرجع`/return-type purposes.
    Lambda,
}

impl Scope {
    pub fn new_global() -> Self {
        let mut scope = Self {
            symbols: IndexMap::new(),
            builtin_names: HashSet::new(),
            parent: None,
            kind: ScopeKind::Global,
            return_type: None,
            inferred_returns: Vec::new(),
        };

        Self::register_builtins(&mut scope);

        scope
    }

    fn register_builtins(scope: &mut Scope) {
        Self::register_core_builtins(scope);
    }

    /// The core built-in functions available globally without an import, as
    /// (name, parameter types, return type).
    ///
    /// This is a list rather than a run of `scope.define` calls so the set can
    /// be enumerated by tests. Each of these must reach a real implementation in
    /// *both* the interpreter and native codegen; six of them silently reached
    /// neither (#222), and nothing could detect that while the names existed
    /// only as straight-line statements.
    fn core_builtins() -> Vec<(&'static str, Vec<Type>, Type)> {
        vec![
            // I/O - دوال الإدخال والإخراج
            ("اطبع", vec![Type::Any], Type::Void),
            ("طباعة", vec![Type::Any], Type::Void),
            ("اطبع_سطر", vec![Type::Any], Type::Void),
            ("اطبع_خطأ", vec![Type::Any], Type::Void),
            ("ادخل", vec![], Type::String),
            ("ادخل_رسالة", vec![Type::String], Type::String),
            // Type introspection - دوال فحص الأنماط
            ("طول", vec![Type::Any], Type::Int),
            ("نوع", vec![Type::Any], Type::String),
            // Type conversion - دوال تحويل الأنماط
            ("عدد", vec![Type::Any], Type::Int),
            ("عدد_عشري", vec![Type::Any], Type::Float),
            ("نص", vec![Type::Any], Type::String),
            ("منطقي", vec![Type::Any], Type::Bool),
            // Control - دوال التحكم
            ("تأكد", vec![Type::Bool], Type::Void),
            ("تأكد_رسالة", vec![Type::Bool, Type::String], Type::Void),
            ("توقف", vec![Type::String], Type::Void),
            ("نم", vec![Type::Int], Type::Void),
            // Arrays - دوال المصفوفات
            ("طول_مصفوفة", vec![Type::Any], Type::Int),
            ("الحق", vec![Type::Any, Type::Any], Type::Void),
            // Strings - دوال النصوص
            // The codepoint slicer, and core tier rather than the `نص` module
            // because §5.2 keeps a no-import name a builtin: it indexes by
            // codepoint, not by byte, which is what keeps a caller's «عدد أحرف»
            // from cutting an Arabic letter in half. Total — a negative start, a
            // start past the end, or a non-positive length all answer `""`, and a
            // length past the end clamps — so it never needs a range check at the
            // call site. Mirrors `trq_string_substr_chars`.
            (
                "قص_حروف",
                vec![Type::String, Type::Int, Type::Int],
                Type::String,
            ),
            // The codepoint accessor: `-1` when there is no first character,
            // since U+0000 is itself a codepoint. Takes `نص` rather than `أي`
            // so it composes at a concrete type in every backend.
            ("حرف_إلى_رمز", vec![Type::String], Type::Int),
            // Its inverse, and `""` where the accessor answers `-1`, so the two
            // are total in both directions.
            ("رمز_إلى_حرف", vec![Type::Int], Type::String),
            // The byte bridge. `مصفوفة<عدد>` rather than `أي` — the only core
            // entry with a concrete array type, and it is what types the
            // element for `س[ي]`; `طول_مصفوفة`/`الحق` take `أي` and cannot.
            (
                "نص_إلى_ثنائي",
                vec![Type::String],
                Type::Array(Box::new(Type::Int)),
            ),
            // Its inverse, and the only core entry *taking* a concrete array.
            // Total: an array that is not the UTF-8 encoding of a string — an
            // element outside 0-255, or an invalid byte sequence — answers `""`,
            // which the caller tells from a legitimately empty result by the
            // length of what it passed in.
            (
                "ثنائي_إلى_نص",
                vec![Type::Array(Box::new(Type::Int))],
                Type::String,
            ),
            // Bitwise - عمليات البتات
            // Functions, not operators: there is no `&`/`|`/`^` token, and
            // `بتات_` opens the name because `و`/`أو` are keywords.
            ("بتات_و", vec![Type::Int, Type::Int], Type::Int),
            ("بتات_أو", vec![Type::Int, Type::Int], Type::Int),
            ("بتات_أو_حصري", vec![Type::Int, Type::Int], Type::Int),
            // Unary, and distinct from the logical `ليس`, which takes a `منطقي`.
            ("بتات_نفي", vec![Type::Int], Type::Int),
            // Total: an amount outside 0-63 is a complete shift rather than a
            // per-backend divergence, so the vacated bits are filled the way
            // each shift always fills them — zeros for يسار and
            // يمين_منطقية, the sign for يمين. See `build_core_builtin_call`.
            ("بتات_إزاحة_يسار", vec![Type::Int, Type::Int], Type::Int),
            ("بتات_إزاحة_يمين", vec![Type::Int, Type::Int], Type::Int),
            // `عدد` is signed and every backend's `Shr` is arithmetic, so zero
            // fill is unreachable through يمين at any amount.
            (
                "بتات_إزاحة_يمين_منطقية",
                vec![Type::Int, Type::Int],
                Type::Int,
            ),
            // Environment - البيئة
            // `getenv(3)`, which no composition of the names above can reach.
            // Total: an absent name, an empty one and `لا_شيء` all answer `""`,
            // so set-but-empty is deliberately indistinguishable from unset.
            ("متغير_بيئة", vec![Type::String], Type::String),
            // Stream writing - الكتابة في مجرى
            // `write(2)`, the byte-level output path the print intrinsics cannot
            // be: their lowering picks a symbol from the static type, and that
            // selection is the dispatch. Takes bytes, so `نص_إلى_ثنائي` feeds it
            // directly. `١` is stdout, `٢` is stderr, `٣` upward a file handle;
            // the answer is the byte count, and `-١` when nothing could be
            // written.
            (
                "اكتب_مجرى",
                vec![Type::Int, Type::Array(Box::new(Type::Int))],
                Type::Int,
            ),
            // Stream reading - القراءة من مجرى
            // `read(2)`, the byte-level input path `ادخل` cannot be: that one
            // frames a line in Rust and answers a decoded `نص`, so no program
            // can read one byte or do its own framing. Answers bytes, so
            // `ثنائي_إلى_نص` decodes them. `٠` is stdin and `٣` upward a file
            // handle; the answer is an empty array whenever there is nothing to
            // read — at EOF, and for a stream that cannot be read from. An array
            // has no value to spare for a sentinel, so those two cases are
            // deliberately indistinguishable.
            (
                "اقرأ_مجرى",
                vec![Type::Int, Type::Int],
                Type::Array(Box::new(Type::Int)),
            ),
            // Path status - حالة المسار
            // `stat(2)`, one field per call so the answer stays an `عدد` and no
            // struct crosses the FFI. `حقل ٠` is the kind — `٠` معدوم، `١` ملف،
            // `٢` مجلد، `٣` موجود وليس أحدهما — and `حقل ١` the byte length of a
            // regular file. Any other field, and the size of anything that is
            // not a file, answer `-١`. Total, and the field is settled before the
            // path, so an unknown field never touches the filesystem.
            //
            // The fourth kind is what lets this one name answer for
            // `ملف_موجود`، `هل_ملف`، `هل_مجلد` and `حجم_ملف` together: a device
            // exists and is not a file, so those first two disagree about it and
            // three values could not carry both.
            ("حالة_مسار", vec![Type::String, Type::Int], Type::Int),
            // Path deletion - حذف المسار
            // `unlink(2)` for a file, `rmdir(2)` for an empty directory, chosen
            // by **lstat**: a symlink is unlinked, never rmdir'd, so this acts on
            // the name where `حالة_مسار` answers about the target. Not recursive
            // — a non-empty directory answers `خطأ`, which keeps `احذف_مجلد`'s
            // contract when it becomes a wrapper. Total: absent, empty and
            // `لا_شيء` all answer `خطأ`.
            ("احذف_مسار", vec![Type::String], Type::Bool),
            // Termination - إنهاء البرنامج
            // `exit(2)`, which `توقف` cannot serve: that one is always status 1
            // and always prints. Total — the status is `حالة & ٢٥٥`, so every
            // `عدد` is valid — and uncatchable, since `حاول` only intercepts a
            // thrown `استثناء`.
            //
            // Two spellings, one primitive. The kasra marks the dropped ya of
            // the imperative and Arabic writers routinely omit it, which is why
            // the keyword table carries `ارمِ`/`ارم` and `أرجع`/`ارجع` as pairs;
            // `normalize_name` is NFC only and does not strip tashkeel, so the
            // variant is a genuinely different identifier and needs its own
            // entry rather than folding into the first.
            ("أنهِ_البرنامج", vec![Type::Int], Type::Void),
            ("أنه_البرنامج", vec![Type::Int], Type::Void),
        ]
    }

    /// The names of the core built-ins, for tests that assert every one of them
    /// is executable in every backend.
    pub fn core_builtin_names() -> Vec<&'static str> {
        Self::core_builtins()
            .into_iter()
            .map(|(name, _, _)| name)
            .collect()
    }

    /// Registers the core built-in functions that are available globally without
    /// requiring an import. All other functions require explicit imports from
    /// stdlib modules.
    fn register_core_builtins(scope: &mut Scope) {
        for (name, params, return_type) in Self::core_builtins() {
            scope.define(Symbol::function(name, params, return_type, Span::default()));
            scope.builtin_names.insert(normalize_name(name));
        }
    }

    /// Returns the type signature for a stdlib builtin function.
    /// This is called by the analyzer when processing imports to register
    /// stdlib functions that are implemented as native builtins.
    ///
    /// # Arguments
    /// * `module` - The stdlib module name (e.g., "رياضيات", "نص", "ملفات")
    /// * `name` - The function name within that module
    ///
    /// # Returns
    /// * `Some(Symbol)` if the function exists in the stdlib module
    /// * `None` if the function is not a stdlib builtin
    pub fn get_stdlib_builtin(module: &str, name: &str) -> Option<Symbol> {
        fn builtin(name: &str, params: Vec<Type>, return_type: Type) -> Symbol {
            Symbol::function(name, params, return_type, Span::default())
        }

        match module {
            // =======================================================================
            // رياضيات (Math) - Mathematical functions
            // =======================================================================
            "رياضيات" => match name {
                // Basic math
                "مطلق" => Some(builtin("مطلق", vec![Type::Any], Type::Any)),
                "مطلق_عدد" => Some(builtin("مطلق_عدد", vec![Type::Int], Type::Int)),
                "قوة" => Some(builtin("قوة", vec![Type::Float, Type::Float], Type::Float)),
                "قوة_عدد" => Some(builtin("قوة_عدد", vec![Type::Int, Type::Int], Type::Int)),
                "جذر" => Some(builtin("جذر", vec![Type::Float], Type::Float)),
                "جذر_تكعيبي" => {
                    Some(builtin("جذر_تكعيبي", vec![Type::Float], Type::Float))
                }

                // Logarithms
                "لوغاريتم" => Some(builtin("لوغاريتم", vec![Type::Float], Type::Float)),
                "لوغ10" => Some(builtin("لوغ10", vec![Type::Float], Type::Float)),
                "لوغاريتم10" => Some(builtin("لوغاريتم10", vec![Type::Float], Type::Float)),
                "لوغ2" => Some(builtin("لوغ2", vec![Type::Float], Type::Float)),

                // Exponential
                "أس" => Some(builtin("أس", vec![Type::Float], Type::Float)),
                "أسي" => Some(builtin("أسي", vec![Type::Float], Type::Float)),

                // Rounding
                "أرضية" => Some(builtin("أرضية", vec![Type::Float], Type::Float)),
                "سقف" => Some(builtin("سقف", vec![Type::Float], Type::Float)),
                "قرّب" => Some(builtin("قرّب", vec![Type::Float], Type::Float)),
                "تقريب" => Some(builtin("تقريب", vec![Type::Float], Type::Float)),
                "اقتطع" => Some(builtin("اقتطع", vec![Type::Float], Type::Float)),

                // Min/Max
                "أقل" => Some(builtin("أقل", vec![Type::Any, Type::Any], Type::Any)),
                "أدنى" => Some(builtin("أدنى", vec![Type::Any, Type::Any], Type::Any)),
                "أقل_عدد" => Some(builtin("أقل_عدد", vec![Type::Int, Type::Int], Type::Int)),
                "أكبر" => Some(builtin("أكبر", vec![Type::Any, Type::Any], Type::Any)),
                "أقصى" => Some(builtin("أقصى", vec![Type::Any, Type::Any], Type::Any)),
                "أكبر_عدد" => {
                    Some(builtin("أكبر_عدد", vec![Type::Int, Type::Int], Type::Int))
                }

                // Clamp
                "حصر" => Some(builtin(
                    "حصر",
                    vec![Type::Any, Type::Any, Type::Any],
                    Type::Any,
                )),
                "حصر_عدد" => Some(builtin(
                    "حصر_عدد",
                    vec![Type::Int, Type::Int, Type::Int],
                    Type::Int,
                )),

                // Other math
                "علامة" => Some(builtin("علامة", vec![Type::Int], Type::Int)),
                "باقي" => Some(builtin("باقي", vec![Type::Int, Type::Int], Type::Int)),
                "قاسم_مشترك" => {
                    Some(builtin("قاسم_مشترك", vec![Type::Int, Type::Int], Type::Int))
                }
                "مضاعف_مشترك" => Some(builtin(
                    "مضاعف_مشترك",
                    vec![Type::Int, Type::Int],
                    Type::Int,
                )),
                "عاملي" => Some(builtin("عاملي", vec![Type::Int], Type::Int)),

                // Trigonometry
                "جا" => Some(builtin("جا", vec![Type::Float], Type::Float)),
                "جيب" => Some(builtin("جيب", vec![Type::Float], Type::Float)),
                "جتا" => Some(builtin("جتا", vec![Type::Float], Type::Float)),
                "جيب_التمام" => {
                    Some(builtin("جيب_التمام", vec![Type::Float], Type::Float))
                }
                "ظا" => Some(builtin("ظا", vec![Type::Float], Type::Float)),
                "ظل" => Some(builtin("ظل", vec![Type::Float], Type::Float)),
                "ظتا" => Some(builtin("ظتا", vec![Type::Float], Type::Float)),
                "ظل_التمام" => Some(builtin("ظل_التمام", vec![Type::Float], Type::Float)),
                "قا" => Some(builtin("قا", vec![Type::Float], Type::Float)),
                "قاطع" => Some(builtin("قاطع", vec![Type::Float], Type::Float)),
                "قتا" => Some(builtin("قتا", vec![Type::Float], Type::Float)),
                "قاطع_التمام" => {
                    Some(builtin("قاطع_التمام", vec![Type::Float], Type::Float))
                }

                // Inverse trig
                "جا_عكسي" => Some(builtin("جا_عكسي", vec![Type::Float], Type::Float)),
                "جيب_عكسي" => Some(builtin("جيب_عكسي", vec![Type::Float], Type::Float)),
                "جتا_عكسي" => Some(builtin("جتا_عكسي", vec![Type::Float], Type::Float)),
                "جيب_تمام_عكسي" => {
                    Some(builtin("جيب_تمام_عكسي", vec![Type::Float], Type::Float))
                }
                "ظا_عكسي" => Some(builtin("ظا_عكسي", vec![Type::Float], Type::Float)),
                "ظا_عكسي2" => Some(builtin(
                    "ظا_عكسي2",
                    vec![Type::Float, Type::Float],
                    Type::Float,
                )),

                // Hyperbolic
                "جا_زائدي" => Some(builtin("جا_زائدي", vec![Type::Float], Type::Float)),
                "جتا_زائدي" => Some(builtin("جتا_زائدي", vec![Type::Float], Type::Float)),
                "ظا_زائدي" => Some(builtin("ظا_زائدي", vec![Type::Float], Type::Float)),

                // Angle conversion
                "الى_راديان" => {
                    Some(builtin("الى_راديان", vec![Type::Float], Type::Float))
                }
                "راديان" => Some(builtin("راديان", vec![Type::Float], Type::Float)),
                "الى_درجات" => Some(builtin("الى_درجات", vec![Type::Float], Type::Float)),
                "درجات" => Some(builtin("درجات", vec![Type::Float], Type::Float)),

                // Random
                "بذرة_عشوائية" => {
                    Some(builtin("بذرة_عشوائية", vec![Type::Int], Type::Void))
                }
                "بذرة_عشوائي" => {
                    Some(builtin("بذرة_عشوائي", vec![Type::Int], Type::Void))
                }
                "عشوائي" => Some(builtin("عشوائي", vec![], Type::Int)),
                "عشوائي_عدد" => Some(builtin("عشوائي_عدد", vec![], Type::Int)),
                "عشوائي_بين" => {
                    Some(builtin("عشوائي_بين", vec![Type::Int, Type::Int], Type::Int))
                }
                "عشوائي_عدد_بين" => Some(builtin(
                    "عشوائي_عدد_بين",
                    vec![Type::Int, Type::Int],
                    Type::Int,
                )),
                "عشوائي_عشري" => Some(builtin("عشوائي_عشري", vec![], Type::Float)),
                "عشوائي_عشري_بين" => Some(builtin(
                    "عشوائي_عشري_بين",
                    vec![Type::Float, Type::Float],
                    Type::Float,
                )),
                "عشوائي_منطقي" => Some(builtin("عشوائي_منطقي", vec![], Type::Bool)),

                _ => None,
            },

            // =======================================================================
            // نص (String) - String manipulation functions
            // =======================================================================
            "نص" => match name {
                // Slicing. `قص_حروف` used to sit here and is now core tier, so it
                // needs no import; `قص_نص` sliced by *byte* and is gone — the
                // primitive string surface is uniformly codepoint-indexed, and
                // byte-level work goes through `نص_إلى_ثنائي`/`ثنائي_إلى_نص`.
                "حرف_في" => Some(builtin(
                    "حرف_في",
                    vec![Type::String, Type::Int],
                    Type::String,
                )),

                // Search
                "يحتوي" => Some(builtin(
                    "يحتوي",
                    vec![Type::String, Type::String],
                    Type::Bool,
                )),
                "يبدأ_بـ" => Some(builtin(
                    "يبدأ_بـ",
                    vec![Type::String, Type::String],
                    Type::Bool,
                )),
                "ينتهي_بـ" => Some(builtin(
                    "ينتهي_بـ",
                    vec![Type::String, Type::String],
                    Type::Bool,
                )),
                "موضع" => Some(builtin("موضع", vec![Type::String, Type::String], Type::Int)),
                "موضع_اخير" => Some(builtin(
                    "موضع_اخير",
                    vec![Type::String, Type::String],
                    Type::Int,
                )),
                "عدد_مرات" => Some(builtin(
                    "عدد_مرات",
                    vec![Type::String, Type::String],
                    Type::Int,
                )),

                // Case conversion
                "كبير" => Some(builtin("كبير", vec![Type::String], Type::String)),
                "صغير" => Some(builtin("صغير", vec![Type::String], Type::String)),
                "عنوان" => Some(builtin("عنوان", vec![Type::String], Type::String)),
                "اعكس_نص" => Some(builtin("اعكس_نص", vec![Type::String], Type::String)),

                // Trimming
                "ازل_فراغات" => {
                    Some(builtin("ازل_فراغات", vec![Type::String], Type::String))
                }
                "ازل_فراغات_يسار" => {
                    Some(builtin("ازل_فراغات_يسار", vec![Type::String], Type::String))
                }
                "ازل_فراغات_يمين" => {
                    Some(builtin("ازل_فراغات_يمين", vec![Type::String], Type::String))
                }

                // Split/Join
                "قسّم" => Some(builtin(
                    "قسّم",
                    vec![Type::String, Type::String],
                    Type::Array(Box::new(Type::String)),
                )),
                "ادمج" => Some(builtin(
                    "ادمج",
                    vec![Type::Array(Box::new(Type::String)), Type::String],
                    Type::String,
                )),

                // Replace
                "استبدل" => Some(builtin(
                    "استبدل",
                    vec![Type::String, Type::String, Type::String],
                    Type::String,
                )),
                "استبدل_كل" => Some(builtin(
                    "استبدل_كل",
                    vec![Type::String, Type::String, Type::String],
                    Type::String,
                )),

                // Repeat/Pad
                "كرر_نص" => Some(builtin(
                    "كرر_نص",
                    vec![Type::String, Type::Int],
                    Type::String,
                )),
                "كرر" => Some(builtin("كرر", vec![Type::String, Type::Int], Type::String)),
                "احشو_يسار" => Some(builtin(
                    "احشو_يسار",
                    vec![Type::String, Type::Int, Type::String],
                    Type::String,
                )),
                "احشو_يمين" => Some(builtin(
                    "احشو_يمين",
                    vec![Type::String, Type::Int, Type::String],
                    Type::String,
                )),

                // Length
                "طول_نص" => Some(builtin("طول_نص", vec![Type::String], Type::Int)),
                "طول_حروف" => Some(builtin("طول_حروف", vec![Type::String], Type::Int)),

                // Validation
                "رقمي" => Some(builtin("رقمي", vec![Type::String], Type::Bool)),
                "حروف_فقط" => Some(builtin("حروف_فقط", vec![Type::String], Type::Bool)),
                "عربي" => Some(builtin("عربي", vec![Type::String], Type::Bool)),

                // Comparison
                "قارن_نص" => Some(builtin(
                    "قارن_نص",
                    vec![Type::String, Type::String],
                    Type::Int,
                )),
                "نصوص_متساوية" => Some(builtin(
                    "نصوص_متساوية",
                    vec![Type::String, Type::String],
                    Type::Bool,
                )),
                "نص_يحتوي" => Some(builtin(
                    "نص_يحتوي",
                    vec![Type::String, Type::String],
                    Type::Bool,
                )),
                "نص_يبدأ_بـ" => Some(builtin(
                    "نص_يبدأ_بـ",
                    vec![Type::String, Type::String],
                    Type::Bool,
                )),
                "نص_ينتهي_بـ" => Some(builtin(
                    "نص_ينتهي_بـ",
                    vec![Type::String, Type::String],
                    Type::Bool,
                )),

                // Conversion
                "عدد_لنص" => Some(builtin("عدد_لنص", vec![Type::Int], Type::String)),
                "عشري_لنص" => Some(builtin("عشري_لنص", vec![Type::Float], Type::String)),
                "منطقي_لنص" => Some(builtin("منطقي_لنص", vec![Type::Bool], Type::String)),
                "نص_لعدد" => Some(builtin("نص_لعدد", vec![Type::String], Type::Int)),
                "نص_لعشري" => Some(builtin("نص_لعشري", vec![Type::String], Type::Float)),

                // Input helpers
                "ادخل_عدد" => Some(builtin("ادخل_عدد", vec![], Type::Int)),
                "ادخل_عشري" => Some(builtin("ادخل_عشري", vec![], Type::Float)),

                _ => None,
            },

            // =======================================================================
            // ملفات (Files) - File system functions
            // =======================================================================
            "ملفات" => match name {
                // File checks
                "ملف_موجود" => Some(builtin("ملف_موجود", vec![Type::String], Type::Bool)),
                "هل_ملف" => Some(builtin("هل_ملف", vec![Type::String], Type::Bool)),
                "هل_مجلد" => Some(builtin("هل_مجلد", vec![Type::String], Type::Bool)),

                // File operations
                "اقرأ_ملف" => Some(builtin("اقرأ_ملف", vec![Type::String], Type::String)),
                "اكتب_ملف" => Some(builtin(
                    "اكتب_ملف",
                    vec![Type::String, Type::String],
                    Type::Bool,
                )),
                "الحق_ملف" => Some(builtin(
                    "الحق_ملف",
                    vec![Type::String, Type::String],
                    Type::Bool,
                )),
                "احذف_ملف" => Some(builtin("احذف_ملف", vec![Type::String], Type::Bool)),
                "انسخ_ملف" => Some(builtin(
                    "انسخ_ملف",
                    vec![Type::String, Type::String],
                    Type::Bool,
                )),
                "انقل_ملف" => Some(builtin(
                    "انقل_ملف",
                    vec![Type::String, Type::String],
                    Type::Bool,
                )),
                "حجم_ملف" => Some(builtin("حجم_ملف", vec![Type::String], Type::Int)),

                // Directory operations
                "انشئ_مجلد" => Some(builtin("انشئ_مجلد", vec![Type::String], Type::Bool)),
                "قائمة_مجلد" => Some(builtin(
                    "قائمة_مجلد",
                    vec![Type::String],
                    Type::Array(Box::new(Type::String)),
                )),
                "احذف_مجلد" => Some(builtin("احذف_مجلد", vec![Type::String], Type::Bool)),
                "مجلد_حالي" => Some(builtin("مجلد_حالي", vec![], Type::String)),
                "مجلد_مستخدم" => Some(builtin("مجلد_مستخدم", vec![], Type::String)),
                "مجلد_مؤقت" => Some(builtin("مجلد_مؤقت", vec![], Type::String)),

                // Path operations
                "ادمج_مسار" => Some(builtin(
                    "ادمج_مسار",
                    vec![Type::String, Type::String],
                    Type::String,
                )),
                "مسار_اب" => Some(builtin("مسار_اب", vec![Type::String], Type::String)),
                "اسم_ملف" => Some(builtin("اسم_ملف", vec![Type::String], Type::String)),
                "امتداد_ملف" => {
                    Some(builtin("امتداد_ملف", vec![Type::String], Type::String))
                }
                "فاصل_مسار" => Some(builtin("فاصل_مسار", vec![], Type::String)),

                _ => None,
            },

            // =======================================================================
            // وقت (Time) - Time functions
            // =======================================================================
            "وقت" => match name {
                "وقت_الآن" => Some(builtin("وقت_الآن", vec![], Type::Int)),
                "وقت_أداء" => Some(builtin("وقت_أداء", vec![], Type::Int)),
                _ => None,
            },

            // =======================================================================
            // تشفير (Crypto) - Cryptography functions
            // =======================================================================
            "تشفير" => match name {
                // SHA-256 (بصمة = fingerprint)
                "احسب_بصمة" => Some(builtin("احسب_بصمة", vec![Type::String], Type::String)),
                "بصمة_ملف" => Some(builtin("بصمة_ملف", vec![Type::String], Type::String)),
                "بصمة_ثنائي" => Some(builtin(
                    "بصمة_ثنائي",
                    vec![Type::Array(Box::new(Type::Int))],
                    Type::String,
                )),
                "طابق_بصمة" => Some(builtin(
                    "طابق_بصمة",
                    vec![Type::String, Type::String],
                    Type::Bool,
                )),

                // Hex encoding
                "إلى_ست_عشري" => {
                    Some(builtin("إلى_ست_عشري", vec![Type::String], Type::String))
                }
                "من_ست_عشري" => {
                    Some(builtin("من_ست_عشري", vec![Type::String], Type::String))
                }
                "ثنائي_إلى_ست_عشري" => Some(builtin(
                    "ثنائي_إلى_ست_عشري",
                    vec![Type::Array(Box::new(Type::Int))],
                    Type::String,
                )),
                "ست_عشري_إلى_ثنائي" => Some(builtin(
                    "ست_عشري_إلى_ثنائي",
                    vec![Type::String],
                    Type::Array(Box::new(Type::Int)),
                )),

                _ => None,
            },

            // =======================================================================
            // ضغط (Compression) - Compression functions
            // =======================================================================
            "ضغط" => match name {
                "اضغط" => Some(builtin(
                    "اضغط",
                    vec![Type::String],
                    Type::Array(Box::new(Type::Int)),
                )),
                "فك_الضغط" => Some(builtin(
                    "فك_الضغط",
                    vec![Type::Array(Box::new(Type::Int))],
                    Type::String,
                )),
                "اضغط_ثنائي" => Some(builtin(
                    "اضغط_ثنائي",
                    vec![Type::Array(Box::new(Type::Int))],
                    Type::Array(Box::new(Type::Int)),
                )),
                "فك_ضغط_ثنائي" => Some(builtin(
                    "فك_ضغط_ثنائي",
                    vec![Type::Array(Box::new(Type::Int))],
                    Type::Array(Box::new(Type::Int)),
                )),
                "اضغط_ملف" => Some(builtin(
                    "اضغط_ملف",
                    vec![Type::String, Type::String],
                    Type::Bool,
                )),
                "فك_ضغط_ملف" => Some(builtin(
                    "فك_ضغط_ملف",
                    vec![Type::String, Type::String],
                    Type::Bool,
                )),
                _ => None,
            },

            // =======================================================================
            // شبكة (Network) - Networking functions
            // =======================================================================
            "شبكة" => match name {
                // TCP
                "اتصل_خادم" => Some(builtin(
                    "اتصل_خادم",
                    vec![Type::String, Type::Int, Type::Int],
                    Type::Int,
                )),
                "أغلق_اتصال" => Some(builtin("أغلق_اتصال", vec![Type::Int], Type::Void)),
                "أرسل" => Some(builtin("أرسل", vec![Type::Int, Type::String], Type::Bool)),
                "أرسل_بايتات" => Some(builtin(
                    "أرسل_بايتات",
                    vec![Type::Int, Type::Array(Box::new(Type::Int))],
                    Type::Bool,
                )),
                "استقبل" => Some(builtin("استقبل", vec![Type::Int, Type::Int], Type::String)),
                "استقبل_بايتات" => Some(builtin(
                    "استقبل_بايتات",
                    vec![Type::Int, Type::Int, Type::Int],
                    Type::Array(Box::new(Type::Int)),
                )),
                "استقبل_حتى" => Some(builtin(
                    "استقبل_حتى",
                    vec![Type::Int, Type::String, Type::Int],
                    Type::String,
                )),
                "هل_متاح" => Some(builtin("هل_متاح", vec![Type::Int], Type::Bool)),
                "استمع" => Some(builtin(
                    "استمع",
                    vec![Type::String, Type::Int, Type::Int],
                    Type::Int,
                )),
                "اقبل_اتصال" => Some(builtin("اقبل_اتصال", vec![Type::Int], Type::Int)),
                "عنوان_محلي" => Some(builtin("عنوان_محلي", vec![Type::Int], Type::String)),
                "منفذ_محلي" => Some(builtin("منفذ_محلي", vec![Type::Int], Type::Int)),

                // UDP
                "ارتبط_منفذ" => Some(builtin("ارتبط_منفذ", vec![Type::Int], Type::Int)),
                "أرسل_إلى" => Some(builtin(
                    "أرسل_إلى",
                    vec![Type::Int, Type::String, Type::Int, Type::String],
                    Type::Bool,
                )),
                "استقبل_من" => Some(builtin(
                    "استقبل_من",
                    vec![Type::Int, Type::Int],
                    Type::String,
                )),
                "رد" => Some(builtin("رد", vec![Type::Int, Type::String], Type::Bool)),

                // DNS
                "حل_اسم_نطاق" => {
                    Some(builtin("حل_اسم_نطاق", vec![Type::String], Type::String))
                }
                "عنوان_محلي_للجهاز" => {
                    Some(builtin("عنوان_محلي_للجهاز", vec![], Type::String))
                }

                // HTTP
                "طلب_ويب" => Some(builtin(
                    "طلب_ويب",
                    vec![
                        Type::String,
                        Type::String,
                        Type::Array(Box::new(Type::String)),
                        Type::String,
                        Type::Int,
                        Type::Bool,
                    ],
                    Type::String,
                )),
                "احصل_ويب" => Some(builtin("احصل_ويب", vec![Type::String], Type::String)),
                "حمّل_ملف" => Some(builtin(
                    "حمّل_ملف",
                    vec![Type::String, Type::String],
                    Type::Bool,
                )),

                // URL encoding
                "رمّز_رابط" => Some(builtin("رمّز_رابط", vec![Type::String], Type::String)),
                "فك_ترميز_رابط" => {
                    Some(builtin("فك_ترميز_رابط", vec![Type::String], Type::String))
                }

                _ => None,
            },

            _ => None,
        }
    }

    /// Returns a list of all stdlib module names
    pub fn get_stdlib_modules() -> &'static [&'static str] {
        &["رياضيات", "نص", "ملفات", "وقت", "تشفير", "ضغط", "شبكة"]
    }

    /// Returns all function names exported by a stdlib module
    pub fn get_stdlib_module_exports(module: &str) -> Vec<&'static str> {
        match module {
            "رياضيات" => vec![
                "مطلق",
                "مطلق_عدد",
                "قوة",
                "قوة_عدد",
                "جذر",
                "جذر_تكعيبي",
                "لوغاريتم",
                "لوغ10",
                "لوغاريتم10",
                "لوغ2",
                "أس",
                "أسي",
                "أرضية",
                "سقف",
                "قرّب",
                "تقريب",
                "اقتطع",
                "أقل",
                "أدنى",
                "أقل_عدد",
                "أكبر",
                "أقصى",
                "أكبر_عدد",
                "حصر",
                "حصر_عدد",
                "علامة",
                "باقي",
                "قاسم_مشترك",
                "مضاعف_مشترك",
                "عاملي",
                "جا",
                "جيب",
                "جتا",
                "جيب_التمام",
                "ظا",
                "ظل",
                "ظتا",
                "ظل_التمام",
                "قا",
                "قاطع",
                "قتا",
                "قاطع_التمام",
                "جا_عكسي",
                "جيب_عكسي",
                "جتا_عكسي",
                "جيب_تمام_عكسي",
                "ظا_عكسي",
                "ظا_عكسي2",
                "جا_زائدي",
                "جتا_زائدي",
                "ظا_زائدي",
                "الى_راديان",
                "راديان",
                "الى_درجات",
                "درجات",
                "بذرة_عشوائية",
                "بذرة_عشوائي",
                "عشوائي",
                "عشوائي_عدد",
                "عشوائي_بين",
                "عشوائي_عدد_بين",
                "عشوائي_عشري",
                "عشوائي_عشري_بين",
                "عشوائي_منطقي",
            ],
            "نص" => vec![
                "حرف_في",
                "يحتوي",
                "يبدأ_بـ",
                "ينتهي_بـ",
                "موضع",
                "موضع_اخير",
                "عدد_مرات",
                "كبير",
                "صغير",
                "عنوان",
                "اعكس_نص",
                "ازل_فراغات",
                "ازل_فراغات_يسار",
                "ازل_فراغات_يمين",
                "قسّم",
                "ادمج",
                "استبدل",
                "استبدل_كل",
                "كرر_نص",
                "كرر",
                "احشو_يسار",
                "احشو_يمين",
                "طول_نص",
                "طول_حروف",
                "رقمي",
                "حروف_فقط",
                "عربي",
                "قارن_نص",
                "نصوص_متساوية",
                "نص_يحتوي",
                "نص_يبدأ_بـ",
                "نص_ينتهي_بـ",
                "عدد_لنص",
                "عشري_لنص",
                "منطقي_لنص",
                "نص_لعدد",
                "نص_لعشري",
                "ادخل_عدد",
                "ادخل_عشري",
            ],
            "ملفات" => vec![
                "ملف_موجود",
                "هل_ملف",
                "هل_مجلد",
                "اقرأ_ملف",
                "اكتب_ملف",
                "الحق_ملف",
                "احذف_ملف",
                "انسخ_ملف",
                "انقل_ملف",
                "حجم_ملف",
                "انشئ_مجلد",
                "قائمة_مجلد",
                "احذف_مجلد",
                "مجلد_حالي",
                "مجلد_مستخدم",
                "مجلد_مؤقت",
                "ادمج_مسار",
                "مسار_اب",
                "اسم_ملف",
                "امتداد_ملف",
                "فاصل_مسار",
            ],
            "وقت" => vec!["وقت_الآن", "وقت_أداء"],
            "تشفير" => vec![
                "احسب_بصمة",
                "بصمة_ملف",
                "بصمة_ثنائي",
                "طابق_بصمة",
                "إلى_ست_عشري",
                "من_ست_عشري",
                "ثنائي_إلى_ست_عشري",
                "ست_عشري_إلى_ثنائي",
            ],
            "ضغط" => vec![
                "اضغط",
                "فك_الضغط",
                "اضغط_ثنائي",
                "فك_ضغط_ثنائي",
                "اضغط_ملف",
                "فك_ضغط_ملف",
            ],
            "شبكة" => vec![
                "اتصل_خادم",
                "أغلق_اتصال",
                "أرسل",
                "أرسل_بايتات",
                "استقبل",
                "استقبل_بايتات",
                "استقبل_حتى",
                "هل_متاح",
                "استمع",
                "اقبل_اتصال",
                "عنوان_محلي",
                "منفذ_محلي",
                "ارتبط_منفذ",
                "أرسل_إلى",
                "استقبل_من",
                "رد",
                "حل_اسم_نطاق",
                "عنوان_محلي_للجهاز",
                "طلب_ويب",
                "احصل_ويب",
                "حمّل_ملف",
                "رمّز_رابط",
                "فك_ترميز_رابط",
            ],
            _ => vec![],
        }
    }

    pub fn new_child(parent: Scope, kind: ScopeKind) -> Self {
        Self {
            symbols: IndexMap::new(),
            builtin_names: HashSet::new(),
            parent: Some(Box::new(parent)),
            kind,
            return_type: None,
            inferred_returns: Vec::new(),
        }
    }

    pub fn new_function(parent: Scope, ret_type: Type) -> Self {
        Self {
            symbols: IndexMap::new(),
            builtin_names: HashSet::new(),
            parent: Some(Box::new(parent)),
            kind: ScopeKind::Function,
            return_type: Some(ret_type),
            inferred_returns: Vec::new(),
        }
    }

    /// Mirrors `new_function`, but tags the scope `Lambda` so capture
    /// detection and return-type inference (issue #180) can distinguish an
    /// arrow lambda's body from a declared function's.
    pub fn new_lambda(parent: Scope, ret_type: Type) -> Self {
        Self {
            symbols: IndexMap::new(),
            builtin_names: HashSet::new(),
            parent: Some(Box::new(parent)),
            kind: ScopeKind::Lambda,
            return_type: Some(ret_type),
            inferred_returns: Vec::new(),
        }
    }

    pub fn kind(&self) -> ScopeKind {
        self.kind
    }

    pub fn define(&mut self, symbol: Symbol) -> bool {
        let normalized = normalize_name(&symbol.name);
        if self.symbols.contains_key(&normalized) && !self.builtin_names.remove(&normalized) {
            return false;
        }
        let mut symbol = symbol;
        symbol.name = normalized.clone();
        self.symbols.insert(normalized, symbol);
        true
    }

    pub fn lookup(&self, name: &str) -> Option<&Symbol> {
        let normalized = normalize_name(name);
        if let Some(symbol) = self.symbols.get(&normalized) {
            Some(symbol)
        } else if let Some(parent) = &self.parent {
            parent.lookup(name)
        } else {
            None
        }
    }

    pub fn lookup_local(&self, name: &str) -> Option<&Symbol> {
        let normalized = normalize_name(name);
        self.symbols.get(&normalized)
    }

    pub fn lookup_mut(&mut self, name: &str) -> Option<&mut Symbol> {
        let normalized = normalize_name(name);
        if self.symbols.contains_key(&normalized) {
            self.symbols.get_mut(&normalized)
        } else if let Some(parent) = &mut self.parent {
            parent.lookup_mut(name)
        } else {
            None
        }
    }

    /// Mark a symbol as used. Traverses parent scopes if not found locally.
    /// Returns true if the symbol was found and marked, false otherwise.
    pub fn mark_used(&mut self, name: &str) -> bool {
        let normalized = normalize_name(name);
        if let Some(symbol) = self.symbols.get_mut(&normalized) {
            symbol.used = true;
            true
        } else if let Some(parent) = &mut self.parent {
            parent.mark_used(name)
        } else {
            false
        }
    }

    /// Records a `أرجع` expression's type at the nearest enclosing
    /// `Function`/`Lambda` scope. Silently drops the type if called with no
    /// such enclosing scope — callers are expected to have already checked
    /// `is_in_function()` before reaching this point.
    pub fn record_return(&mut self, ty: Type) {
        if matches!(self.kind, ScopeKind::Function | ScopeKind::Lambda) {
            self.inferred_returns.push(ty);
        } else if let Some(parent) = &mut self.parent {
            parent.record_return(ty);
        }
    }

    /// Drains the return types collected for the nearest enclosing
    /// `Function`/`Lambda` scope. Used by block-bodied lambda inference to
    /// fold `أرجع` statement types into the lambda's overall return type.
    pub fn take_inferred_returns(&mut self) -> Vec<Type> {
        if matches!(self.kind, ScopeKind::Function | ScopeKind::Lambda) {
            std::mem::take(&mut self.inferred_returns)
        } else if let Some(parent) = &mut self.parent {
            parent.take_inferred_returns()
        } else {
            Vec::new()
        }
    }

    /// Looks up `name`, but stops at the nearest enclosing `Function`/
    /// `Lambda` boundary instead of continuing past it. Used by capture
    /// detection (issue #180) to tell a lambda's own params/locals (visible
    /// without crossing a function boundary) apart from an outer local that
    /// would have to be captured across one.
    pub fn lookup_in_current_function(&self, name: &str) -> Option<&Symbol> {
        let normalized = normalize_name(name);
        if let Some(symbol) = self.symbols.get(&normalized) {
            Some(symbol)
        } else if matches!(self.kind, ScopeKind::Function | ScopeKind::Lambda) {
            None
        } else if let Some(parent) = &self.parent {
            parent.lookup_in_current_function(name)
        } else {
            None
        }
    }

    /// Returns the `ScopeKind` of whichever scope in the chain first defines
    /// `name`, without regard for function boundaries. Used alongside
    /// `lookup_in_current_function` to classify a capture: a name defined at
    /// `Global` or `Class` scope is never a capture (issue #180).
    pub fn defining_scope_kind(&self, name: &str) -> Option<ScopeKind> {
        let normalized = normalize_name(name);
        if self.symbols.contains_key(&normalized) {
            Some(self.kind)
        } else if let Some(parent) = &self.parent {
            parent.defining_scope_kind(name)
        } else {
            None
        }
    }

    /// Whether the current scope is (transitively) inside an arrow lambda's
    /// body, stopping the moment a declared function's own body is reached —
    /// so a `دالة` declared inside a lambda's body is not itself considered
    /// "inside a lambda" for capture-detection purposes.
    pub fn in_lambda_body(&self) -> bool {
        match self.kind {
            ScopeKind::Lambda => true,
            ScopeKind::Function => false,
            _ => self
                .parent
                .as_ref()
                .is_some_and(|parent| parent.in_lambda_body()),
        }
    }

    pub fn is_in_loop(&self) -> bool {
        if self.kind == ScopeKind::Loop {
            true
        } else if matches!(self.kind, ScopeKind::Function | ScopeKind::Lambda) {
            // `أوقف`/`استمر` can only break a loop in the same function
            // frame — a loop outside the enclosing function/lambda body is
            // not a valid target (the IR builder isolates its loop_stack
            // across function boundaries for the same reason).
            false
        } else if let Some(parent) = &self.parent {
            parent.is_in_loop()
        } else {
            false
        }
    }

    pub fn is_in_function(&self) -> bool {
        if matches!(self.kind, ScopeKind::Function | ScopeKind::Lambda) {
            true
        } else if let Some(parent) = &self.parent {
            parent.is_in_function()
        } else {
            false
        }
    }

    pub fn is_in_class(&self) -> bool {
        if self.kind == ScopeKind::Class {
            true
        } else if let Some(parent) = &self.parent {
            parent.is_in_class()
        } else {
            false
        }
    }

    pub fn get_function_return_type(&self) -> Option<Type> {
        if matches!(self.kind, ScopeKind::Function | ScopeKind::Lambda) {
            self.return_type.clone()
        } else if let Some(parent) = &self.parent {
            parent.get_function_return_type()
        } else {
            None
        }
    }

    pub fn pop(self) -> Option<Scope> {
        self.parent.map(|p| *p)
    }

    pub fn symbols(&self) -> impl Iterator<Item = &Symbol> {
        self.symbols.values()
    }

    /// Get all symbols from this scope and all parent scopes.
    pub fn all_symbols(&self) -> Vec<&Symbol> {
        let mut all: Vec<&Symbol> = self.symbols.values().collect();
        if let Some(ref parent) = self.parent {
            all.extend(parent.all_symbols());
        }
        all
    }

    /// Names this scope chain resolves to a *user* declaration rather than a
    /// built-in — main's own declarations plus whatever it imported.
    ///
    /// This is the visibility the IR builder must agree with. It sees the
    /// linked AST, which carries every merged module declaration under its bare
    /// name whether or not it was imported, so re-deriving "is this declared?"
    /// from that AST makes a module's private `اطبع` outrank the built-in in a
    /// program that never imported it.
    pub fn user_defined_names(&self) -> HashSet<String> {
        let mut names: HashSet<String> = self
            .symbols
            .keys()
            .filter(|name| !self.builtin_names.contains(*name))
            .cloned()
            .collect();
        if let Some(ref parent) = self.parent {
            names.extend(parent.user_defined_names());
        }
        names
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scope_lookup() {
        let mut global = Scope::new_global();
        global.define(Symbol::variable("x", Type::Int, true, Span::default()));

        let child = Scope::new_child(global, ScopeKind::Block);

        assert!(child.lookup("x").is_some());
        assert!(child.lookup("اطبع").is_some());
        assert!(child.lookup("nonexistent").is_none());
    }

    #[test]
    fn test_scope_shadowing() {
        let mut global = Scope::new_global();
        global.define(Symbol::variable("x", Type::Int, true, Span::default()));

        let mut child = Scope::new_child(global, ScopeKind::Block);
        child.define(Symbol::variable("x", Type::String, true, Span::default()));

        let symbol = child.lookup("x").unwrap();
        assert_eq!(symbol.ty, Type::String);
    }

    #[test]
    fn test_loop_detection() {
        let global = Scope::new_global();
        let func = Scope::new_child(global, ScopeKind::Function);
        let loop_scope = Scope::new_child(func, ScopeKind::Loop);
        let block = Scope::new_child(loop_scope, ScopeKind::Block);

        assert!(block.is_in_loop());
        assert!(block.is_in_function());
    }

    #[test]
    fn test_function_return_type() {
        let global = Scope::new_global();
        let func = Scope::new_function(global, Type::Int);

        assert_eq!(func.get_function_return_type(), Some(Type::Int));
    }

    #[test]
    fn test_function_return_type_from_nested_scope() {
        let global = Scope::new_global();
        let func = Scope::new_function(global, Type::String);
        let block = Scope::new_child(func, ScopeKind::Block);
        let inner_block = Scope::new_child(block, ScopeKind::Block);

        assert_eq!(inner_block.get_function_return_type(), Some(Type::String));
    }

    #[test]
    fn test_function_return_type_void() {
        let global = Scope::new_global();
        let func = Scope::new_function(global, Type::Void);
        let loop_scope = Scope::new_child(func, ScopeKind::Loop);

        assert_eq!(loop_scope.get_function_return_type(), Some(Type::Void));
    }

    #[test]
    fn test_no_return_type_in_global_scope() {
        let global = Scope::new_global();
        let block = Scope::new_child(global, ScopeKind::Block);

        assert!(block.get_function_return_type().is_none());
    }
}
