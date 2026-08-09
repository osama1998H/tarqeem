# AI Implementation Notes

Decisions and discoveries recorded by AI-assisted sessions, newest first.

## 2026-08-09 — Issue #183: احصل/عيّن/حالة become contextual keywords

### Decision: parser-level contextual keywords, lexer untouched
The lexer keeps emitting `Get`/`Set`/`Case`; the parser maps them back to
identifier strings everywhere except their reserved contexts (خاصية accessor
blocks, تطابق arm heads). New helper `identifier_like_name(&Token) ->
Option<&str>` in `src/parser/parser/mod.rs` mirrors the existing
`expect_type_name` pattern (type keywords as names). It returns the token's
lexeme, so the user's spelling is preserved: عين (no shadda) and عيّن both lex
to `Set` but are distinct identifiers, exactly as they would be if they
weren't keywords — stdlib's `دالة عيّن` must be called with the same spelling.
(An earlier draft normalized عين→عيّن; the code review flagged that as silent
identifier renaming — `tarqeem fmt` would rewrite user source — so it was
dropped. The lexeme is safe to use directly because the lexer NFC-normalizes
the entire source in `Lexer::new`.) Chosen over de-reserving in the lexer
because it leaves property/match parsing, LSP highlighting, and lexer tests
untouched.

Widened sites: `expect_identifier`, `check_identifier`, `expect_type_name`
(mod.rs); `parse_prefix` (identifier handling hoisted to an early return
before the match, single source of truth for the kind set) +
`try_parse_arrow_params` + `try_parse_type_args`'s `looks_like_type` gate
(expr_parser.rs); `parse_pattern` enum lookahead (stmt_parser.rs). Safe
because `Precedence::of` returns `None` for these kinds (Pratt loop stops
before an arm-head `حالة`) and both reserved contexts dispatch on the token
kind before any identifier path. Error recovery updated to match:
`synchronize_to_member` resumes at Get/Set/Case (and خاصية, a pre-existing
omission); `synchronize_to_arm` treats `Case` as the next arm head only at a
plausible arm start (after Newline/LeftBrace/comment — comment tokens swallow
their trailing newline), since mid-line حالة is now an identifier use.
Beware: `Parser::previous()` indexes `current - 1` and panics at position 0 —
guard any `previous()` call reachable at the stream start.

