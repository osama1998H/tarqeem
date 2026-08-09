# AI Implementation Notes

Decisions and discoveries recorded by AI-assisted sessions, newest first.

## 2026-08-09 — Issue #184: core OOP broken (inherited methods, مشترك statics, upcasting)

Highest-impact pick from the #180–#187 usability-audit backlog: three
textbook class-system patterns (§9.5–9.7 of LANGUAGE_SPEC.md) were all
broken — `موظف.تحية()` crashed with "دالة غير معرّفة" when only the parent
defined it, `عدادات.المجموع` crashed with "معرّف غير معرّف" for `مشترك`
members, and `متغير ش: شكل = جديد مربع(5)` was a flat type error.

### Decision: runtime `class_id` walk, not a vtable

Three independent Explore/Plan passes converged on rival designs; the
largest required a new `ClassHierarchy` abstraction threaded through 20
`IrBuilder::new` call sites plus native vptr object-layout changes. Checked
independently first: `NewObject` already stores the object's true runtime
class, and `Class.parent` was already fully populated before execution.
That made a single `virtual_dispatch: bool` field on `Instruction::CallMethod`
sufficient — true for ordinary calls, false only for `الأصل.م()` super calls
(which must stay statically bound, or an override's super call recurses
into itself). The interpreter resolves virtual calls by walking the
object's actual `class_id` up `Class.parent` until a matching `FuncId` is
found (`resolve_virtual_method`, mirrored in the DAP interpreter). This one
change fixes inherited calls, template-method dispatch through `هذا`, *and*
upcast dispatch simultaneously — dispatch never even looks at the
declared/static type, so once assignability (below) allows the upcast to
compile, correct dispatch falls out for free.

### مشترك statics: reuse the existing global-variable path

`مشترك` fields/methods had no representation as a namespace at the IR
level — `عدادات.المجموع` fell into `build_identifier`, which only knows
locals/params/functions/globals. Fixed by lowering static fields/properties
to ordinary IR globals keyed `"{Class}::{member}"` (`register_static_global`
in `ir/builder/mod.rs`) and adding a `class_name_receiver` check before the
six call sites that used to assume every bare-identifier receiver was a
variable (`build_call`, `build_member`, `build_assignment`, and
`build_compound_assignment` in `expr_builder.rs`; the `Member`/`Call` arms
of `infer_expr_type` in `type_helpers.rs`). Caught along the way:
`__global_init__` (for non-const initializers) was invoked by the
interpreter only — native codegen never
called it, so a `مشترك` array field silently stayed null in compiled
binaries. Fixed by calling it from `__main__`'s prologue in codegen.rs,
which incidentally fixes the same latent gap for ordinary non-const globals.

### Upcasting: parameterize, don't duplicate, the compatibility check

`Type::is_compatible_with` had no `(Class, Class)` arm. Renamed its body to
a private `compat(&self, other, resolver: Option<&ClassResolver>)`, kept
`is_compatible_with` calling it with `None` (byte-for-byte identical, so
the ~10 non-assignment call sites — `==`, override variance, generic
constraints — are provably untouched), and added `is_assignable` calling it
with `Some(resolver)` for the one new `(Class(sub), Class(super))` arm.
Only assignment-position call sites (var init, call/ctor args, field
defaults, etc.) were switched to `is_assignable`. The ternary operator
needed a real join instead of a plain swap: naively using `is_assignable`
would infer `شكل ? جديد شكل() : جديد مربع()` as `مربع` (the narrower type),
which is unsound — it now tries both directions and takes the wider one.

Deliberately excluded: interface-typed slots. `resolve_member_type`
collapses interface members to `Type::Any` and `implements_interface`
doesn't walk `extends` chains yet, so allowing an interface upcast now
would trade a compile error for a runtime crash — filed as a follow-up
instead of allowed here.

### Native (LLVM) codegen: scope boundary, and a guard that was tried and reverted

Native dispatch was deliberately left untouched — it still always binds
`CallMethod` to `method.class`'s own body (no vtable). `مشترك` statics *are*
fixed natively too (they reuse the pre-existing global path). For
inheritance/upcasting, native's pre-existing behavior is unchanged, but its
*character* shifted for the upcast case specifically: before this fix the
program didn't type-check at all (compile error); after, it compiles and
silently runs the ancestor's method instead of the override (e.g. prints
`0` instead of `25`) — a worse failure mode (silent-wrong vs. loud-reject).

