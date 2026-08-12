# AI Implementation Notes

Decisions and discoveries recorded by AI-assisted sessions, newest first.

## 2026-08-12 — examples/ consolidated 21 → 10, and the CI matrices stopped being hand-written

### The corpus was mostly duplication

`جمع`/`طرح`/`ضرب`/`قسمة`/`مضروب`/`فيبوناتشي` were each redefined in four or five
files. `اختبار_بسيط` was a strict subset of `حاسبة`; `لعبة_الحياة_بسيط` a stub of
`لعبة_الحياة` whose neighbour count was a fake (it counted edges, not neighbours);
`ضغط` already imported `تشفير` and re-demonstrated all of `بصمة`. Each file cost
three CI matrix runs, so the duplication was paid for on every push.

The new rule is **one example per language area**: `مرحبا`, `أساسيات`, `دوال`,
`أصناف`, `تعداد`, `صياغة`, `حاسبة`, `لعبة_الحياة`, `تشفير_وضغط`,
`اختبار_اطار_العمل`. A new feature adds a section to an existing file; only a
genuinely new area adds a file.

Renames, for anyone following an old link: `دوال_سهمية` → `دوال`,
`صنف`/`وراثة`/`خواص`/`رؤية_بسيط` → `أصناف`,
`متغيرات`/`تحكم`/`اختبار_مجموعات` → `أساسيات`,
`وضع_البرنامج`/`أسطر_متعددة` → `صياغة`, `بصمة`/`ضغط` → `تشفير_وضغط`.

### What was preserved deliberately, not incidentally

Three of the deleted files were regression witnesses, not demos, and their exact
shapes had to survive the merge:

- **`وراثة`** exists because `صنف` avoids the #249 bug *by accident* — its
  subclass assigns only its own fields, so inherited reads happen inside parent
  methods where `هذا` is already the parent type. Its three-level hierarchy, its
  reads through the derived class, and its deliberately-absent `م.مساحة()` call
  (#253) are now a marked section at the end of `أصناف`, not folded into the
  surrounding style.
- **`خواص`** is the #239 witness — two auto-properties that must keep distinct
  slots. `نقطة(3، 4)` still prints `3` then `4`.
- **`دوال_سهمية`** claimed every form in it works in all three backends, and
  `LANGUAGE_SPEC.md` states that as an invariant. The merged `دوال` keeps every
  lambda form, its inline expected-value comments, and the ت٠٣٠١/د٠٣٠٦ limits
  footer; the named-function half was renamed `اجمع` to avoid colliding with the
  existing `جمع` lambda.

Dropped on purpose: the stray `صنف محمود` that sat at the end of the old `دوال`,
unrelated to functions and never instantiated.

### The one accepted coverage loss

`compare-backends` carried `KNOWN_DIVERGENT: "ضغط:native"` for #185 — native
`طول` counts UTF-8 bytes where the interpreter counts characters. `بصمة` had no
divergence and was fully output-diffed natively; merging the two puts the whole
file behind the allowlist, so SHA-256's native output is no longer diffed.

Taken knowingly. The alternative — dropping the `طول()` prints so the merged file
agrees — deletes the only CI witness for an open bug, which the allowlist's own
comment forbids: entries are removed when the issue is *fixed*, not when the
evidence is removed. Verified the divergence is confined to the three `طول`
lines; the digests and the gzip round-trip agree on all three backends.

### The matrices were the real maintenance cost

`examples.yml` ran three hand-maintained 21-entry matrices. That is the same
class of hazard as the `assert!(parseable >= 66)` floor below: a hand-kept number
standing in for a fact the tool could compute. It had already failed —
`رؤية_بسيط` sat in `examples/` while being in none of the three matrices, so it
was never run at all.

Replaced with a `list-examples` job that emits the names from the same glob
`check-examples` uses, consumed via `fromJSON`. Adding an example no longer
touches the workflow. The job errors on an empty glob, because a `[]` matrix
spawns zero jobs and reports success — indistinguishable from every example
passing.

Two adjacent fixes in the same file: `compare-backends` was missing from the
`summary` job's `needs:`, so the aggregate summary omitted the only job that
validates output; and `fail-fast: false` was preserved on all three matrices.

### `mod examples` was three-quarters vacuous

`tests/integration_tests.rs` named four example files, each wrapped in
`if path.exists()`. Two of the four were on the delete list — and the guard means
deletion produces a *passing* test, not a failing one. The other seventeen
examples were never covered there. Replaced with a directory walk that asserts
every `examples/*.ترقيم` parses, plus a non-empty assertion so the walk cannot go
vacuous the same way one level up.

`src/fmt/formatter.rs` needed no change: its corpus guard became a set
(`KNOWN_UNPARSEABLE`) in `fd8636a`, so deleting eleven files no longer trips it.

### Verification

All 10 examples were run under interpreter, JIT and native with
`env -u TARQEEM_HOME` (the variable silently shadows this checkout's `stdlib`),
and each backend's stdout diffed against the interpreter's. Exactly one
divergence: `تشفير_وضغط:native`. Merged outputs were also diffed against the
concatenated pre-merge baselines — `مرحبا` is byte-identical to old
`مرحبا` + `أهلا`.

## 2026-08-12 — Deleting four examples broke CI through a magic number, not the matrix

### The failure was not where the deletion was

`0fd3d83` removed `examples/planned/`, `examples/wasm/` and the `examples/حاسبة/`
package, four of which were `.ترقيم` files. The obvious suspect was
`.github/workflows/examples.yml`, which names examples one per matrix entry — but
the Examples workflow **passed**. Its `أهلا` and `حاسبة` entries resolve to
`examples/أهلا.ترقيم` and `examples/حاسبة.ترقيم`, top-level files that still
exist; only the same-named *directory* went away. Diffing the matrix against
`examples/*.ترقيم` confirms 21/21 with nothing stale on either side, so nothing
needed removing from CI at all.

What failed was `Full Test Suite` / `Test fmt`, on one line of
`src/fmt/formatter.rs`: `assert!(parseable >= 66)`. The corpus walk had gone from
67 parseable files to 63.

### The guard was measuring the wrong thing

That floor exists to stop a parser regression hiding behind the test's skip
branch: files that fail to parse are skipped rather than asserted on, so a
regression that made files unparseable would silently shrink coverage and leave
the test vacuously green. A bare count does detect that — but it cannot tell it
apart from a corpus that legitimately shrank, and it needs a human to re-baseline
the number every time either happens. Four deleted examples and a real parser
regression are the same event to it.

Verified it was the former before touching the number: all four deleted files
still parse under today's parser (extracted from `0fd3d83^`, run through
`tarqeem parse`), and the single non-parsing file today is
`stdlib/أخطاء/فهرس.ترقيم` — the already-allowlisted #243 blocker. 63 + 4 = 67 ≥
66, so nothing regressed.

So the fix is not `66` → `63`. The count is now an exact **set** compared against
a `KNOWN_UNPARSEABLE` allowlist, mirroring the two lists this repo already keeps
(`tests/integration_tests.rs`, and the `ALLOWED` guard in `examples.yml`) and
following their rule that the list may only shrink. An unexpected entry names the
regressing file instead of reporting a number that drifted; a missing entry means
#243 is fixed and says which three places to drop it from. Adding or deleting
examples no longer touches this test.

### Found in passing: `ليس`/`!` in infix position spins the Pratt loop (#266)

The negative test that proves the new guard bites — drop an unparseable file into
`examples/` and confirm the assert fires — used
`هذا ليس كوداً صالحاً ((( ` as the bad input and **hung** `cargo test` past 600s.

Two guesses died on the way to the answer, both worth recording because both were
plausible and both were *wrong in the same way*: reasoning from syntax rather
than from the parser's own tables.

1. **"The unclosed `(((`."** From LANGUAGE_SPEC §4.6.1 — a newline inside an open
   bracket is not a statement terminator, so an unclosed one leaves nothing to
   resynchronise on. Disproved: unclosed `(`, `[` and `{` all exit 1 promptly.
2. **"A dangling NOT with no operand."** Bisecting to `س ليس` made this look
   right. Disproved by one more test: `س ليس ص` *has* an operand and still hangs.

What actually matters is the operand **before** the NOT — the token is in *infix*
position:

    س ليس        hangs      ليس       exits 1
    س !          hangs      !         exits 1
    س ليس ص      hangs      س ⏎ !     exits 1
    اطبع(س ليس)  hangs      س و/أو/+/-  exits 1

`src/parser/precedence.rs:50` scores `Bang` as `Precedence::Unary`, so the Pratt
loop in `parse_precedence` does not break on it and calls `parse_infix`. But
`parse_infix` has no `Bang` arm and falls through to `_ => Ok(left)`, which
returns **without advancing**. Same token, same precedence, forever. `Bang` is
the only token in that state: everything else `Precedence::of` scores has an
infix arm that advances (`PlusPlus`/`MinusMinus` are handled as postfix inc/dec),
and prefix-only tokens scoring `None` break out.

The generalisable lesson is about the catch-all, not about NOT: a Pratt `parse_infix`
whose fallback returns without consuming turns any future precedence-without-arm
into a hang rather than an error.

Not confined to `parse` — `check` and `run` hang identically, so the LSP is
exposed too: typing `س !` mid-expression can hang the language server on a
keystroke. Distinct from #234 (an IR-builder terminator bug where a method ending
in `تطابق` loops at *runtime*); this one never leaves the parser. Filed as #266,
pre-existing and unrelated to this change — not fixed here.

## 2026-08-12 — Issue #259: `صدّر` hid a declaration's members from two passes