### Discoveries — remaining stdlib parse blockers are NOT this bug
After the fix, `مجموعات/قائمة.ترقيم` and `مجموعات/قاموس.ترقيم` parse. Files
still failing have independent causes:
- `اختبار/نتائج.ترقيم:27` — enum variant named `خطأ` collides with the
  boolean-false keyword (separate collision, not #183's keywords).
- `شبكة/http.ترقيم:77` — `منشئ_كامل(...)` named-constructor member syntax
  is not supported by the parser at all.
- `اختبار/مشغل.ترقيم:257` — a function body containing only a line comment
  fails to parse (same family as #194's comment handling).
- Importing قائمة still can't `جديد قائمة()` (د٠٠٠٣): imported classes are
  registered as scope Symbols only, never into `class_resolver` — that is
  issue #182's import machinery, not the keyword collision.
- `TARQEEM_HOME` (set on this machine) shadows the repo `stdlib_trq` in
  `find_stdlib_path`; unset it when verifying stdlib changes with the CLI.

## 2026-08-07 — Issue #186 part 2: hoist top-level functions and enums

### Decision: hoist in the analyzer only
Forward references failed only in semantic analysis — the IR builder already
hoists function signatures (`src/ir/builder/mod.rs`, "First pass") and the
interpreter resolves by `FuncId`. Fix lives entirely in `src/semantic/`:
`Analyzer::analyze` now runs `hoist_enum_decl` then `hoist_func_decl` loops
between `register_types` and `add_type_members`.

### Design constraints discovered (worth knowing for future passes)
- **Enums must hoist before functions**: `resolve_type` only returns
  `Type::Enum` for names already in `self.enums`; otherwise `parse_type_name`
  falls back to an incompatible `Type::Class(name)`. A function signature
  mentioning a later enum would silently get the wrong type.
- **`Scope::define` returns false on any existing key**, so pass 3 must not
  re-define hoisted symbols or every top-level function reports د٠١٠١.
  Discriminator: `self.scope.kind() == ScopeKind::Global` — only top-level
  pass-3 statements and `Export(Declaration(_))` reach the declaration
  analyzers with the global scope current. Nested functions keep
  define-in-place and are deliberately NOT hoisted.
- **The hoist pass must unwrap `StmtKind::Export(ExportItems::Declaration)`**
  because `analyze_export` analyzes the inner declaration with the global
  scope current — without unwrapping, exported functions would never be
  defined at all. Note `register_types` does NOT unwrap Export (pre-existing
  gap for exported classes, left untouched: fixing it requires also fixing
  `add_type_members` or vtable validation breaks).
- Duplicate detection for top-level functions/enums moved into the hoist
  pass — still exactly one د٠١٠١ per collision, verified by count-asserting
  tests. Known diagnostic shift: `متغير س` + later `دالة س` now reports
  against the variable (the hoisted function claims the name first).

### Related discoveries
- Part 1 of #186 (bare أرجع) was already fixed by `97c0673`.
- Two new parser bugs found during investigation, filed separately:
  the end-marker check runs before the accumulated-error check so
  `synchronize()` overshoot masks real errors behind
  `متوقع 'الحمد_لله' في نهاية الملف`; and a trailing doc comment (`///`)
  after any statement fails to parse (lexer folds the newline into
  `DocComment`; terminator checks only match `LineComment`).

## 2026-08-07 — CI unblock + usability audit

### CI failures: two root causes
1. Commit `06958ae` ("fix") added an unformatted `eprintln!` to
   `src/debug/interpreter/mod.rs`, failing `cargo fmt --all -- --check`.
   The **Lint & Format** job gates every other CI job (`needs: lint`), so
   the whole pipeline skipped on main and on every PR branched from it.
2. CI installs the *latest stable* Rust (unpinned). Clippy lints drift with
   every release: local 1.92 passed while CI 1.97 failed with 36 new lints
   (`redundant reference in write! args`, `manual_checked_ops`,
   `unnecessary_sort_by`, collapsible match arms, `result_large_err`).
   **Lesson: run `rustup update stable` before trusting local clippy, and
   always use CI's exact invocation (`--all-targets --all-features -- -D warnings`).**

### Verified language bugs fixed (PR #179)
- REPL fed raw input to the file parser, which mandates the
  بسم_الله/الحمد_لله markers → every REPL line failed. Fixed by
  auto-wrapping input (`wrap_repl_input` in `src/cli/commands/mod.rs`).
- `runtime-rs` printed false as `خاطئ`; the language literal (and the
  interpreter/JIT rendering) is `خطأ`. Aligned `trq_print_bool` and
  `trq_bool_to_string`.
- LLVM codegen: `Phi` results were never registered in `var_types`, so
  `GlobalStore` defaulted them to i64 — a string ternary initializing a
  global emitted `store i64 <ptr>` (invalid IR). Phi now registers its type.
- IR builder: implicit عدد→عدد_عشري coercion (spec §5.6) was missing for
  initializers. Globals kept `Constant::Int` under an f64 type (invalid
  `global double 5`); locals stored the raw i64 bit pattern into a double
  slot (printed ~2.47e-323 natively). Added `coerce_int_to_float` +
  constant coercion in the global collection pass.
- LANGUAGE_SPEC §6.6's own ternary example used the reserved keyword
  `حالة` as a variable name; example renamed to `وضع`.

### Key architectural discoveries
- **بسم_الله/الحمد_لله are deliberate mandatory file markers**
  (`Ast::with_markers`); do not remove them — align docs/tools instead.
- `tarqeem run` = interpreter (default) / `--jit`; native is `tarqeem compile`.
  There is no `--interpret` flag (CLAUDE.md corrected).
- The linker resolves `libtrq.a` via `TARQEEM_RUNTIME_PATH` →
  `CARGO_MANIFEST_DIR/target/{release,debug}` → `~/.tarqeem/lib`. When the
  compiler binary runs *outside* cargo, `CARGO_MANIFEST_DIR` is unset and a
  **stale installed runtime** in `~/.tarqeem/lib` silently wins. Rebuild it
  with `cargo build --release -p tarqeem-runtime` and re-install after
  runtime changes.
- Most tests in `tests/runtime_rs_e2e_tests.rs` only run semantic analysis
  (`analyzes_ok`) — they never execute programs, which is why broken
  features kept 1,300+ tests green (issue #187).

### Open confirmed bugs (multi-agent audit, adversarially verified)
Filed as issues #180–#187: lambdas/first-class functions unfinished at IR
stage; exception system (استثناء class missing, propagation, `_trq_throw`);
local module imports; stdlib collections keyword collision (احصل/عيّن —
fixed by #183, contextual keywords);
core OOP (inherited methods, مشترك access, upcasting); native divergences
(طول bytes-vs-chars, نوع symbol, stdlib segfault, نص? IR); parser blockers
(bare أرجع, forward references). Raw audit: 40 findings across 6 lenses;
10 verified, 10/10 confirmed, 0 refuted.