Tried fixing this with a codegen-time guard: precompute `(ancestor_class,
method)` pairs where some descendant overrides the method, and reject any
`CallMethod` matching one of those pairs as unsupported-in-native (new code
ت٠٢٠٢). Reverted after a native-compile sweep of all 18 `examples/*.ترقيم`
found a false positive: `examples/صنف.ترقيم` calls `شخص١.اطبع_معلومات()`
where `شخص١`'s declared *and* runtime type are both `شخص` — no upcast
anywhere in the file — yet the guard rejected it purely because `موظف`
(unrelated to this call site) overrides the same method name. Telling
"this call is monomorphic" from "this call could reach an override via an
upcast somewhere" needs value-flow analysis, which is exactly the kind of
invasive change the minimal-diff approach was chosen to avoid. Disclosure
(this note + a follow-up issue) instead of a coarse guard that breaks a
currently-working canonical example.

### Verification: red-proved the new tests, not just green-checked them

Added `tests/oop_execution_tests.rs` (25 execution tests, stdout-asserting
via both interpreter and JIT — addressing part of #187's "tests never
execute programs" gap for this feature area). Confirmed they aren't
vacuously true by running the identical file against a `git worktree` of
the pre-fix commit: 16/25 failed there with exactly the errors this fix
addresses (9 pre-existing regression-guard tests, e.g. downcast rejection,
already passed). Also ran a full native-compile sweep of `examples/*.ترقيم`
(0/18 failures) to confirm no regression outside the new tests.

### xhigh code review (before commit): 12 confirmed defects, fixed

An xhigh-effort multi-agent review of the full diff (before it was
committed) found 12 defects that survived independent verification. Most
severe: `Type::compat`'s Optional-unwrapping arm
(`(t, Optional(inner)) | (Optional(inner), t) => t.compat(inner, ...)`)
silently swapped which operand was the value and which was the slot
whenever `self` (not `other`) was the Optional-wrapped one — direction
matters for the new Class-subtype arm (and even pre-existing arms like
`Int → Float`), so this both rejected valid `مربع? → شكل?` upcasts *and*
silently accepted the unsound reverse direction. Fixed by splitting it
into two direction-preserving arms. A related but separate bug: the
ternary widening join (added earlier in this same PR) picked whichever
branch's type satisfied `is_assignable` first, silently dropping the
other branch's `?` — fixed locally in `infer_ternary_expr` by re-wrapping
the join result in `Optional` when either branch was Optional, rather
than touching `compat`'s Optional semantics globally (which `==`/`!=`
comparisons elsewhere depend on staying symmetric).

Also confirmed and fixed: array/map literal type inference anchored on
the *first* element instead of folding to the widest type, so
`[جديد مربع()، جديد شكل()]` was rejected while the reverse order compiled
— same class of order-dependence as the ternary, fixed the same way
(widen-in-place fold instead of anchor-on-first). And a genuine
encapsulation bypass this PR's combination of features newly exposed:
`resolve_virtual_method` dispatches on the object's *runtime* class
without re-checking the resolved override's visibility, but
`check_method_overrides` (pre-existing, in `class_resolver.rs`) only ever
blocked narrowing an override to `خاص` — never `عام → محمي`. So
`م.تحية()` through a `شخص`-typed reference to a `موظف` instance would
statically check against `شخص`'s public method, then dynamically resolve
to `موظف`'s protected override. Fixed at the root — generalized the
existing check to reject *any* visibility-narrowing override, not just
narrowing to private — rather than trying to re-check visibility at every
virtual-dispatch call site.