### One of five analyzer passes did not unwrap `صدّر`

`Analyzer::analyze` walks the top-level statements five times. Four unwrapped
`صدّر` — `register_types`, `hoist_enum_decl`, `hoist_func_decl` and the third
pass via `analyze_export` — and `add_type_members` did not. So an exported class
was registered with an **empty member table**: `جديد` reported
`الصنف 'س' ليس له منشئ`, every field and method reported ص٠٣٠١ (including from
inside the class's own method bodies), and the emptiness propagated into
subclasses' vtables, so an *un-exported* child of an exported parent failed too.

The module-side twin `add_module_type_members` had unwrapped since #182, which is
why the bug was invisible from the consumer side: importing an exported class
worked, and only the file that *declared* it was broken. That asymmetry is worth
remembering — a pass duplicated for "main" and "modules" can drift, and here the
main copy was the stale one. `git log -L` shows `add_type_members` never
unwrapped; `register_types` gained its unwrap during #181 and the sibling was
missed.

### The unsound symptom was the interface one

Every other consequence was a spurious error on correct code. But
`add_type_members` also feeds `add_interface_methods`, so `صدّر ميثاق` registered
**zero methods** — leaving `ClassResolver::validate` nothing to require and
silently suppressing ص٠٢٠١. A class that ignored an exported contract compiled
clean. Its regression test is therefore the one that had to be watched failing in
the *opposite* direction: analysis succeeded where it must fail.

### Nothing downstream needed a change

`Export` reaches the IR builder intact for main's statements —
`link_program` carries them verbatim (`linker.rs:145-153`) and only `disposition`
strips the wrapper, and only for modules. What compensates is
`as_top_level_decl` (`ir/builder/mod.rs:159`), used by every top-level scan, plus
`build_stmt`'s recursion through `ExportItems::Declaration`. Verified rather than
assumed: `--dump-ir` output for a class with and without `صدّر` is **byte-identical**,
and the interpreter/JIT/codegen consume `ir::Module`, in which `Export` does not
exist. So the defect was confined to the analyzer, and the fix is one line.

### The same defect, independently, in the LSP

`collect_symbols` (`lsp/analysis/document.rs`) matched the raw statement too, so
every exported declaration was missing from the symbol table — no hover, no
go-to-definition, no member or enum-variant entries, on exactly the declarations
a library publishes. That file already had its own `unwrap_exported_decl` for the
entry-point check; only `collect_symbols` was not using it. Three copies of this
one-line helper now exist (linker, IR builder, LSP), each deliberate for layering
reasons — which is precisely why they drift. `lsp/handlers/{completion,folding,inlay_hints}.rs`
still do not unwrap; filed separately.

### Why 1,300+ tests missed it

Every exported-class fixture was in a safe bucket: parse-only
(`phase3_criteria_tests.rs::test_export_class`), `{}` with no members to lose
(`ir/builder`'s fixtures), or module-side, i.e. the path that already worked
(`module_execution_tests.rs::test_imported_class_constructs_and_reads_field`).
The one main-file fixture with real members asserts ص٠٦٠٢, which fires in pass 1
before members matter. Systemically: CI's `check-stdlib` job runs `tarqeem parse`,
never `check`, and no example uses `صدّر صنف` — so 20 stdlib files could not be
`check`ed and nothing reported it.

### Measured effect on the stdlib

`check` errors across all 43 stdlib files: **1270 → 764**.
`مجموعات/قائمة.ترقيم` goes 62 → 0; طابور، مكدس، مجموعة، متكرر and
اختبار/نتائج each drop to 1; وقت/وقت 169 → 71.

One file *rises*: `اختبار/توكيدات.ترقيم` 68 → 159. Not a regression — analysis now
gets far enough to surface the consequences of #243. `stdlib/أخطاء/فهرس.ترقيم`
does not parse (`صدّر صنف خطأ` names a class with a reserved token), so
`خطأ_تأكيد` cannot be imported, so `خطأ_توكيد_مفصل يرث خطأ_تأكيد` has no parent
and is not an `استثناء` subclass — hence 25 fresh ص٠٦٠١ on its `ارمِ` statements.
A file that did not compile still does not compile; the error count is not the
metric there.

Left alone deliberately: #231 (static member access on *imported* classes, still
ص٠٥٠٢ after this fix — re-verified), and the generic-substitution gap that makes
a `ن`-typed constructor argument fail as `ن٠٠٠١ متوقع ن، وُجد عدد` with or without
`صدّر`. The generic regression test takes an `عدد` parameter to avoid masking
itself on that.

## 2026-08-12 — Review round on #259: turning a check on revealed that the check was wrong

A high-effort review of the #259 fix produced 10 findings. The important lesson:
**enabling a dormant check is not the same as enforcing a correct one.** The first
write-up of #259 claimed the newly reachable ص٠٢٠١ would "surface pre-existing
bugs". It surfaced pre-existing *false positives* instead.

### `check_interface_implementations` compared types by name

`صدّر ميثاق` had registered zero methods, so contract conformance was never
checked from the export side. Registering the methods activated
`ClassResolver::check_interface_implementations`, which compares
`expected_ty != actual_ty` exactly. Two shapes the language explicitly allows
cannot survive that:

- **`أي`.** Specified as "يقبل أي نمط" (LANGUAGE_SPEC §5.5). The spec's own §9.7
  example, `ميثاق قابل_للمقارنة { دالة قارن(آخر: أي) -> عدد }`, was rejected for
  every concrete implementor.
- **An unsubstituted type parameter.** `ميثاق حاوية<ن>` resolves `ن` through
  `parse_type_name`, which yields `Type::Class("ن")` — not `Type::Generic` — and
  `InterfaceInfo` has no `type_params` field at all, so the resolver cannot even
  tell `ن` is a variable. Every implementor of a generic contract was rejected.

`check_method_overrides` had the same defect via exported generic *parents*.

Fix: `ClassResolver::is_unconstrained` — a declared type that cannot be enforced
by name (`أي`, `Type::Generic`, or a `Type::Class` naming neither a registered
class nor a registered interface, recursing through containers) imposes no
requirement. The "unknown `Class` name is a type variable" heuristic avoids
plumbing `type_params` into `InterfaceInfo`; a genuinely misspelled type is still
reported as د٠٠٠٣ where the name is used.

Note this repaired the **un-exported** case too — `ميثاق حاوية<ن>` without `صدّر`
was equally broken on develop, and `stdlib/مجموعات/متكرر.ترقيم` (which declares
`صدّر ميثاق متكرر<ن>`) went from 1 error to 0.

### The prelude guard has to be in both passes, not just one

`register_types` refuses a redefinition of `استثناء` and returns *without
registering*, so the entry under that name stays the prelude's. Once
`add_type_members` unwrapped `صدّر`, it called `add_class_members("استثناء", …)`
on that surviving prelude entry and replaced `fields`/`constructor` wholesale —
so the one correct ص٠٦٠٢ refusal arrived with two bogus errors pointing at
correct `ارمِ`/`خ.رسالة` code. The fix's own comment said the two passes "must
agree on what counts as a declaration"; they agreed on unwrapping and disagreed
on the guard. The prelude's own members arrive via `add_module_type_members`, so
skipping in `add_type_members` costs nothing.

### A flat symbol map only stayed honest while it was half-empty

`collect_symbols` inserts each function's parameters into
`HashMap<String, SymbolInfo>` unconditionally. Harmless while exported
declarations were skipped; once they are collected, every function in an
all-exported module — i.e. every stdlib module — contributes parameters, and a
parameter named `س` displaces a top-level `صدّر ثابت س`. That converts silence
into a confidently wrong hover. Changed to `or_insert_with`, so a real top-level
symbol always wins.

### Left open deliberately

- A constructor-less exported class now passes `check` and crashes at run time on
  `عداد::منشئ` (#211). Pre-existing; the un-exported form crashes identically.
  This fix removed the bogus compile-time gate that hid it, and synthesizing a
  default constructor is an IR-side change.
- A `خاصية` cannot satisfy a `ميثاق`'s `دالة` contract, because
  `add_class_members` registers a property as `__احصل_<name>` and never under the
  declared name. Same behaviour for un-exported interfaces on develop; whether a
  property *should* satisfy a method contract is a language question, not a bug
  to patch mid-fix.
- Exported enum variants land under `::`-joined keys, which the outline's
  `'.'`-only nesting filter renders flat. Cosmetic, pre-existing for un-exported
  enums.
- `صدّر صدّر صنف س` parses into nested `Export(Export(…))`, and all four unwrap
  helpers are single-level, so the class becomes invisible and is reported as the
  misleading `د٠٠٠٣ صنف غير معروف`. The parser should reject a repeated `صدّر`.

## 2026-08-11 — Issue #228: 33 of 43 stdlib files did not parse

### The issue listed three causes; there were nine

#228's "observed causes" section named banner comments, `استورد *` and `صنف خطأ`.
A per-file static scan that reproduces **exactly** the 33 failures found nine, and
the parser reports only the first error per file (#206), so six were invisible
behind the comment error. Two of the nine had no issue at all and are now #255
(a call argument list cannot span lines) and #256 (`2.5e10` does not lex).

Technique worth reusing: to see past the first-error mask **without** changing the
compiler, copy the tree, strip every comment-only line and rewrite the barrels
textually, then re-parse. That forecast 25/43 and named the four hidden categories
before a line of Rust changed; the real fix landed on 25/43 at the same point.

### `Ast.module_doc`: sized by measurement, not by symmetry

The naive #203 fix — loop the trivia collection, keep the last doc, demote the
rest — would have rewritten 22 stdlib module headers from `///` to `//` on
`fmt -w`, because `format_leading_trivia` renders `leading_comments` with
`write_comment_lines`. The measurement that decided the design: **all 22 orphan
doc blocks in the corpus sit on line 3**, immediately after `بسم_الله`; zero
mid-file. So one additive `Ast` field, not the eight-node trivia refactor
`AI_NOTES` already rejected once (see the #205 entry).

`doc_comment_is_module_header` hoists a leading doc when a *nearer doc* follows,
when nothing follows, or when what follows owns no `doc_comment` field. That third
clause repaired three pre-existing silent losses nobody had filed: a doc above the
first `استورد` was demoted to `//` (ملفات/مجلد، مجموعات/فهرس) and a doc above a
bare `صدّر *` was dropped outright (اختبار.ترقيم lost all five `///` lines).

**Do not widen it to "any nearer comment".** That reading made `/// وثيقة` +
`// ملاحظة` + `دالة` hoist the doc away from the function it documents. The
distinction that works: a *declaration* with no doc field means the doc is the
file's; executable code means it belongs to that statement and keeps demoting.

A demoted doc is re-inserted at `doc_position`, not appended. Appending moved a
demoted `/// وثيقة` below a `//` written after it — `fmt` reordering a run the
user wrote.

### Keyword-in-name: a separate helper, and why widening the old one breaks

`identifier_like_name` also backs `check_identifier`, which is the *name-or-type
disambiguator* for enum-variant payloads (`decl_parser.rs:646`) and
`looks_like_type` (`expr_parser.rs:488`). Adding the type keywords there makes
`مصفوفة<عدد>` parse as a field named `مصفوفة` with a stranded `<عدد>`. Hence
`declaration_name`/`variant_name` as siblings. Pinned by
`test_generic_type_in_enum_variant_payload_still_parses_as_a_type`.

The `::` fence is what makes `خطأ` safe as an enum variant and unsafe as a class
name: a variant is only ever reached through `::`, while `جديد خطأ()` parses its
callee as a primary expression and yields `Literal(Bool(false))`. That is the
reason #243 must rename, not relax.

Declaring the name is half the job: without the `.`-position site
(`expr_parser.rs:368`) a method named `عدد` parses and stays uncallable. And
مصفوفة/قاموس/أي were missing from `parse_prefix`, so a parameter named `مصفوفة`
was declarable and unreadable in its own body.

### Newline-as-trivia needs the *operand* side too

`bracket_depth` + `within_brackets`, consulted in `parse_precedence`. The
non-obvious part: skipping newlines only before the *operator* is not enough —
`parse_infix` recurses into `parse_precedence` for the right operand, which lands
on the newline and calls `parse_prefix` on it. Both sides of the loop, or wrapped
`&&` chains still fail. Depth is restored on the error path, or one malformed
argument list joins every following statement to the next line.

`إذا`/`طالما`/`افعل-طالما`/`تطابق` consume their own `(`, so they need
`within_brackets` explicitly; the array-literal loop was the existing pattern all
of this copies.

### Two float bugs the fmt corpus guard caught, not review

Assembling a float as `mantissa * 10^exp` overflows to infinity for f64::MAX
(`1.7976931348623157e308`, رياضيات/ثوابت.ترقيم) and loses precision per digit.
`scan_number` now rebuilds the literal with ASCII digits — which is also what
makes Arabic-Indic floats (`٢٥.٥`) work — and defers to `str::parse::<f64>`.
Then the formatter printed floats with `Display`, expanding f64::MAX to 309
digits that re-parsed as an out-of-range *integer*; `{:?}` is the shortest
round-tripping form.

Both only surfaced because `test_format_repo_corpus_is_reparsable_and_idempotent`
skips unparseable files, so the moment ثوابت.ترقيم parsed it entered the corpus.
Its floor is now 66 parseable files (was 33) — without raising it, the guard
stays vacuously green for everything this work unblocked.

### Renaming Latin stdlib identifiers is not mechanical

رياضيات/اساسي.ترقيم called the runtime by `pow`/`sqrt`/…, and for **six** of the
fourteen the wrapper's own name *is* the runtime's Arabic name — so the "rename to
Arabic" instruction produces `دالة جذر { أرجع جذر(س) }`, infinite recursion, a
silent hang. Those six pass-throughs are removed (documented in the module header);
`قوة` calls the typed `قوة_عدد`; and قاسم_مشترك/مضاعف_مشترك/عاملي are implemented
in Tarqeem, which keeps the API and drops a runtime dependency.

Verifying those three exposed #257: a user function named after a runtime builtin
collides at the LLVM symbol level (`قاسم_مشترك` → `trq_gcd`), so it runs in the
interpreter and JIT and fails to link natively. Pre-existing on `develop`.

### What a newly-parsing file does next

`tarqeem check` on a stdlib module *in isolation* now reports its own runtime calls
as د٠٠٠١ معرّف غير معروف, because builtins are registered per importing module
(`Scope::get_stdlib_builtin`) and a direct check has no import. Not a regression —
نص/اساسي.ترقيم, untouched here, behaves the same — and not something #228 can fix;
it is the same ground as #229.

## 2026-08-11 — Issues #249 and #250: inherited members resolved to nothing

### The issue's diagnosis was inverted, and the empirical check was cheap

#249 reported that "the subclass's LLVM struct type contains only its *own* fields"
and concluded the struct had to be flattened. The opposite is true, and one grep
settles it:

```
$ tarqeem compile ب.ترقيم --emit-llvm -o ب.ll && grep '= type' ب.ll
%class.أصل = type { i64 }
%class.فرع = type { i64, i64 }        # flattened correctly
```

`emit_class_definition` → `collect_class_fields` already recurses through
`Class.parent` (populated at `stmt_builder.rs:347-354`), and `NewObject` already
sizes `trq_alloc` from the flattened list — so the allocation-size hypothesis is
disproven too. **Codegen was correct throughout; nothing in `src/codegen/` changed.**

Worth keeping as a technique: `%class.فرع` having *zero* fields and having *one*
field produce the **identical** `invalid getelementptr indices` from clang, so the
reported symptom could not discriminate between "struct too short" and "index too
large". Reading the emitted struct is what does.

### Root cause: a missing walk behind three silent fallbacks

`FieldId`/`MethodId` mean *(defining class, index own-relative to that class)* —
the convention `stmt_builder.rs:516-521` documents and codegen's
`inherited_field_count[field.class] + field.index` implements. But nothing walked
the parent chain: `get_field_info` searches one class's own fields, and the
property accessor lookups keyed flat maps on the *receiver's* class, unlike their
`مشترك` twins `resolve_static_property{,_setter}`. On a miss, `build_member` /
`store_to_member` substituted `index: 0`, type `Ptr(Void)`, and the receiver as
owning class. Three wrong values in one instruction, visible directly in the IR:

```
setfield %0, %class.فرع.قيمة, %1        # قيمة is declared on أصل
%4: *void = getfield %3, %class.فرع.قيمة  # and it is an عدد
```

Hence two unrelated-looking symptoms from one defect:

- **`invalid getelementptr indices`** when the subclass declares no fields of its
  own — `inherited_count[فرع] (1) + 0` against a one-slot struct.
- **SIGSEGV** when it does — both ctor writes computed slot 1 and aliased, which is
  in-range so clang accepted it; the *lost type* is what crashed, because `Print`
  dispatches on `Ptr(_)` to `trq_print(ptr %x)` (`codegen.rs:1650-1656`) and
  dereferenced the integer `6`.

This is the second bug from this fallback pattern after #239, whose own fix
recorded the rule being applied here: *a missing backing field is an error, not a
fallback to 0 — falling back is the bug.* The fallbacks are now
`unknown_member_error`.

### #250 was the same bug, seen from the read side

#250 ("`الأصل(...)` loses a parent-constructor write to an auto-property") is fixed
by this change, with no `الأصل`-specific code involved — and its title is a
mis-diagnosis. The write was never lost: objects are `Rc<RefCell<TrqObject>>` and
argument passing clones the `Rc`, so no aliasing bug is possible. What happened is
a **name** mismatch. Inside `أصل::منشئ`, `هذا` is typed `أصل`, so the write routed
through the setter and stored the backing field `_قيمة` correctly; the read through
a `فرع`-typed reference missed the accessor lookup and degraded to a raw `GetField`
named `قيمة` — a name no slot carries — so the interpreter returned `Null`.

That also explains the issue's whole isolation table, including why two of its four
rows "work": a child writing the inherited auto-property itself degrades on *both*
sides to the same wrong name, so interpreted it cancels out. It was never right,
only symmetrically wrong — and natively it still addressed the wrong slot.

### Strictness boundary: "known class" ≠ "has a layout"

Making resolution failure a hard error looked safe and was not. Object literals are
typed `Struct(ClassId("__anonymous__"))`, a class `collect_class` never registers
because codegen resolves its fields by name — so the receiver *looks* known while
the lookup necessarily fails. Gating on `class_fields.contains_key` rather than
`class_id_opt.is_some()` is what keeps `سجل.اسم` compiling; `أي`-typed and
unresolved receivers (imported symbols degrade to `أي`, #229) keep the lenient path
for the same reason. `test_object_literal_member_read_stays_lenient` guards it and
passes with *and* without the fix — it is a boundary guard, not a bug test.

### Why 1,373 tests and a full CI examples matrix missed it

`tests/oop_execution_tests.rs` covers inheritance well but is in-process with **no
native leg**, and native codegen is the only backend that honours `field.index`.
`compare-backends` (added for #239) would have caught it, but *no example program
read an inherited member through a subclass-typed reference* — `examples/صنف.ترقيم`
has the child assign only its own fields, with inherited reads happening inside the
parent's own methods where `هذا` is typed as the parent. The gap was in the corpus,
not the harness. `examples/وراثة.ترقيم` closes it, and
`tests/inheritance_execution_tests.rs` adds 8 three-backend fixtures (7 of which
fail without the fix). `property_execution_tests.rs::test_accessor_indices_are_own_class_relative`
noted it *had* to assert IR indices because this bug blocked running the program;
it now has an executing sibling.

### Discovered while testing, filed separately

- **#253** — inherited **method** calls have the identical defect and are *not*
  fixed here: `build_method_call` names the receiver in `MethodId.class`, so native
  emits a call to `@{subclass}::{method}`, which is never defined, and the missed
  `method_return_types` lookup leaves the result `Ptr(Void)`. Kept out to keep this
  change reviewable; `examples/وراثة.ترقيم` deliberately avoids the shape and says
  so. Any realistic inheritance program hits it, so it is the natural next fix.
- Object literals still lose their **second** member natively (`سجل.عمر` after
  `سجل.اسم`). Verified pre-existing against a stashed build; belongs with #185.
- `compare-backends` is still missing from the `summary` job's `needs:`
  (`.github/workflows/examples.yml`), so the aggregate summary does not reflect it.
- `رؤية_بسيط` was in `examples/` but in none of the three hand-maintained CI
  matrices — added alongside `وراثة`. The glob-based `compare-backends` job is the
  one that actually keeps up.

## 2026-08-11 — Issue #239: auto-property accessors all addressed field slot 0

### The issue's own diagnosis was wrong twice over

#239 reported `examples/خواص.ترقيم` printing `4` where the interpreter prints `3`,
and guessed "an off-by-one or last-write-wins in the **native** field index".
Both halves are wrong, and the difference matters:

- **It is an IR-builder bug, not a codegen bug.** `build_property_getter` and
  `build_property_setter` (`src/ir/builder/stmt_builder.rs`) emitted
  `GetField`/`SetField` with a literal `index: 0`, never calling `get_field_info`.
  The IR is wrong for every backend; codegen is merely the only consumer that
  *honours* the index. `src/interpreter/executor/mod.rs` destructures the index
  away and keys on `field.name`, which is why the interpreter looks correct and
  why "native divergence" was a misleading frame.
- **Not an off-by-one — always 0.** So all auto-properties on a class share one
  slot *and* a write through any of them overwrites whatever real field occupies
  index 0.

The discriminating experiment, worth keeping as a technique: interleave plain
fields with auto-properties and give each a distinct value. `plain=11, auto=22,
plain=33, auto=44` printed `44 44 33 44` natively — the plain field at index 0
reads back a value written through an auto-property, while the plain field at
index 2 is untouched. An off-by-one predicts garbage in the last slot instead.
Reading indices out of the built IR then confirmed it directly.

### Why 1,300+ tests and a full CI examples matrix missed it

`خواص` is in CI's compiled-examples matrix and *ran* there every push — but
`.github/workflows/examples.yml` only checked exit codes, and no workflow
compared output at all. The one property test that executes anything
(`oop_execution_tests.rs::test_static_auto_property`) misses twice: `مشترك`
properties use the index-free `GlobalLoad`/`GlobalStore` path, and it has no
native leg. Note the JIT leg proves nothing here either — Tier-0 is the
interpreter, and short programs never promote to Cranelift.

Fixed by a new `tests/property_execution_tests.rs` (all three backends, exact
stdout) plus a `compare-backends` CI job that diffs interpreter against native
output for every example, with an explicit `KNOWN_DIVERGENT` allowlist. The
allowlist checks both directions: a listed example that starts *agreeing* also
fails, so entries cannot rot after their issue is fixed. It holds only `ضغط`
(#185, native `طول` counts bytes).

### A missing backing field is an error, not a fallback to 0

`backing_field_index` returns `Err(IrError)` rather than `unwrap_or(0)`.
Defaulting to slot 0 *is* the bug being fixed; a silent fallback would let the
next layout change reintroduce it invisibly.

### Discovered while testing, filed separately — not fixed here

- **#250 — `الأصل(…)` loses a parent-constructor write to an auto-property.**
  Fails in the *interpreter*, so it is unrelated to field indices. Parent
  constructed directly works; a child writing the inherited property itself
  works; `الأصل` with a plain field works. Only the exact combination drops it.
- **#249 — inherited instance fields are unusable natively.** The subclass's
  LLVM struct type omits its parent's fields, so codegen's
  `inherited_count + index` GEPs out of bounds: `invalid getelementptr indices`
  when the child declares no own fields, SIGSEGV when it does. Reproduces with
  plain fields and no properties. `examples/صنف.ترقيم` escapes it only because
  its child constructor touches just its own fields and inherited reads happen
  inside the parent's own methods.
- **#251 — field/property defaults are never applied.** `خاصية قيمة: عدد = 7`
  reads as `لا_شيء` interpreted and `0` natively — both wrong, and divergent.
  `build_new` emits no instance field/property initializers at all; the native
  `0` is just `alloc_zeroed`, so any non-zero default is silently wrong.

Consequence for test design: the inherited-auto-property case cannot be asserted
end to end today, so the own-class-relative index convention is asserted against
the built IR instead of through a running program.

### Code review caught two defects the fix itself created or left behind

An xhigh review of the PR found 10 findings; the two that mattered:

**1. Fixing the accessors was not enough — compound assignment bypassed them.**
`build_compound_assignment`'s member arm duplicated the plain-assignment store
with a bare `SetField` carrying an empty class, the property's *own* name instead
of its `_`-prefixed backing field, and its own hardcoded `index: 0`. So `ن.ص += 1`
still produced two different wrong answers: the interpreter stored a by-name
field no getter reads (the `+=` vanished), while native wrote slot 0 and
corrupted the *other* property. The lesson generalises — the same defect can sit
in a second copy of the same logic, so the fix was to extract one
`store_to_member` both paths call, not to patch the constant twice.

**2. The new hard error broke working programs.** `collect_class` runs over
top-level statements only, so a class declared inside a function or block has no
entry in `class_fields` — and `backing_field_index` turned that into a build
failure for a program that previously ran, while `tarqeem check` still reported it
clean. Fixed at the cause: `build_class_decl` now collects the layout on demand,
which also registers the class in `module.classes` so nested classes work
natively rather than merely not crashing. (Instantiating one is still rejected by
the analyzer with د٠٠٠٣ — a separate, pre-existing limitation.)

Guarding against reintroduction cost two more things worth keeping:

- `assert_prints` compares raw stdout byte for byte across backends *in addition*
  to the trimmed line-based expectation. `lines()` normalises whitespace, so on
  its own it cannot see a backend that prints `"3 "`, adds a blank line, or drops
  the trailing newline — the same silent-divergence class.
- The `compare-backends` job asserts each run's **exit status**, diffs the **JIT**
  as well as native, and keys `KNOWN_DIVERGENT` per `example:backend`. Discarding
  exit codes meant two backends broken enough to print nothing produced two empty
  files and "agreed"; a per-example allowlist would have excused a future JIT
  regression in `ضغط`, whose bug (#185) is native-only. A separate
  `NATIVE_UNSUPPORTED` list covers examples native codegen *documents* refusing
  (`ارمِ` → ت٠٣٠٣), which `KNOWN_DIVERGENT` cannot: it is only consulted once a
  binary exists.

## 2026-08-10 — Issue #181: the exception system, from unusable to usable

`ارمِ` could not be used in **any** program. `Analyzer::is_error_type` accepts
`استثناء` or a subclass and the catch parameter was already typed
`Type::Class("استثناء")` — both correct, both **inert**, because no `استثناء`
class was ever registered and there was no mechanism anywhere to predeclare one.
The stdlib declared the hierarchy under the name `خطأ`
(`stdlib/أخطاء/فهرس.ترقيم:21`), which is `TokenKind::False` and cannot parse
as a class name — so there was no reachable way to make anything throwable.

Four independent root causes, one per layer:

### 1. Semantic: the class did not exist → an injected prelude module

`src/semantic/prelude.rs` holds `صنف استثناء` as embedded Tarqeem source, parsed
and inserted into `ModuleLoader` under the synthetic path `<تمهيد ترقيم>`.

Registering it directly in `ClassResolver` would not have been enough:
`جديد استثناء(…)` needs an object layout and a constructor body in the `Ast` that
`IrBuilder::build` consumes. Going through the module cache reuses the entire
#182 pipeline — `register_module_types` puts the class in the hierarchy ahead of
`build_vtables`, `add_module_type_members` attaches `رسالة`, and `link_program`
merges the declaration into the program AST — so one insertion serves the
interpreter, the JIT and native codegen. Both of those consumers iterate
`loader.modules_in_load_order()`, the raw cache, not the import graph, which is
why a module nothing imports is still picked up.

Embedded rather than read from `stdlib/`: the LSP and DAP have no stdlib
search path (#230), and a prelude that can go missing at run time takes `ارمِ`
with it.

Two consequences of always having one module in the cache:
- `link_program`'s empty-cache fast path (`main.clone()`) never fires now, so
  every program pays the merge. Left as is: the merge is one pass over a
  three-statement AST plus a `HashMap` insert per declared name, judged
  negligible against parse and analysis. **Not measured** — `benches/end_to_end.rs`
  was not run, so if compile-time regressions ever surface, start here.
- `register_class` is a `HashMap::insert`, so a user's own `صنف استثناء` would
  have replaced the base class *silently* while `link_program` merged both
  declarations into the IR. The name is now reserved (ص٠٦٠٢). The four semantic
  tests that used to hand-declare `استثناء` are exactly the evidence this
  mattered: they passed for years on top of that silent overwrite.

Also: `analyze_throw` now returns early on `Type::Error`. A failed `جديد` already
reported د٠٠٠٣, and the follow-up rendered as `لا يمكن رمي نوع غير خطأ 'خطأ'` —
`Type::Error::arabic_name()` is `خطأ` — reading as if the user had thrown a
boolean. The message itself now names `استثناء` and carries ص٠٦٠١.

### 2. IR: the catch parameter was bound as a slot, not a value

`build_try` inserted the parameter into `variables` but not `parameters`, so
`build_identifier` treated it as an alloca and emitted `Load` on a non-pointer:
`متوقع ptr، وُجد object` on *any* use of `خ`, not just field access.
`add_pattern_bindings` had this right for `تطابق` bindings all along.

It also needed `var_types[exception_var] = Struct(ClassId("استثناء"))`. Without
it, member access took `build_member`'s unknown-class branch, `خ.رسالة` came back
as `Ptr(Void)`, and `"…" + خ.رسالة` lowered to *integer* addition — a fix that
looked complete and still printed the wrong thing.

Two more defects in the same function: `TryBegin` was emitted even with no
`التقط`, registering a handler whose whole body was a jump past the exception
(so `حاول { ارمِ … } أخيراً { … }` silently discarded it); and `TryEnd` + `Jump`
were appended unconditionally after a body ending in `ارمِ`, which
`has_terminator` classifies as a terminator — instructions after a terminator are
invalid LLVM IR. Both blocks are now created only for the clauses that exist, and
every join jump is guarded by the new `current_block_needs_terminator`.

### 3. Interpreter: unwinding stopped at the throwing frame

`try_stack` lives on the `CallFrame`, so a callee that found no handler could
only return a Rust `Err`, and `?` at the four call sites carried it past every
enclosing `حاول`. One call level was enough to defeat `التقط`.

The payload is now parked in the existing `current_exception` slot before the
`Err`, and all four call sites route through `finish_call`, which converts an
`ErrorKind::UnhandledException` back into `InstructionResult::Throw` so the
caller's own handler stack is consulted. It falls back to propagating the `Err`
when the slot is empty, so a genuine interpreter failure is never swallowed.

An uncaught exception now reports its `رسالة` rather than `<استثناء>`.

`src/debug/interpreter/` needed the identical port. It duplicates the main
interpreter's instruction handling (issue #223) — including its own
`DebugCallFrame.try_stack` and its own four call sites — so the IR-level fixes
reached it for free while the propagation fix did not. Left alone, `tarqeem debug`
would abort on an exception `tarqeem run` catches. Verified by
`test_debug_interpreter_catches_exception_from_callee`, which drives
`DebugInterpreter` directly (the DAP wire protocol needs a client).

### 4. Native: not broken — unimplemented, and now refused

`TryBegin`/`TryEnd` lower to LLVM *comments*, the `catch.N:` block is emitted
with **zero predecessors**, and `@trq_throw`/`@trq_get_exception` are declared but
defined nowhere in `runtime-rs` (`nm libtrq.a` confirms). There is no `invoke`,
no `landingpad`, no `personality`, no setjmp. Real native EH is a design project,
so `build_throw` now blocks native lowering with ت٠٣٠٣ instead. Design deferred to #238.

The block keys on `Instruction::Throw` **only**. A `حاول`/`التقط`/`أخيراً` with
nothing thrown compiles and runs natively today, and
`test_try_catch_finally_without_throw_runs_natively` guards it. That case links
only because LLVM discards the unreachable catch block — which still contains a
call to the undefined `@trq_get_exception` — before the reference reaches the
linker. **Nothing in `codegen.rs` was touched for this reason**: deleting the
declaration would make the module unparseable, and erroring the `GetException`
arm would fail every try/catch program.

`Function::native_block_reason: Option<String>` became
`native_block: Option<NativeBlock>` carrying its own message and code. The old
shape hard-coded ERR_UNTYPED_LAMBDA_PARAM and wrapped every reason in advice to
"declare concrete types" — wrong for a construct no annotation can fix.

Discovery along the way: `CodegenError` has carried a `code` since ت٠٣٠١ but
`compile.rs` dropped it, so **no codegen error code had ever been printed** and
none could be passed to `tarqeem اشرح`. One-line fix; ت٠٣٠١ and ت٠٣٠٢ become
visible too.

### Deliberately left undone

- `أخيراً` does not run when an exception propagates out of a frame, nor on an
  early `أرجع` from a try body. Expressing it needs a `Rethrow` the IR does not
  have. Documented in LANGUAGE_SPEC §11.4, filed as #242.
- The six spec'd `استثناء_*` subclasses, and the spec's two-constructor
  `استثناء` (Tarqeem has no constructor overloading). Both recorded as spec
  deviations rather than silently ignored; the stdlib's `أخطاء` module, which
  still declares the hierarchy as the unparseable `خطأ`, is #243.
- The JIT's Cranelift compilers used to *skip* exception instructions via
  `_ => {}`, i.e. compile the function with the throw deleted. They now raise
  `JitError::unsupported_instruction`. Latent today — `JitExecutor` always
  delegates to the interpreter — but a miscompile the moment dispatch is wired.

### Verification

`tests/exception_execution_tests.rs`, 16 cases — 15 through the real binary, one
driving `DebugInterpreter` directly. Mutation-verified: removing the
`parameters.insert` fails 5, reverting the cross-frame routing fails 4, removing
the native block fails 8, reverting the debug-interpreter port fails 1.

All 19 `examples/*.ترقيم` were run under all three backends and their outputs
cross-compared. Two diverge natively — `ضغط` (`طول` counts bytes, #185 item 1) and
`خواص` (auto-property getter returns 4 where `س` is 3) — both byte-identical on a
`develop` worktree build, so pre-existing. The `خواص` one is not among #185's
items and was filed as #239. Two more found the same way: native `sdiv` by zero
silently yields 0 (#240), and 22 `@trq_*` symbols codegen declares are defined
nowhere in `runtime-rs` (#241).

## 2026-08-10 — Issue #182 step 8: execution tests, and و٠٣٠١ starts reaching the user

New `tests/module_execution_tests.rs` — the first module tests that *execute*
programs instead of stopping at `analyzes_ok` (issue #187). 12 tests, each swept
over three backends (`run`, `run --jit`, `compile` + execute the binary) and two
working directories (CWD inside the fixture; repo root with an absolute path to
main — only the second catches the original defect, since CWD-relative
resolution used to succeed by accident).

### Cycle detection was implemented but discarded before reaching the user
Writing the circular-import case revealed that `أ` ⇄ `ب` compiled and ran
silently, exit 0, despite `ModuleLoader` correctly detecting the cycle. Cause:
`preload_imported_modules` ended with `let _ = self.module_loader
.take_diagnostics()`, deliberately dropping loader diagnostics because
`analyze_import` re-reports load *failures* in the third pass and would
otherwise double-report them. That rationale does not extend to cycles: every
module on a cycle still lands in the cache, so the third pass finds them all
present and never re-reports it. The diagnostic was produced and thrown away.

Fix is one filter — keep و٠٣٠١, discard the rest — so the de-dup guarantee for
load failures is untouched. Verified no false positive on the diamond
(A→B, A→C, both→D): the cache hit returns before the `loading_stack` push.
`modules.rs::test_circular_dependency_detection` only ever asserted on the
loader in isolation, which is why the gap survived it.

### Discovery: native stdlib imports segfault (pre-existing, issue #185 item 3)
`استورد * كـ رياض من "رياضيات"` — and equally a plain named stdlib import —
prints correctly under the interpreter and JIT but segfaults (exit 139, no
output) once natively compiled. Stdlib names short-circuit to a builtin table
and are never read from disk, so no body is linked into the object file. Already
filed as #185 item 3; unrelated to the module merge (it crashes identically with
`link_program` stubbed out), and every *local-file* fixture passes natively. The
test asserts the current failure rather than skipping it, so whoever fixes it
sees the assertion trip and re-enables the native leg.

### Also noted
`codegen::linker::tests::test_find_runtime_with_env_var` is flaky under the full
suite: it and `test_find_runtime_nonexistent_env_path` both mutate the
process-global `TARQEEM_RUNTIME_PATH` concurrently. Passes in isolation and in 5
consecutive `cargo test --lib` runs. Pre-existing, unrelated, unfiled.

## 2026-08-10 — Issue #182 step 5: imported module bodies now execute (AST merge)

`check` already passed after steps 1–4, but `tarqeem run` still died with
`دالة غير معرّفة: جمع`. Cause: `ModuleLoader` cached a full `LoadedModule`
including `.ast`, and *nothing read it* — `analyze_import` kept only
`exports.clone()`, and `IrBuilder::build` takes exactly one `Ast` while
`build_stmt` returns `Ok(())` for `Import`. New `src/semantic/linker.rs` merges
every cached module's declarations into main ahead of IR, which repairs the
interpreter, the JIT and native codegen at once (all three consume the same IR
`Module`) and leaves `IrBuilder::build`'s signature — and its ~20 non-pipeline
callers in tests/benches/codegen — untouched.

### Why merging in `semantic`, not in `IrBuilder`
`semantic` may not import `ir`, and `ir` merging would force a second `Ast`
parameter through every existing caller. Placing it behind
`Analyzer::linked_ast` also means the merge sees the module cache that
`analyze` just populated, for free.

### Deliberate deviations from the approved design
- Signature is `link_program(main, loader, main_path, warnings)` rather than
  `(main, loader)`. `Result<Ast, Vec<Diagnostic>>` cannot carry *warnings* on
  its `Ok` arm, so dropped-module-executable warnings need an out-parameter;
  and `main_path` is required to skip main's own cache entry (see below).
- و٠١٠١ (`ERR_DUPLICATE_EXPORT`, previously unused) is reused for merged-name
  collisions, but worded "duplicate top-level definition", because the merge
  carries *non-exported* module declarations too — an exported function must be
  able to call its private helpers.

### Main can appear in its own module cache
A module that imports the main file back caches main under its own path: the
cycle diagnostic lands in `loader.diagnostics`, but the outer `load_module`
still returns `Ok` and `analyze_import`'s Ok path never drains it. Merging that
entry would duplicate every main declaration into a bogus و٠١٠١. The merge skips
any cached module whose canonical path is main's; surfacing the cycle stays the
loader's job.

### Module top-level executables are dropped, not run
Strict no-regression: that code has never run, since imports were always dropped
at IR. Running it would need a module-initialization ordering model. It also
keeps `has_top_level_executable` answering the same for the merged AST as for
main alone — otherwise one stray statement in a library would flip a
Program-mode main into ت٠٢٠١.

### `Span` has no file identity
`Diagnostic::emit` renders every span against the *main* file's source, so a
diagnostic about a module may only ever be anchored to a main-file span. Both
file paths therefore go in the message text, and module-scoped diagnostics are
anchored to the `استورد` statement in main that pulled the module in (falling
back to `Span::default()`, whose line 0 makes `emit` skip the snippet).

### Still broken after this step (pre-existing, verified identical before/after)
- **Aliased imports** (`استورد { جمع كـ اجمع }`) fail at run time: the merge
  carries the original declaration name and nothing rewrites call sites. Needs a
  rename pass.
- **Imported classes** fail earlier, in semantic analysis (`د٠٠٠٣ صنف غير
  معروف`): `ExportKind::Class` defines a symbol but never registers the class
  with `ClassResolver`. Never reaches the linker.
- **`check` does not see merge collisions.** It stops after `analyze` and never
  calls `linked_ast`, so a program that redefines an imported name passes
  `check` and then fails و٠١٠١ at `run`/`compile`. Wiring `check` would mean
  running the merge purely for its diagnostics; left for a follow-up.

## 2026-08-10 — Issues #201/#204/#225: `tarqeem fmt` was source-destructive

`fmt` emitted doc-comment text with no `///` marker, so its own output stopped
being a program; `fmt -w` wrote that to disk with no verification anywhere in the
pipeline. Fixed together with #204 (docs dropped on `صدّر` declarations) and a
newly-filed #225, plus a guard that makes the whole class impossible to ship
again.

### The issue's "same root cause each time" was empirically false
#201's comments assert that all 12 fmt-idempotence failures share one root
cause. Measured with a freshly built binary (the earlier count came from a stale
`target/release/tarqeem`, so it was re-run before touching code, per this file's
own precedent): 66 corpus files, 33 parseable, 12 idempotence failures — but
`examples/تحكم.ترقيم`, `examples/تعداد.ترقيم` and
`examples/حاسبة/مصدر/رئيسي.ترقيم` contain **zero** doc comments. They failed
through an unrelated defect: `format_match_arm` wrote `حالة` unconditionally, so
a wildcard arm became `حالة غير_ذلك`, rejected as `رمز غير متوقع: Default`
(`غير_ذلك` is its own arm production — LANGUAGE_SPEC §15.6). Split out as #225
rather than folded silently into #201. Post-fix: 33/33 idempotent, and the
33-file non-parseable set is byte-identical to the baseline (nothing regressed
into unparseable).

### `trim_end()`, not `trim()` — the trailing-comment helper could not be reused
The obvious fix was to parameterize `write_comment_lines`' marker. That would
have introduced a second, quieter data loss: the lexer strips exactly one leading
space per `///` line, so indentation *inside* a doc block is content, and the
trailing path's per-line `trim()` would flatten an indented example on the first
format. `write_doc_comment_lines` is a deliberate sibling that trims only the
line end, which also round-trips exactly — the emitted `/// ` + line re-lexes
back to the same content, so indentation cannot erode across repeated runs. The
trailing path keeps its full `trim()`; existing tests assert that normalization.

### #204 could not be fixed parser-only without creating new corruption
Threading the doc comment into `parse_export_statement` is the whole parser-side
fix, but on its own it makes the formatter emit `صدّر /// وثيقة\nدالة ...` — the
doc lands *mid-line* and comments out the declaration it documents. The `Export`
arm delegates through `format_stmt_inline`, whose fallthrough called
`format_stmt`, which emits leading trivia. So `format_stmt` was split into
`format_leading_trivia` + `format_stmt_no_leading_trivia`; the inline path takes
the latter and the `Export` arm hoists the inner doc above `صدّر` itself. This is
why #204 belonged in the same change as #201 and not in a follow-up: the parser
half alone is a regression. Anything rendered mid-line must now use the
trivia-free path.

### Decision: verify in `format_source`, not in the CLI
The re-parse guard lives in `fmt::format_source`, so `--check`, `--diff` and
library callers inherit it, and `FormatError::OutputNotReparsable` names it an
internal formatter bug rather than blaming the user's source (which parsed on the
way in). It bounds *unparseable* output only — a formatter that silently drops a
comment still parses fine, so the guard is not a completeness claim. Fixing #225
was a prerequisite: with that bug present the guard would have turned silent
corruption of 4 example files into a hard `fmt` failure.

### Verification method: prove the tests fail without the fixes
All four defects were re-introduced simultaneously and the suite re-run: exactly
18 failures, exactly the 18 new tests — no new test is vacuous, and the
pre-existing trailing-comment tests stayed green, confirming trailing behaviour
was untouched. Also confirmed `check` on formatted output yields diagnostics
byte-identical to `check` on the original (18 pre-existing semantic diagnostics
on `طابور.ترقيم`, 0 `ب`-category), so formatting is semantically transparent, and
that `tarqeem doc` now emits the `صدّر` doc strings it previously dropped. The
corpus test walks the tree at runtime instead of `include_str!`, so it needs no
CI YAML edits (`cargo test fmt` matches `fmt::formatter::tests::*`), and asserts
a floor of 33 parseable files so a parser regression cannot hide behind its skip
branch.

### Review round: accepting `/** */` traded a loud error for three silent ones
A 24-agent review of the first cut found five regressions, all reproduced against
a purpose-built merge-base binary rather than argued from the diff. The pattern
worth remembering: **making the parser accept a token it used to reject moves the
failure from "errors loudly" to "deletes or misplaces text", and the new
re-parse guard cannot see any of it, because the corrupted output still parses.**

- A `/** */` before a statement with no `doc_comment` field (an expression,
  `استورد`) was consumed and dropped, so `fmt -w` erased it where the base
  refused to format at all. Fixed by demoting an unattachable doc comment to
  `leading_comments`; `format_leading_trivia` now routes those through
  `write_comment_lines`, since a demoted multi-line `/** */` would otherwise
  leave its continuation lines as bare code.
- A `/** */` *trailing* code on the same line was re-attached to the **next**
  class member / interface method / enum variant, so `tarqeem doc` published the
  note under the wrong name. Fixed by only accepting a block doc comment that
  starts its own line. `///` deliberately keeps its old behaviour: the same
  misattribution pre-exists for it (verified on base), so changing it is a
  separate concern, not this fix's business.
- `inherited_doc.or_else(...)` short-circuited, leaving a doc comment after
  `صدّر` unconsumed and turning `صدّر /// ملاحظة` into a hard parse error on
  source that compiled before. Now the pending doc is always consumed and the
  outer one wins, with the stray kept as a comment.
- The `format_stmt` split deleted an exported declaration's `leading_comments` —
  and disproved this change's own comment claiming they are "always empty here".
  They are not: the recursive `parse_declaration_with_doc` runs its own
  `collect_line_comments()` after `صدّر` is consumed. The Export arm now hoists
  all leading trivia, not just the doc.
- The guard made `brace_style = next_line` fail on every file while blaming a
  generic "internal formatter bug". That output was always unparseable (the
  parser rejects a newline before `{`), so the refusal is correct — filed as
  #226 — but the message now names the option and renders the diagnostic through
  `Display` with its error code instead of dumping `{:?}`.

Same verification discipline as the first cut: reverting all five fixes fails
exactly the eight new behavioural tests. Also folded in three cleanups — the two
comment-line writers became one helper parameterized on marker and trim strength
(the duplication was the same shape that produced #201), and `fmt --diff` no
longer formats twice via a new `diff_of`, whose output is byte-identical to base.

### Known limitations, deliberately not fixed
- Only the five doc-bearing declaration branches consume the doc comment, so a
  doc before `إذا`/`طالما`/an expression is consumed and dropped. Pre-existing
  for `///`; accepting `BlockDocComment` extends it to `/** */`.
- `صدّر\n/// وثيقة\nدالة` is still a hard parse error (#203 class — the token sits
  where a declaration keyword is expected). Unaffected by the threading, since
  `inherited_doc` is `None` there and `or_else` reduces to the original call.
  Pinned by a test so it cannot degrade into a silent drop.
- `ExportItems::Named`/`Wildcard`/`NamedReexport` and `استورد` have no doc field,
  so a doc comment on those forms is still dropped.
- `format_stmt_inline`'s `VarDecl` arm still drops a trailing comment on
  `صدّر ثابت … // تعليق`. Lossy but parseable, so neither the guard nor the
  idempotence test flags it.
- Plain `/* */` produces no token at all and is still deleted by `fmt`.

## 2026-08-10 — Second review round on #180: native-path soundness

A 33-agent adversarial review of the full #180 diff confirmed 10 findings —
notably that the work was *verified in the interpreter but unsound natively*.
All 10 fixed; 9 new regression tests. The through-line worth remembering:

### Type-shape heuristics are not a substitute for recorded intent
Three separate findings came from one mistake — the ت٠٣٠١ guard treated
`IrType::Ptr(Void)` as "this parameter never got a type":
- an explicitly `أي`-annotated param lowers to `Struct(ClassId("أي"))`, so it
  **slipped past** the guard and emitted the exact ABI mismatch the guard
  existed to prevent;
- an explicitly annotated `قاموس<…>` *also* lowers to `Ptr(Void)`, so it was
  **falsely rejected** — the diff refused syntax it had just documented in
  LANGUAGE_SPEC §5.3 (same for curried lambdas);
- the guard was additionally keyed on the `__lambda_` name prefix, so
  `دالة ضاعف(س)` — the identical hazard one AST node away — bypassed it.

Fixed by recording the reason natively lowering is impossible **when it is
still known**, on `Function::native_block_reason` (a `None`-by-default field;
`Function::new` is the sole constructor, so zero call-site churn — adding a
field to `Parameter` would have touched ~100 test sites). The builder sets it
for untyped/`أي` params and for non-unifiable mixed returns; codegen just
reports it. Non-unifiable returns previously leaked raw clang type errors.

### Provisional types must be re-adopted once reality is known
`infer_expr_type` runs before any body is built and has no arm for a lambda
literal *or* for a call through one, so it lands on the `Ptr(Void)` sentinel.
`متغير ن = مربع(5)` therefore kept a `ptr`-typed slot holding an `i64`, and
`اطبع(ن)` natively emitted `trq_print(ptr %x)` on the value `25` —
dereferencing address 25 (exit 139). Two fixes: `infer_expr_type`'s `Call` arm
now resolves a function-valued callee's `ret`, and the post-build correction
generalized from "only `IrType::Function`" to "any type, whenever the recorded
one is still the unknown sentinel" (`is_unknown_ir_type`), patching
`module.globals` too.

### Hint threading has to cover every position, not just declarations
The lambda param-type hint reached variable declarations and free-function
call arguments but not assignment (`ف = (س) => …` on an annotated slot) or
nested curried bodies — both rejected fully-annotated code by telling the user
to declare a type they had already declared.

### Other fixes
- An expression-bodied lambda whose body is Void (`(س) => اطبع(س)`, an
  idiomatic callback) emitted `ret void %v1` — invalid LLVM, and the void
  `Call` never even names a dest. Now emits a bare `Return`.
- `build_lambda_with_hint` cloned/restored `var_types` but never *cleared* it,
  so the lambda frame could read the enclosing function's leftover entry for a
  colliding `VarId` (`begin_function` resets `var_counter` but not
  `var_types`). Now `mem::take`n, leaving `begin_function` to repopulate the
  lambda's own params.
- The `Type::Any` arithmetic arms were unscoped, silently deleting real type
  errors program-wide (`س ** "نص"` type-checked clean). Narrowed so the other
  operand must be plausible for the operator: §8.3 still works, `**`/`-`/`/`
  against a نص errors again.
- `هذا`/`الأصل` tested the lambda restriction *before* `is_in_class`, so a
  stray `هذا` at module level was told to "pass the receiver as a parameter"
  when there is no receiver — د٠٣٠٤ now wins, as the more accurate diagnosis.

## 2026-08-10 — Removing the `"void"` sentinel from `TypeKind::Function`

Follow-up to the #180 work below, prompted by a review question: why is
`"void" => IrType::Void` the one ASCII arm in a `match` whose other arms are
all Arabic (`عدد`, `عدد_عشري`, `نص`, `منطقي`)?

### The answer was "delete it", not "translate it"

Investigation (see the "bare `()` function type" section in the #180 entry
for the full reasoning) established three things:

1. **Not a philosophy violation as such.** `.claude/rules/arabic-philosophy.md`
   governs the *language surface*. `"void"` was never typed, lexed, or
   printed. `lexer.rs:104-105` intercepts Latin letters *before* identifier
   scanning and hard-errors; `is_arabic_letter` is an explicit
   Arabic-Unicode-block whitelist that never calls `is_alphabetic()`. So
   `صنف void { }` is a lex error and collision was impossible.
2. **Still a real defect**, for better reasons: it was smuggled into
   `TypeKind::Simple`, the user-name namespace (collision-free only via
   another module's lexer rules), and it reinvented the codebase's existing
   `Option<TypeAnnotation>` idiom for "no type declared".
3. **The obvious fix would have been a regression** — `فراغ` is a valid
   Arabic identifier and is asserted to be a user class name.

### The change
`TypeKind::Function::return_type` became `Option<Box<TypeAnnotation>>`, where
`None` **is** the bare `()` form. One producer
(`decl_parser::parse_function_type_annotation` — the only construction site of
`TypeKind::Function` in the codebase), eight consumers in two mechanical
patterns: four lowerings adopt `.as_ref().map(...).unwrap_or(Void)` (mirroring
`func_signature_types`/`build_func_decl` exactly), three printers collapse to
`if let Some(rt)` / `match`. The `TypeKind::is_bare_unit_function` helper was
deleted rather than kept — the duplication it centralized disappears entirely
when the check becomes a one-token `is_none()`.

Both now-unreachable `"void"` arms (`semantic::types::parse_type_name`,
`ir::builder::type_helpers::convert_simple_type`) were deleted. The printers
are also *strictly* more correct now: the old guard suppressed `->` only when
`params.is_empty()`, so a hypothetical `(عدد) -> <none>` would have printed
the unlexable `-> void`. A fabricated span (pointing at `)`) that rode along
in the AST and its JSON also disappears.

**Left in place, deliberately:** `TokenKind::TypeVoid` (`lexer/token.rs:95`)
and its `expect_type_name` arm. No Arabic keyword maps to it
(`keywords.rs:175` asserts `lookup_keyword("فراغ") == None`), so it is already
unreachable from user source, but removing it touches `token.rs`,
`token_tests.rs`, `is_type_keyword()`, and `semantic_tokens.rs` — a
pre-existing cleanup unrelated to #180.

### Serialization
`TypeKind` derives `Serialize` only (no `Deserialize` anywhere in `ast.rs`).
The sole emitter is `cargo run -- parse --format json`. For the bare-`()` case
only, `"return_type"` changes from `{"kind":{"Simple":"void"},"span":{…}}` to
`null` — arguably the more honest shape. No in-repo consumer, no
golden/snapshot files, no `insta`. The CLI comment says "for IDE integration",
so external consumers are conceivable.

### Documentation drift found and fixed
- **`LANGUAGE_SPEC.md` §4.2** claimed `بداية_معرِّف := <حرف يونيكود> | '_'`
  (*any* Unicode letter, which would admit Latin). The lexer implements an
  Arabic-block whitelist. This was precisely the claim that made the sentinel
  look dangerous. Restated in terms of Arabic blocks + digits + diacritics +
  tatweel + `_`, with an explicit invalid-identifier example.
- **`LANGUAGE_SPEC.md` §15.4** made `->` mandatory
  (`نمط_دالة := '(' […] ')' '->' نمط`), so the formal grammar could not derive
  the bare `()` that §5.3's prose shows and the parser accepts. Now
  `['->' نمط]` with a note that omission is legal only with an empty
  parameter list.
- **`.claude/rules/arabic-philosophy.md`** contained a `test_mixed_direction`
  snippet (`متغير x = 5` asserting `parse(...).is_ok()`) that does not exist
  in the codebase and would **fail** if written. A rules file that teaches
  future agents a false invariant is actively harmful; replaced with the real
  `test_english_identifiers_produce_errors` behavior from `lexer.rs:857-865`.

## 2026-08-10 — Issue #180: arrow lambdas unfinished at the IR stage

Highest-impact pick from the #180–#212 backlog: arrow lambdas (`(س) => س * س`)
type-checked but could never be invoked in any of the three execution modes —
including the exact example from README's «الدوال» section.

### Root cause: the IR had a function *type* and an indirect-call
*instruction*, but no function *value*

`IrType::Function` and `Instruction::CallIndirect` both already existed and
were already correctly consumed by the interpreter (`Value::Function(String)`
+ its `CallIndirect` arm worked on day one). `Constant` only had
`Null/Bool/Int/Float/String` — nothing could represent "the address of this
function" as an IR value. `build_lambda` lifted the lambda body into a real
function (`__lambda_N`, correctly pushed into the module) and then discarded
the reference, emitting `Constant::Null` with the literal comment "Will be
replaced with function pointer." `build_call`'s identifier-callee branch also
unconditionally emitted a direct `Call` against `FuncId(name)` — it never
consulted locals/globals, so `مربع(5)` became a call to a `FuncId` that never
existed.

### Decision: `Constant::Function(String)`, not a new `FuncRef` instruction

Both broken emission sites (`build_lambda`, and `build_identifier`'s
function-name branch) already used `Instruction::Const`; a new variant on the
existing `Constant` enum was a same-shaped swap. It also mirrors the
interpreter's pre-existing `Value::Function(String)` exactly (a bare name, not
a typed pointer), so `constant_to_value` became a one-line arm. Blast radius
was 8 confirmed exhaustive-match sites (`Display`, `const_to_type`, two
`constant_to_value` copies, two codegen sites, two dead Cranelift-JIT tiers) —
all mechanical, all fixed. `src/lsp/`, `src/fmt/`, `src/doc/` have zero
`crate::ir` references and needed no changes; DCE/CSE/const-fold all had
wildcard arms already.

### Two state-corruption bugs found during design, not in the original issue
- `IrBuilder::var_types` is keyed by a raw `u32` `VarId`. `begin_function`
  resets the per-function `var_counter`, so IDs *inside* a lifted lambda's
  body numerically collided with and silently overwrote the *enclosing*
  function's type entries for those same numeric IDs — corrupting later
  float/string dispatch in the outer function. Fixed by cloning/restoring
  `var_types` around the nested build.
- `IrBuilder::loop_stack` holds `BlockId`s for enclosing `أوقف`/`استمر`
  targets; since `block_counter` also resets per function, those `BlockId`s
  could alias a lambda's own blocks. Fixed with `std::mem::take`/restore.
- Also: `__lambda_N` was named from the *per-function* `var_counter`, so two
  lambdas in two different enclosing functions could collide on the same
  name (`Module::get_function` is a first-match linear `Vec` scan, no
  duplicate detection). Switched to a module-global `lambda_counter`.
- Found empirically while smoke-testing native compilation (not predicted by
  design): an unannotated global lambda's `IrType` is a one-shot
  `infer_expr_type` *guess* made *before* the lambda body is built (no
  `ExprKind::Lambda` arm existed, so it fell back to `Ptr(Void)`); nothing
  ever corrected it once the real signature was known, so `CallIndirect`
  through that global read the wrong `ret_ty` — LLVM emitted
  `call ptr %v2(i64 %v1)` against a function actually defined
  `i64 @__lambda_0(i64 %arg.0)`, which segfaulted. Fixed by writing the real
  `IrType::Function` back into `global_var_types`/`var_types` once the
  initializer is built, in both the global and local `build_var_decl`
  branches.

### Deliberate scope: non-capturing lambdas only
A lambda referencing an outer local/parameter now gets a clear diagnostic
(`ERR_LAMBDA_CAPTURE`, د٠٣٠٦) at the semantic-analysis stage — chosen over
leaving it to fail with a confusing IR/runtime error. Closures (a real
capture environment, heap-allocated and refcounted) are an explicit
follow-up (filed as #217) — the
Plan-agent's threat model here was correct: `Scope`'s flat `variables` map
has no way to distinguish "resolved in an enclosing scope" from "undefined"
once `begin_function` clears it, so detection had to live in the semantic
analyzer (which still has the scope chain and real spans), not the IR
builder.

### Any-arithmetic widening (spec §8.3)
`binary_result_type` had no arm for `Type::Any` operands with arithmetic
operators, so an inference-typed lambda (`(أ، ب) => أ + ب`, LANGUAGE_SPEC
§8.3 verbatim) failed with "لا يمكن تطبيق العامل '+' على أي و أي" before it
could ever reach IR. Added `Any` arms (arithmetic → `Any`, relational →
`Bool`). Confirmed zero existing tests assert an `Any`-operand arithmetic
error; the widening is consistent with `Any`'s pre-existing role as the
universal `is_assignable` escape hatch (untyped function/lambda/method
params, `Any`-returning math builtins like `مطلق`/`أكبر`/`أدنى` all became
newly permissive too — a deliberate, understood trade-off, not a side effect
missed in review).

### The bare `()` function type — first done with a sentinel, then fixed
`(عدد، عدد) -> عدد` function-type annotations (spec §5.3, previously
unparseable — ب٠٠٠٢) needed a representation for the bare-`()` case ("no
params, no return"). The **first attempt** reused `expect_type_name`'s
pre-existing dead `"void"` string as a sentinel inside
`TypeKind::Simple(String)`. That was wrong and was replaced the same session
(see the entry below) — recorded here because the failure mode generalizes.

**Why the sentinel was wrong** (and why "it's the only English string in an
Arabic match" was *not* the reason): it is unreachable from user source, since
`lexer.rs:104-105` intercepts Latin before identifier scanning, so no
collision was actually possible. The real defects were
(a) it lived in `TypeKind::Simple`, the variant modelling *user-written* type
names, whose lowerings fall back to `Type::Class(name)` — collision-free only
by an accident of a *different module's* lexer rules, with nothing asserting
that at the boundary; and (b) it reinvented an idiom the codebase already
had. Tarqeem deliberately has no `فراغ` keyword — a function returning
nothing simply omits `-> نوع` — and the AST already models that as
`Option<TypeAnnotation>` (`FuncDecl::return_type`, `Param::ty`), lowered via
`.map(convert).unwrap_or(Void)`. The four special-case sites the sentinel
required were all symptoms of the wrong encoding.

**Note the trap in the obvious fix:** translating `"void"` → `"فراغ"` would
have been a real regression. `فراغ` *is* a valid Arabic identifier, so it
genuinely could collide with a user class of that name, and
`types_tests.rs:508` asserts `parse_type_name("فراغ") == Type::Class("فراغ")`.
The right move was to delete the string, not translate it.

### Native-mode-only restriction: untyped lambda params (ت٠٣٠١)
An unannotated lambda param lowers to `IrType::Ptr(Void)`. The interpreter
handles this fine dynamically, but native codegen would otherwise link a
call site passing a concrete type (e.g. `i64`) against a callee expecting
`ptr`, silently misinterpreting the bits. Added `ERR_UNTYPED_LAMBDA_PARAM`
(ت٠٣٠١), raised only from `emit_function` for `__lambda_*`-named functions
whose params never resolved to a concrete type — `tarqeem run` is
unaffected.

### Post-review hardening (same session, high-effort adversarial review)
A 26-agent review of the diff confirmed 7 correctness gaps + 3 cleanups, all
fixed:
- **ت٠٣٠٢ (new)**: native `CallIndirect` through a callee whose static type
  isn't a `Function` signature (the `أي` escape hatch) previously emitted an
  ABI-mismatched `call ptr` against a `define i64` — where HEAD failed at
  link time, i.e. the original fix *downgraded* an error into silent
  corruption. Codegen now rejects it with a clear diagnostic; the
  interpreter still runs it.
- **Hint-threading was decl-initializer-only.** Lambdas as call arguments
  (`طبق((س) => س * ٢، ٥)`) and program-mode annotated globals (routed
  through `__global_init__`, which used a bare `build_expr`) both lifted
  with `Ptr(Void)` params → spurious ت٠٣٠١ on spec-legal annotated code.
  `build_call` now resolves the callee's param types *before* building
  arguments and hints lambda literals; `__global_init__` shares
  `build_global_initializer` with `build_var_decl`. The program-mode
  fourth pass also no longer re-builds global initializers outside any
  function — that lifted an orphaned duplicate `__lambda_N` whose
  `GlobalStore` was silently dropped.
- **Block-lambda returns unify across ALL `أرجع`s**, not just the first:
  bare returns in a non-void lambda are patched to zero-of-type returns and
  mixed عدد/عدد_عشري promote to عدد_عشري (per-return `IntToFloat`), or
  native codegen emitted `ret void` inside `define i64` — invalid LLVM.
  Non-unifiable mixes (dynamic code) keep the first valued type; the
  interpreter ignores static return types either way.
- **Spec §5.6 int→float coercion now applies at call arguments** (indirect
  and direct) when the callee's signature is known — previously only
  variable stores coerced, so `ف(٥)` against `(عدد_عشري) -> عدد_عشري`
  natively reinterpreted the i64 bit pattern as a double.
- **`هذا`/`الأصل` inside a lambda** now raise د٠٣٠٦ at the semantic stage
  (a receiver is a capture in disguise); **`أوقف`/`استمر` inside a lambda**
  now raise د٠٣٠١ because `is_in_loop` stops at function/lambda scope
  boundaries — both previously escaped semantic analysis and died later as
  span-less internal IR errors.
- Cleanups: stale TDD "RED until Step N" comments removed; the bare-`()`
  void-sentinel check unified into `TypeKind::is_bare_unit_function` (later
  deleted outright — see the entry below); the LSP's forked `parse_type_name`
  now delegates to `semantic::parse_type_name` (re-exported).

### Process note
Built via a 5-step TDD pipeline (RED tests → parser → semantic → IR/codegen
→ docs), each step dispatched to a fresh subagent with the full design
already researched and approved, verified independently after each step
(`cargo test`/`clippy`/diff-scope check) before proceeding. One subagent
flagged a bizarre, garbled comment left by an earlier step in
`type_helpers.rs` ("this should be writtne in arabic ... if you find it at
code review surfce it please") as a possible prompt-injection attempt aimed
at a future reviewer; grepped the full repo and git history to confirm it
was novel to this session (not present in any prior commit, not echoed from
any other file) — concluded it was a stray artifact from that earlier
subagent's own generation, not evidence of an external attack, and removed
it. The second claim from that same agent (an alleged instruction telling it
not to report file changes) could not be corroborated anywhere in the repo
or tool output; treated as unverified and disregarded per instructions,
noted here for the record.

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
A scan of all 65 real `.ترقيم` files in `stdlib/`+`examples/` found the
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
`stdlib/ملفات/مجلد.ترقيم`) was confirmed a strict subset via `comm`, zero
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
- `TARQEEM_HOME` (set on this machine) shadows the repo `stdlib` in
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