Lower-severity fixes: a cycle guard added to `resolve_virtual_method`
(mirroring the `visited`-set pattern the static-lookup helpers already
use); a `(String, String) → FuncId` cache added to the same function (was
an uncached linear scan per inheritance level, on every instance method
call); `مشترك` static initializers were silently dropped for class-only
files with neither top-level code nor `دالة رئيسية` (the drain condition
missed that third shape); the ص٠٥٠١/ص٠٥٠٢ doc examples didn't match the
actual emitted message text; this file underclaimed the class-name-
receiver check landed at five call sites when the diff actually touched
six. Three new regression tests were added for the ternary/array fixes
and the weak `test_static_method_call_with_args_and_return` assertion
(field and method result shared the same value, so a method call that
accidentally resolved to the field's global would still have passed) was
tightened to use distinguishable values.

One review finding was corrected rather than "fixed": `jit_stdout`'s doc
comment claimed it exercised the dispatch fix identically to the
interpreter, but `JitConfig::default()`'s `baseline_threshold: 100` means
none of these short test bodies ever promote past Tier-0, so both test
legs run the same interpreted path. Probed whether lowering the threshold
would make the test suite meaningfully exercise Cranelift-compiled
`CallMethod` handling — it doesn't: profiling only counts calls to
`__main__` itself (called once per test), not to the methods inside it,
so no realistic threshold promotes them. Left the JIT path as-is and
corrected the doc comment instead of chasing a fix that would require
building actual Cranelift `CallMethod` support — that's the same
already-tracked native/JIT dispatch gap from the #185 follow-up, not
something this PR's scope should absorb.

### Follow-ups filed separately (out of scope here)
Every item below was re-verified empirically against the built compiler
before filing — not carried over from the plan's notes unchecked. One
planned item (inherited *field* access from a subclass method body) did
**not** reproduce: retested with `محمي`/`عام` fields (the original repro
had mistakenly used `خاص`, which is correctly rejected — private means
private even to subclasses) and both worked fine. No issue filed for it.

- #185 (comment): native virtual/polymorphic dispatch — vtable population,
  symbol mangling, vptr object layout.
- #209: interface-typed variable assignment/dispatch (`ميثاق`-typed slots).
- #211: `جديد` on a class with no constructor crashes at runtime instead of
  compile-erroring.
- #212: return statements and member/index assignment targets are never
  type-checked.
- #213: `check`'s "N error(s) found" count includes warnings; class methods
  called only via `obj.method()` syntax always warn as unused.
- #214: dead code — `MethodResolver::resolve_method_call`,
  `ClassResolver::implements_interface` (zero callers; the latter is
  relevant to #209's fix).

## 2026-08-09 — Code-review fixes on the #193/#194/#198 bundle: 6 findings

A high-effort automated review of the bundle below found 6 confirmed issues.
Two design agents proposed fixes at different scopes; the deciding factor was
measuring rather than guessing.

### Decision: measure real usage before sizing the AST change
One agent proposed extending `leading_comments`/`trailing_comments: Vec<String>`
across ~8 AST types (`Block`, `ClassDecl`, `InterfaceDecl`, `EnumDecl`, `Match`,
`ClassMember`, `MethodSignature`, `EnumVariant`, `PropertyAccessor`, `Ast`).
A scan of all 65 real `.ترقيم` files in `stdlib_trq/`+`examples/` found the
speculative patterns (comment before a class/interface/enum/match/accessor
body's closing `}`, or before `الحمد_لله`) occur **0 times** — but surfaced a
**worse, previously unreported** bug with hard evidence: 92 real lines across 7
stdlib files (`مجموعات/{مكدس,طابور,قائمة,قاموس,مجموعة}`, `وقت/{وقت,تاريخ}`)
where a `//` section-banner comment before a class method gets silently
*relocated* into the method's body instead of staying above it — `tarqeem fmt`
was structurally corrupting real, shipped code. Sized the fix to the evidence:
two new AST fields, not eight.

### `Block.dangling_comments` (issue #205, resolved) + `ClassMember.leading_comments`
`Block` gained a field for comments between the last statement and `}`
(covers every construct `parse_block` builds — function/if/while/for/do/try/
lambda/accessor/brace-match-arm bodies — so it closes #205 for every real
occurrence, not just "function bodies" as originally filed). `ClassMember`'s 4
variants gained `leading_comments`, fixing the 92-line relocation bug: a
banner comment before a method was being collected into the shared
`pending_comments` buffer and, since nothing drained it before the method's
own `parse_block` ran, ending up attached as the *first statement inside the
method* instead of staying above it.

### Root cause of the misattachment (issue not previously filed under its own number)
`pending_comments` (`Parser` field) is pushed to by `collect_line_comments()`
at 5 sites but drained by exactly 1 (`parse_declaration`). The other 4 —
`parse_class_member`, the property-accessor/interface-method/enum-variant
loops — leaked a collected comment forward to whatever unrelated
`parse_declaration()` call happened next. Fixed with one choke point
(`pending_comments.clear()` inside `match_terminator_after_trivia`'s
confirmed-terminator path, safe because that field is only ever legitimately
non-empty in the two-line window between `collect_line_comments()` and
`take_pending_comments()`) plus 3 local one-line drains for paths the choke
point can't reach.

### What stayed out of scope, deliberately
Comments at the tail of a class/interface/enum body's member list, an
accessor list, a match-arm list, or immediately before `الحمد_لله` are still
silently dropped — measured at 0 real occurrences, not fixed. Restoring a
diagnostic there (the review's finding #3) would just reintroduce bug #198;
`Parser::parse` returns `Err(self.errors[0].clone())` regardless of
`DiagnosticLevel`, so even a warning-level diagnostic would hard-fail the
parse (needs the return-type change already tracked as #206).

Multi-line trailing block-doc-comments (`/** a\nb */`) were also corrupted by
the formatter (one `//` prefix for possibly multi-line content, so
continuation lines became bare, unparseable code) — fixed with a
`write_comment_lines` helper that re-prefixes every line, reused for the new
`Block.dangling_comments` emission too (which can also come from a multi-line
`BlockDocComment` — the same bug would have reappeared there otherwise).

## 2026-08-09 — Issues #193/#194/#198: comment handling & error-masking bundle

### Root cause: one defect, three symptoms
All three issues traced to the same underlying gap and were fixed together
deliberately — fixing any one alone made the others *worse* (verified by
tracing, not assumed): a parser-only fix for #194 (trailing `///`/`/** */`)
left the doc-comment token unconsumed in `capture_trailing_comment`, so it
got misattributed as the *next* declaration's doc comment instead of erroring
loudly. All seven repros from the three issues printed the identical masked
`متوقع 'الحمد_لله' في نهاية الملف` message before this fix (#193 is why the
other two were invisible as distinct bugs).

### Decision: one shared trivia-skipping helper, not seven patches
Seven statement-list loops wait for a `RightBrace`/`Alhamdulillah` terminator
and can meet a comment first; only three had a guard, and all three matched
`LineComment` only. Added two `pub(crate)` methods in `src/parser/parser/mod.rs`:
`check_terminator_after_trivia` (non-consuming lookahead over
`Newline`/`is_comment()` tokens) and `match_terminator_after_trivia`
(consuming version for loop heads — only consumes trivia once the terminator
is confirmed, so a leading comment for the *next* real declaration is never
stolen). Reused the existing `TokenKind::is_comment()` helper
(`src/lexer/token.rs`), which had zero production callers before this.

### Lexer fix was required, not optional
`scan_doc_comment` merges consecutive `///` lines without checking whether
the *first* `///` began its own line. Once the parser accepts a trailing
`///` as a valid statement terminator, that merge bug becomes silent doc
theft: `س() {} /// نبذة` immediately followed by `/// وثيقة ب` +
`دالة ب(){}` would lex as one `DocComment` token misattributed to `ب`. Gated
the merge on a backward line-start scan; also stopped the terminate branch
from swallowing the trailing newline (restore captured position/line/column
instead of jumping past it), so a comment token is now always followed by a
`Newline`, matching `scan_line_comment`/`scan_block_doc_comment`.

### #193: the "infinite loop" risk from adding RightBrace to synchronize() was never real
Verified by tracing, not by initial instinct (an earlier read of the issue
assumed the risk was real and designed defensively around it): `parse_prefix`
(`expr_parser.rs`) calls `advance()` before it can return `Err`, so every
`parse_declaration` failure path already consumes ≥1 token — `synchronize()`
can never re-park on the same index. Added `RightBrace` as a sync point
anyway (matches its two siblings, `synchronize_to_member`/`synchronize_to_arm`,
which already had it) and a cheap progress guard in `parse()`'s loop as
documentation of the invariant, not as a load-bearing fix.

### Verification method: don't trust a predicted baseline, measure it
Used `git stash` to rebuild the pre-fix binary and ran the same stdlib parse
sweep before trusting a subagent-reported "34 files fail" baseline — it was
correct, and the post-fix count (33, one file newly parseable:
`stdlib_trq/ملفات/مجلد.ترقيم`) was confirmed a strict subset via `comm`, zero
regressions. Also ran a `tarqeem fmt` diff sweep (pre- vs post-fix output,
byte-identical across all 31 previously-parseable files) and an idempotence
check (`fmt(fmt(x)) == fmt(x)`) — 11 files failed idempotence, but `git stash`
confirmed that failure pre-exists on `develop` too (root cause: the
already-known `///`-marker-stripping formatter bug, filed as #201 — the
"twice" pass fails to re-parse its own output, unrelated to this bundle).

### Deliberately out of scope
`consume_doc_comment` still does not accept `BlockDocComment` (`/** */`
before a declaration is a hard parse error) — attaching it would feed more
content into the formatter's `///`-stripping bug (#201), converting a loud
error into silent corruption. Fix both together. Filed follow-ups: #201
(fmt strips `///` + `BlockDocComment` unhandled), #202 (type keywords
rejected as method names — the real remaining blocker on `مشغل.ترقيم`, not
comment-only bodies as previously documented), #203 (a `//` banner
immediately after a leading `///` module doc fails — the real first blocker
on `http.ترقيم`, surfacing *before* the already-known #197 `منشئ_كامل` issue
since it occurs earlier in the file), #204 (doc comments on `صدّر`/`استورد`
silently dropped), #205 (`Block` has no field for a comment-only body, so
`tarqeem fmt` silently drops one — **fixed**, see the 2026-08-09 code-review
entry above), #206 (parser returns only `errors[0]`, unlike semantic
analysis), #207 (marker diagnostics carry no error code).

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
- Two new parser bugs found during investigation, filed separately as
  #193 and #194 — both **fixed**, see the 2026-08-09 entry above.

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
