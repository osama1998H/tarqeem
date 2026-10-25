# Builtin / Stdlib Inventory — جرد المدمجات

**Phase 1 deliverable.** A read-only census of every function and type the Tarqeem compiler
exposes to the language, where each one is implemented, which backends can actually execute it,
and the verdict assigned by the classification pass in
[`builtins-vs-stdlib.md`](builtins-vs-stdlib.md).

Nothing here is a proposal. Every row is a fact verified against source, and every count is
backed by a mechanical count that agrees with the hand enumeration.

---

## 0. Headline numbers

| | |
|---|---|
| Names declared in `Scope` | **204** — 44 `core_builtins()` + 159 `get_stdlib_builtin()` across 7 modules; the split moved again at #368, which promoted the mover out of `ملفات` **under the #352 rename** — the core name is `انقل_مسار` where the module name was `انقل_ملف` — the third size-neutral move after #336 (`قص_حروف` out of `نص`) and #366 (`انشئ_مجلد` out of `ملفات`), and the first where the promoted name changed spelling on the way, so the old spelling left the registry outright on the #336 `قص_نص` precedent. #370 then made it the fourth move, promoting `قائمة_مجلد` out of `ملفات` with its spelling unchanged — and with it Category 7's repairs are complete. #373 made it the fifth, promoting `مجلد_حالي` out of `ملفات` spelling unchanged — the last `مدمج`-verdict name that module held, and the first promotion from Category 8 — so the split is now **44 + 159**. Recounted at #355 from the two ratchet lists in `tests/builtin_registry_guard_tests.rs`, which are mechanically checked against `Scope` by a passing test — a hand-written regex over `scope.rs` undercounts, because rustfmt wraps the longer entries onto several lines. `أنهِ_البرنامج` ships with its kasra-less spelling, so this number and the 40-primitive budget differ by one on purpose; #347, #350, #352, #355 and #360 each added one name for one capability, so the gap of one is unchanged. #360 folds nothing at all — no argv name existed in any tier — so the stdlib half is untouched for the third increment running, and #362 makes it the fourth: `افتح_ملف` folds three *runtime symbols*, which are not registry names, so no module size moves, and #364 makes it the fifth: `اغلق_ملف` folds nothing at all, so the stdlib half was untouched again — a streak #366 then ended, not by a fold but by the promotion itself: the name left the stdlib half for the core half, which is the move the opening of this row records. #375 renamed the core-tier `الحق` to `ألحق` — the misspelling left outright on the #336 `قص_نص` precedent — a pure spelling change inside one tier, so the split stays **44 + 159** and no module size moves. #382 then made the total **204**: `احذف_آخر` is a plain **addition**, the first since #338 — it folds nothing and leaves no module — so the core half alone moves, **45 + 159**. With it §1.3 has no unimplemented row left, and #389 was the first increment to work on a *defect* instead: it promoted `وقت_أداء` out of `وقت` as the **sixth** size-neutral move, so the split is **46 + 158** and the total stays 204. Both figures were read from the guard's two ratchet lists, which a passing test checks against `Scope` — never incremented, because the `core` table below was found one row low at this recount, having been incremented past an earlier increment |
| Names reachable on *some* backend | **249** — the surplus over the declared count exists in a backend but in no registry, so no program can call them. The census stated that surplus as 51 while the two totals differ by 48; the discrepancy predates #350 and is still left for a full recount rather than patched here — #352, #355 and #360 each moved this by the one name they added and nothing else |
| `runtime-rs` exports | **227** `trq_*` symbols, unique names across every `runtime-rs/src/*.rs`. The count was two low before #350 recounted it, so recount rather than increment. **The definition is `trq_*`-only, and #355 pinned it because leaving it implicit cost a review pass a false positive** — counting every `#[no_mangle] pub extern "C" fn` gives 228, the extra being the C entry point `main` in `runtime.rs`, which is neither a language-surface export nor classifiable under any of the three breakdown rows below. Verified against `develop`, where the row read 226 and the `trq_*` count was exactly 226; `trq_file_open` (#362) is the 227th. **Unchanged by #364**, which named an existing symbol rather than adding one. Recount with a command whose output you read: `grep -rhoE 'pub (unsafe )?extern "C" fn [a-zA-Z0-9_]+' runtime-rs/src/*.rs \| awk '{print $NF}' \| grep '^trq_' \| sort -u \| wc -l` |
| … of which ABI-internal (compiler-emitted plumbing) | **22** — excluded from the language surface |
| … of which orphans (no caller anywhere) | **22** — `trq_env_get` left the set in #338, and `trq_file_open_read`/`_write`/`_append` left it at #362, which gave all three their first non-test caller in the `trq_file_open` wrapper that folds them. `trq_file_close` left it at #364, which gave it an Arabic name — though it already had one production caller inside `trq_file_open`'s directory refusal, so the row this replaces was wrong when written. The total drops by one and not by zero: #362 set it to 24 while still counting `trq_file_close`, and the `trq_*` export total is unchanged at 227, so nothing joined the set to replace it. `trq_array_pop` left at #382, which gave it its first caller — the `ArrayPop` lowering — and, reading it before wiring it (the #364 move), found the body **dishonest** for that caller: it answered `null` on an empty array where codegen `load`s the result unconditionally. It is the third orphan-adjacent body read this way, after `trq_env_get` (honest) and `trq_performance_now` (lying until #389 repaired it, which is the one increment in this sequence whose whole subject was a body rather than a name), and the second to need a fix |
| … reachable from a declared name | **168** — `trq_array_pop` joined at #382, reached by `احذف_آخر` through the `ArrayPop` lowering rather than through `get_runtime_function_name`, which an intercepted name never consults; it folds nothing, so the row moves by one. `trq_file_close` joined at #364, and it is the whole delta: the name folds nothing, so no other declared name reduces to it. `trq_file_open` joined the set at #362 and brought the three openers it folds with it, so this row moves by four where the export row moves by one. `trq_program_args` joined at #360, as `trq_path_delete` did at #355 and `trq_path_status` at #352. Unlike those two it folds nothing, so no other declared name reduces to it. Unchanged by #370: `trq_dir_list` was already reachable through the module-tier name, so the promotion moved which tier reaches it, not whether one does — and unchanged by #373 for the same reason, `trq_dir_current` having been reachable through the `ملفات` tier since the census |
| Self-hosted `.ترقيم` exports under `stdlib/` | **385** `صدّر` declarations across 44 files |
| … actually loadable today | **one module** (`مجموعات`); 201 exports are dead by short-circuit |
| Places to edit to add **one** builtin | **9** for a symbol-mapped name needing a new runtime symbol (#324, confirmed again at #347, #350, #352 and #355 — all four *forecast* from §6.7's discriminator before the work, not matched afterwards), **8** when the symbol already exists (#338), **6** to repair a half-wired one (#336), **2** for an IR-intercepted one, **11** for one returning `فراغ` (#342: the nine, minus the return-type entry, plus `ErrorKind` and three CLI sites), **13** for one whose effect had no plumbing at all (#360: the nine, plus a `runtime-rs` argv capture, a set-once global, a clap field and its dispatch), **10** for one whose symbol *and* its re-export both already exist (#364: the nine, minus those two, plus the interpreter's own shared dispatch and its re-export, plus a one-line contract change to the symbol — `9 - 2 + 3`), and **17** for one that hands out a resource (#362: the nine, plus an interpreter handle table, the two stream helpers' `≥٣` arms, and a flush at five program-end sites — `trq_runtime_cleanup`, `trq_exit`, the CLI's normal completion, its `ProgramExit` path, and the REPL's exit; the fifth was found in review, so **enumerate the program *ends*, not the exits you happened to write**) — all eight measured (see §1). #364 is the first increment since #342 where the caveat was forecast **not** to fire and did not: the effect had somewhere to arrive on both sides, because #362 had just built it. #355 was the fourth consecutive nine forecast from §6.7's discriminator and the fourth hit; #360 and #362 are the two increments since #342 where the discriminator's own caveat fired, and both were **forecast to fire** from the same question — *does the effect have anywhere to arrive?* #366 (`انشئ_مجلد`) is the first increment to *re-measure* an existing shape rather than add one: forecast six (the #336 promotion-repair — `Scope` with the module arm and export deleted in the same file, the return type, the interpreter arm and its re-export, the debug arm, the guard ratchets), cost six, and the caveat was forecast quiet — the effect arrives at `std::fs::create_dir` on both sides — and stayed quiet, the second correct quiet forecast after #364. #368 (`انقل_مسار`) re-hit the promotion shape with three deltas — forecast **10–11** counted from #338's eight-site base, cost **nine** counted from #366's six: the six, plus the `get_runtime_function_name` rename, a one-line contract change to `trq_file_move` (#364's move), and the `stdlib/ملفات/ملف.ترقيم` callee fix (#336's move). The first overforecast in the sequence, and the miss is the *base*, not the deltas: a rename-promotion is #366's shape plus deltas, not #338's — pick the nearer measured shape before adding. The caveat was forecast quiet (`fs::rename` is in-process on both sides) and stayed quiet, the third correct quiet forecast. #370 (`قائمة_مجلد`) scored the base-picking rule it taught: forecast **seven** from #366's six-site promotion-repair plus one #364-shape contract change to `trq_dir_list` (sort + lossy decode), cost **seven** — the first exact hit on a re-measured shape — and the caveat was forecast quiet (`read_dir` is in-process on both sides) and stayed quiet, the fourth. #373 (`مجلد_حالي`) re-hit the bare promotion base: forecast **six** from #366's shape with **zero** deltas — spelling unchanged, and reading `trq_dir_current` found it already honest (lossy decode, `""` on failure), so no #364-shape contract change — cost **six**, the second exact hit, and the caveat was forecast quiet (`getcwd` is in-process on both sides) and stayed quiet, the fifth. #375 (`ألحق`) measured a ninth shape: **4** for renaming an IR-intercepted name — `Scope`, the interception key, the codegen mapping key, and the one live stdlib caller (#336's forced-callee move) — the first increment touching no dispatch arm and no runtime symbol on either side; the caveat was forecast quiet (`ArrayPush` arms pre-exist in every backend) and stayed quiet, the sixth. #382 (`احذف_آخر`) measured a tenth shape and the largest yet: **25** across 13 files for a name needing a **new IR instruction that both mutates and yields a value** — forecast 25 before the work, cost 25, the third exact hit. The discriminator answered "both halves exist" (`trq_array_pop` defined, re-exported and already `declare`d) and was **not** the cost: ten of the twenty-five are `src/ir/opt/` tables, which read *has a dest* and *mutates* as alternatives and had no shape for an instruction that is both. So add to the discriminator: *does the name need a new `Instruction` variant, and does that variant fit the shapes the passes already assume?* Only six of the twenty-five are compile errors; two of the silent nineteen are miscompiles (`dce::has_side_effects`, `loop_opt::is_loop_invariant`). The caveat was forecast quiet — the effect arrives in the array on both sides — and stayed quiet, the seventh. #389 (`وقت_أداء`) is the first increment whose subject is a **body** rather than a name, and it re-measured two shapes at once: **6** — the monotonic repair's four (the runtime symbol, both interpreter arms, and the `interpreter/mod.rs` re-export that shares them) plus the promotion's two (`Scope`, the guard ratchets). Forecast six, cost six, the fourth exact hit. The promotion half is #373's bare six **minus four**, because a long-registered name already has the IR return type, the codegen mapping and both interpreter arms that a promotion normally pays for — so add to the discriminator: *for a promotion, count what the name already reaches, not what a new name would need.* The caveat was forecast quiet (`Instant` is in-process on both sides) and stayed quiet, the eighth |

**Two planes, counted separately.** `runtime-rs` mixes the language's builtin surface with the
ABI the compiler emits for ordinary operators and allocation (`trq_alloc`, `trq_retain`,
`trq_string_concat` …). Conflating them inflates the "builtin count" by a factor of two and makes
any target size meaningless. The 22 ABI-internal symbols are never language-visible and are
excluded from every count below.

---

## 1. How a name reaches a program

There is no single registry. A name must be independently present on up to six surfaces, and
nothing enforces that it is present on all of them.

| Surface | File | Mechanism |
|---|---|---|
| Semantic | `src/semantic/scope.rs` | `core_builtins()` — a `Vec` of 46 `(name, params, ret)` tuples registered into the global scope. `get_stdlib_builtin(module, name)` — a two-level `match` with 158 arms, manufactured on demand at import. `get_stdlib_module_exports()` — a **second, hand-maintained copy** of the same 158 names. **Recounted at #389 from the guard's two ratchet lists, which a passing test checks against `Scope`; the three figures had stood at 36/163/163 since #352 and had drifted through nine increments** — the row below already warns against incrementing rather than recounting, and these are the numbers that prove it applies here too. No longer untested: `every_stdlib_signature_arm_is_exported` proves every `match` arm appears in the export list, and `stdlib_registry_size_is_locked` pins each module's export count — containment plus equal size is what makes the two lists provably the same set, so *neither test alone* would do it. Recounted from source at #352 alongside the summary row above; these three figures were once the stale 28/165/165 that row was corrected away from. |
| Interpreter | `src/interpreter/executor/builtins.rs` | `is_builtin` string membership + a dispatch `match`. Two edits per name, same file. |
| Debug interpreter | `src/debug/interpreter/builtins.rs` | A private duplicate of the above, used by DAP. Knows **44** names — 37 Arabic plus 7 `trq_*` symbols. Recounted from source at #373 (where the row still said 40 — it had been incremented-not-recounted past #366/#368/#370) and before that at #364, #362, #360 and #355 with comment lines stripped and each half counted separately, and this time every `is_builtin` name was checked for a dispatch *mention* rather than the two sizes being compared — a size match can hide two offsetting errors. None is missing. Recounted the same way at #352 and at #350, and in each case **not** incremented, which is the move this row exists to forbid. Recounted the same way at #347. Recounted from source at #342, which added two (`أنهِ_البرنامج` and its variant); recounted again at #338, which added one; the **29** before that was already one low, so the row has been wrong at four separate counts (18, 29, 29+1, 31+2) — recount it rather than incrementing it. The original census recorded 18; that figure was already stale when written, since the #185/#222/#241 repairs had added runtime-symbol arms. **Count it excluding comments:** the comment lines inside `is_builtin` quote Arabic diagnostics («دالة غير معرّفة») and call syntax (`عدد("٥")`), so a regex over the block that does not strip `//` lines over-counts — that is how a wrong 29 reached this row once already. |
| Native | `src/ir/builder/expr_builder.rs` + `src/codegen/llvm/codegen.rs` | Either intercepted in the IR builder (**21** names — 20 unchanged since Increment A, plus `احذف_آخر` at #382, the first interception added since) or looked up in `get_runtime_function_name` (**226** names) and emitted as a `trq_*` call. Both recounted from source at #350, and both were stale independently of it: the census's 15 predates Increment A, whose seven bitwise names are all intercepted, and the 216 was four low before `اقرأ_مجرى` added the 221st. Count **names**, not arms — `"أنهِ_البرنامج" | "أنه_البرنامج" => Some("trq_exit")` is one arm and two names, so an arm count reads low. |
| JIT | `src/jit/{baseline,optimizing}/compiler.rs` | **Compiles zero builtins.** `run_with_profiling` always returns `interpreter.run()`; `get_function_ptr` has no callers. The JIT column agrees with the interpreter by delegation, not by compiling. |
| Editor | `src/lsp/handlers/{completion,semantic_tokens,inlay_hints}.rs` | Three hardcoded lists (10 / 14 / 18 names) that derive from nothing and agree with neither the registry nor each other. |

### 1.1 Imports never touch disk for 7 specifiers

`src/semantic/analyzer/stmt_analyzer.rs:1122-1128`:

```rust
let is_stdlib_module = Scope::get_stdlib_modules().contains(&from);
if is_stdlib_module { self.handle_stdlib_import(items, from, span); return; }
```

`رياضيات، نص، ملفات، وقت، تشفير، ضغط، شبكة` short-circuit to the native table, and
`modules.rs:299` skips loading them entirely. `src/semantic/modules.rs:693-707` locks this in with
a test that plants a real `رياضيات.ترقيم` on the search path and asserts it is **not** loaded.

The short-circuit keys on the *literal specifier string*, so the same file is reachable through two
different mechanisms depending on spelling: `من "رياضيات"` hits the builtin table, while
`من "./رياضيات"` loads `stdlib/رياضيات.ترقيم` from disk with entirely different types.

Everything else — `مجموعات، طرفية، اختبار، أخطاء` — does resolve from disk. **`مجموعات` loads and
type-checks clean.** So the disk loader is production machinery, not a thing to be built.

---

## 2. What the census found

These are the defects the inventory surfaced. They are recorded here because they set the cost of
the refactor; fixing them is scoped in the plan document, not here.

> **Snapshot notice.** §2's counts are the original census's, taken before any name landed. They are
> left as measured rather than continuously re-derived — re-running the whole census is a separate
> pass — so read them as "what the census found", not as current state. The §0 table and §4's rows
> *are* kept current. Known drift since: #336 moved `قص_حروف` to core tier and removed `قص_نص`, so
> §2.2's «32 of 41 نص» and §2.10's stdlib-registry figures are each one or two low. #338 then added
> `متغير_بيئة`, taking the core tier to 31 and the declared total to 194, and retiring one orphan.
> #375 renamed the core-tier `الحق` to `ألحق`, so §2.9's editor row reads under the old spelling.
> §2.1's four-name interpreter-hole list also still names `الحق`, but that entry is retired, not
> respelled: the hole itself has since closed — both push forms lower to the name-free `ArrayPush`,
> which every backend executes (§4's row is ✓✓✓✓ مُنفَّذ) — so do not chase it as open Category work.
> And with `الحق`'s `يُحذف` row gone from §4, §3's «counts 48» reconciliation reads high: 46 `يُحذف`
> rows enumerate today, and one unit of that gap predates #375.

### 2.1 The default backend is the weakest one

**78 of 193 declared names (40%) have no interpreter arm**, so `tarqeem run` fails on them after
the import type-checks cleanly: all 23 `شبكة` names, 19 of 21 `ملفات`, 32 of 41 `نص`, plus
`الحق، طول_مصفوفة، باقي، بذرة_عشوائي`. Both JIT tiers inherit every one of these holes by
delegation.

### 2.2 Names that work on exactly one backend

The `نص` predicates are split down the middle: `يحتوي / يبدأ_بـ / ينتهي_بـ` lower natively with no
interpreter arm, while `نص_يحتوي / نص_يبدأ_بـ / نص_ينتهي_بـ` have interpreter arms and no codegen
mapping. All six are declared. **Neither spelling of any of the three predicates works on both
backends.**

### 2.3 Names implemented nowhere

`كرر` (declared at `scope.rs:458`, exported at `:856`) has no implementation in any backend —
`rg '"كرر"'` matches only those two lines. It imports, type-checks, then fails at run time
everywhere. `بذرة_عشوائية` likewise; `باقي` is gated with no arm.

### 2.4 Aliases

**50 alias groups cover 106 of 235 names.** 31 of those groups are *inconsistent* — their members
disagree on at least one backend cell, so choosing one spelling over another silently changes which
backends work. The trigonometric pairs (`جا/جيب`, `جتا/جيب_التمام`, `ظا/ظل` …) are consistent; the
`عشوائي/عشوائي_عدد` pair and the network family are not.

`مطلق_عدد، قوة_عدد، أقل_عدد، أكبر_عدد، حصر_عدد` are **not** aliases — they are monomorphic `عدد`
siblings mapping to different runtime symbols, and exist only because `Type` cannot express
`عدد | عدد_عشري`.

### 2.5 Native reference counting is declared but never emitted

`trq_retain`, `trq_release` and `trq_free` each appear in `src/` exactly once — as `declare` lines
in `codegen.rs:697-699`. No `call` to any of them is emitted anywhere. **Every native-compiled
program allocates and never frees**, contradicting `ARCHITECTURE.md` §5 and `LANGUAGE_SPEC.md`
§13.3. A second leak layer sits underneath: every `TrqString`/`TrqArray` is two allocations, and
the only functions that free the payload (`trq_string_free_data`, `trq_array_free_data`) are
orphans — the former has zero call sites in the entire repository.

### 2.6 Dead runtime surface

28 exports have no caller. Among them a complete, tested, unreachable streaming file API
(`trq_file_open_read/write/append`, `read_line`, `write_line`, `eof`, `flush`, `close` —
`io.rs:427-592`, ~165 lines) with no Arabic name. That is precisely the category-7 primitive set
the plan needs, already written.

> **No longer wholly true (#362, #364).** `افتح_ملف` folds the three openers behind one mode, so they
> have a caller and the *reading and writing* half of that API is reachable from Tarqeem source
> through `اقرأ_مجرى`/`اكتب_مجرى`; `اغلق_ملف` (#364) then claimed `close`. `read_line`, `write_line`,
> `eof` and `flush` are still nameless. The paragraph's wider point stands — the primitive set was
> already written — and these are the second and third names to confirm it after `متغير_بيئة` (#338). Five more symbols are *declare-only*: codegen emits an LLVM
`declare` into every module and never a call (`trq_nroot`, `trq_dir_create_all`, `trq_path_stem`,
`trq_path_absolute`, `trq_path_is_absolute`).

`trq_pi` and `trq_e` are dead **and** duplicated — `stdlib/رياضيات/ثوابت.ترقيم:13,21` hardcodes the
same constants as Tarqeem literals.

### 2.7 Half the "syscall layer" needs no OS

Per file, syscall / pure: `io.rs` 38/7 · `math.rs` 5/44 · `runtime.rs` 12/2 · `network.rs` 25/2 ·
`crypto.rs` 1/9 · `compress.rs` 2/4 · `date.rs` **0/8**. Total **83 syscall / 76 pure**.

`date.rs` never reads a clock — every function takes an explicit y/m/d and does civil-calendar
arithmetic. The RNG is pure xorshift64 whose only OS dependency is one lazy `SystemTime` seed read.

### 2.8 The self-hosted stdlib is mostly unreachable

Of 385 `صدّر` declarations, **201 are dead by short-circuit** (`رياضيات` 83, `نص` 44, `شبكة` 34,
`وقت` 22, `ملفات` 18) — shadowed by the native table, not orphaned: importing the same file *by
path* works. The rest:

- `طرفية` — loads, then dies at link with `و٠١٠١`: `خط_افقي_حرف` is declared as both a function
  (`اساسي.ترقيم:258`) and a constant (`تنسيق.ترقيم:30`). One rename fixes the module.
- `أخطاء` — does not **parse**: `فهرس.ترقيم:21` says `صدّر صنف خطأ {` and `خطأ` is the boolean-false
  keyword (`ب٠٤٠١`). `اختبار` dies transitively through it.
- `تشفير` and `ضغط` — no `.ترقيم` file exists at all; 100% native.
- Seven flat stub files (`stdlib/نص.ترقيم` …) shadow their own directories and **return lies**:
  `نص.ترقيم:28` is `دالة يحتوي(نص، جزء) -> منطقي { أرجع خطأ }`, `ملفات.ترقيم:15` is
  `دالة اقرأ_ملف(مسار) -> نص { أرجع "" }`. Inert today only because the short-circuit fires first.

### 2.9 Editor drift

The LSP offers `جذر، مطلق، اقرأ_ملف، اكتب_ملف` as no-import completions; none is in
`core_builtins()`. Accepting one of those suggestions produces code the analyzer rejects. The
converse gap: 9 real core builtins (`طباعة، اطبع_خطأ، ادخل_رسالة، تأكد، تأكد_رسالة، توقف، نم،
طول_مصفوفة، الحق`) are never highlighted or completed. `get_builtin_hover` and
`get_builtin_signature_help` are `#[allow(dead_code)]` with no non-test callers — users get no
hover text and no signature help for any builtin at all.

### 2.10 Test coverage

`test_every_core_builtin_agrees_across_backends` (`tests/builtins_execution_tests.rs:904`) drives
`core_builtin_names()` and fails if a **new core builtin** lands without an execution probe — but
excludes four names, so `ادخل، ادخل_رسالة، اطبع_خطأ` have zero execution coverage in any backend.

**No test locks either registry's size.** `core_builtins()` has 18 entries; the only place `18`
appears is a *comment*. Contents are locked one-directionally — removing a core builtin fails,
adding a nineteenth passes silently. The stdlib half is structurally unguardable: `get_stdlib_builtin`
is a `match` with no enumeration accessor.

Of 165 stdlib registry names, **8 (≈5%) have any cross-backend execution assertion**, six of them
through the single example `تشفير_وضغط.ترقيم`. `tests/module_execution_tests.rs:584-631` pins the
native leg of a stdlib import as **broken by inverted assertion** — importing any stdlib function
segfaults natively (exit 139, #185).

`runtime_symbols_tests.rs` guards codegen's `@trq_*` declarations against `runtime-rs` definitions,
but carries an 11-entry `KNOWN_UNDEFINED` allowlist — it is a tracked-hole registry, not
protection — and it guards only the `trq_*` layer, never the Arabic→symbol layer above it.

---

## 3. Reading the table

`ن` semantic (`scope.rs`) · `مف` interpreter · `تن` debug interpreter · `أص` native (IR + codegen).
`✓` present · `✗` absent · `~` present but incomplete (typically: lowered with no IR return type
registered, so the result carries a `Ptr(Void)` sentinel and composing it is silently wrong).

The JIT columns are omitted because they are not independent — both tiers compile zero builtins and
mirror the interpreter.

**Verdicts** — assigned by the classification pass, criteria in
[`builtins-vs-stdlib.md`](builtins-vs-stdlib.md) §1:

| الحكم | Meaning | Count |
|---|---|---|
| `مدمج` | Stays a compiler/runtime primitive | 19 existing (+21 new = the 40-name registry) |
| `مكتبة` | Migrates to self-hosted Tarqeem stdlib | 121 |
| `يُحذف` | Alias collapse, or dead in every backend | 48 |
| `مؤجل` | Sockets — deferred to a separate registry (Increment K) | 47 |

`معطّل اليوم` marks a name that is **declared but non-functional right now** — no interpreter arm,
no native lowering, and no runtime symbol. It is orthogonal to the verdict, which describes the
target state.

**How verdicts were assigned.** 108 names carry an explicit per-name verdict from the
classification pass. The remainder are derived mechanically from the same published rules —
membership in the 40-name registry (`مدمج`), the socket family (`مؤجل`), an alias group recorded in
the dispatch matrix (`يُحذف`), otherwise `مكتبة`, which is the criteria's default. The derivation is
deterministic; no row was assigned by judgement outside those rules.

**Reconciling with the plan document.** Its executive summary counts 26 alias collapses and 11 dead
names against the **193 declared** names; this table counts 48 against the **245 reachable** names,
which additionally covers the 52 that exist in a backend but in no registry. Likewise the plan
defers **12 socket primitives** while this table marks **47 socket-family names** deferred — 12 is
the primitive count after collapse, 47 is the raw name count. Both are correct; they count
different universes.


## 4. The inventory

#### `core` — 48

Rows marked **مُنفَّذ** landed after this census; the backend columns are re-verified,
not carried over from the original pass.

| الاسم | ن | مف | تن | أص | رمز وقت التشغيل | الحكم | ملاحظة |
|---|:-:|:-:|:-:|:-:|---|---|---|
| `أضف` | ✗ | ✗ | ✗ | ✓ | `-` | مكتبة |  |
| `ألحق` | ✓ | ✓ | ✓ | ✓ | `trq_array_push` | مدمج | primitive، **مُنفَّذ** (#375) — وُحِّد الرسم على `ألحق` (أمرُ أَلْحَقَ) في الصيغتين العامة والعضوية، وأزيل `الحق` من غير مرحلة تحذير على سنّة #336 و#368؛ والصيغتان تنزلان إلى `ArrayPush` نفسها فلا ذراع تفريق في أي خلفية |
| `احذف_آخر` | ✓ | ✓ | ✓ | ✓ | `trq_array_pop` | مدمج | primitive، **مُنفَّذ** (#382) — تعليمة وسيطة جديدة `ArrayPop` لا اسمٌ في جدول الرموز، فلا تمرّ بـ`get_runtime_function_name`؛ والصيغتان العامة والعضوية تنزلان إليها، وحلّت العضوية محلّ `احذف` المُزال. وأُلزمت `trq_array_pop` بالكليّة قبل وصلها |
| `ادخل` | ✓ | ✓ | ✓ | ~ | `trq_input` | يُحذف | alias/dead |
| `ادخل_رسالة` | ✓ | ✓ | ✗ | ~ | `trq_input_prompt` | مكتبة |  |
| `اطبع` | ✓ | ✓ | ✓ | ~ | `trq_print, trq_print_optional…` | مدمج | primitive |
| `اطبع_خطأ` | ✓ | ✓ | ✗ | ~ | `trq_print_error` | مدمج | primitive |
| `اطبع_سطر` | ✓ | ✓ | ✗ | ~ | `trq_print` | يُحذف | alias/dead |
| `اقرأ_سطر` | ✗ | ✓ | ✗ | ✗ | `-` | مكتبة |  |
| `بتات_أو` | ✓ | ✓ | ✓ | ✓ | `-` | مدمج | primitive، **مُنفَّذ** (#306) |
| `بتات_أو_حصري` | ✓ | ✓ | ✓ | ✓ | `-` | مدمج | primitive، **مُنفَّذ** (#309) |
| `بتات_إزاحة_يسار` | ✓ | ✓ | ✓ | ✓ | `-` | مدمج | primitive، **مُنفَّذ** (#317) |
| `بتات_إزاحة_يمين` | ✓ | ✓ | ✓ | ✓ | `-` | مدمج | primitive، **مُنفَّذ** (#320) |
| `بتات_إزاحة_يمين_منطقية` | ✓ | ✓ | ✓ | ✓ | `-` | مدمج | primitive، **مُنفَّذ** (#322) |
| `بتات_نفي` | ✓ | ✓ | ✓ | ✓ | `-` | مدمج | primitive، **مُنفَّذ** (#312) |
| `بتات_و` | ✓ | ✓ | ✓ | ✓ | `-` | مدمج | primitive، **مُنفَّذ** (#302) |
| `ثنائي_إلى_نص` | ✓ | ✓ | ✓ | ✓ | `trq_string_from_bytes` | مدمج | primitive، **مُنفَّذ** (#333) |
| `قص_حروف` | ✓ | ✓ | ✓ | ✓ | `trq_string_substr_chars` | مدمج | primitive، **مُنفَّذ** (#336) — نُقل من `نص` |
| `متغير_بيئة` | ✓ | ✓ | ✓ | ✓ | `trq_env_get` | مدمج | primitive، **مُنفَّذ** (#338) |
| `اكتب_مجرى` | ✓ | ✓ | ✓ | ✓ | `trq_write_stream` | مدمج | primitive، **مُنفَّذ** (#347) — أول عمليات الإدخال/الإخراج |
| `اقرأ_مجرى` | ✓ | ✓ | ✓ | ✓ | `trq_read_stream` | مدمج | primitive، **مُنفَّذ** (#350) — النصف القارئ من زوج المجرى |
| `حالة_مسار` | ✓ | ✓ | ✓ | ✓ | `trq_path_status` | مدمج | primitive، **مُنفَّذ** (#352) — أول ما يسأل نظام الملفات، وتُجمَع فيها أربعة أسماء من `ملفات` بلا حذف أحدها |
| `احذف_مسار` | ✓ | ✓ | ✓ | ✓ | `trq_path_delete` | مدمج | primitive، **مُنفَّذ** (#355) — أول ما يُغيّر نظام الملفات، و`lstat` لا `stat`: تعمل على الاسم حيث تسأل أختها عن الهدف |
| `معاملات_البرنامج` | ✓ | ✓ | ✓ | ✓ | `trq_program_args` | مدمج | primitive، **مُنفَّذ** (#360) — أول ما يقرأ ما سُئل عنه البرنامج، وبها تكتمل أداة سطر الأوامر مع `أنهِ_البرنامج` |
| `افتح_ملف` | ✓ | ✓ | ✓ | ✓ | `trq_file_open` | مدمج | primitive، **مُنفَّذ** (#362) — أول ما يُنشئ معرِّفاً يبقى بعده، وبها بلغ زوج المجاري ما ليس طرفية |
| `اغلق_ملف` | ✓ | ✓ | ✓ | ✓ | `trq_file_close` | مدمج | primitive، **مُنفَّذ** (#364) — نصفها الآخر، وبها يبلغ المكتوبُ الملفَ قبل نهاية البرنامج لا عندها |
| `انشئ_مجلد` | ✓ | ✓ | ✓ | ✓ | `trq_dir_create` | مدمج | primitive، **مُنفَّذ** (#366) — نُقلت من `ملفات` على سنّة `قص_حروف` (#336)، وبها يكتمل ثالوث المسار: `حالة_مسار` تسأل و`احذف_مسار` تُزيل وهذه تُنشئ، غيرَ متعدية |
| `انقل_مسار` | ✓ | ✓ | ✓ | ✓ | `trq_file_move` | مدمج | primitive، **مُنفَّذ** (#368) — نُقلت من `ملفات` وسُمّيت `انقل_مسار` على سنّة `حالة_مسار` (#352)، إذ `rename(2)` على الاسم في الطرفين وتنقل المجلد والوصلة أيضاً؛ والوجهة القائمة لا تُستبدل إلا ملفاً اعتيادياً، فتبقى الكتابة الذرّية ويتفق الجواب بين المنصات |
| `قائمة_مجلد` | ✓ | ✓ | ✓ | ✓ | `trq_dir_list` | مدمج | primitive، **مُنفَّذ** (#370) — نُقلت من `ملفات` بالاسم نفسه، فاكتملت بها إصلاحات المرتبة السابعة؛ والجواب أسماء مجردة مرتّبةً بترتيب الرموز — ترتيب readdir الخام يختلف بين أنظمة الملفات فلا يصلح في عقد — والاسم الفاسد يُقرأ متساهلاً ولا يُسقَط، والمصفوفة الفارغة جواب كل رفض، لا يتميّز من المجلد الفارغ قصداً |
| `مجلد_حالي` | ✓ | ✓ | ✓ | ✓ | `trq_dir_current` | مدمج | primitive، **مُنفَّذ** (#373) — نُقلت من `ملفات` بالاسم نفسه: آخر اسم حكمُه `مدمج` بقي في تلك الوحدة، وأول نقلٍ من المرتبة الثامنة. حالُ العملية لا بيئتُها — «PWD» لا تبلغه `متغير_بيئة` — والجواب حرفيٌّ كما يُبلّغ به النظام، متساهل القراءة، و`""` رفضُها الذي لا شريك له في المعنى |
| `أنهِ_البرنامج` | ✓ | ✓ | ✓ | ✓ | `trq_exit` | مدمج | primitive، **مُنفَّذ** (#342) — الوحيدة بلا نوع إرجاع مسجَّل، قصداً |
| `أنه_البرنامج` | ✓ | ✓ | ✓ | ✓ | `trq_exit` | مدمج | هجاء ثانٍ للسابقة، لا عملية ثانية |
| `تأكد` | ✓ | ✓ | ✗ | ✓ | `trq_assert` | مكتبة |  |
| `تأكد_رسالة` | ✓ | ✓ | ✗ | ✓ | `-` | مكتبة |  |
| `توقف` | ✓ | ✓ | ✗ | ~ | `trq_panic` | مدمج | primitive |
| `حرف_إلى_رمز` | ✓ | ✓ | ✓ | ✓ | `trq_string_char_code` | مدمج | primitive، **مُنفَّذ** (#324) |
| `رمز_إلى_حرف` | ✓ | ✓ | ✓ | ✓ | `trq_string_from_char_code` | مدمج | primitive، **مُنفَّذ** (#326) |
| `نص_إلى_ثنائي` | ✓ | ✓ | ✓ | ✓ | `trq_string_to_bytes` | مدمج | primitive، **مُنفَّذ** (#330) |
| `طباعة` | ✓ | ✓ | ✗ | ~ | `trq_print` | يُحذف | alias/dead |
| `طول` | ✓ | ✓ | ✓ | ✓ | `trq_array_len, trq_string_len…` | مدمج | primitive |
| `طول_مصفوفة` | ✓ | ✗ | ✗ | ~ | `trq_array_len` | يُحذف | alias/dead |
| `عدد` | ✓ | ✓ | ✓ | ✓ | `trq_string_to_int_checked` | مدمج | primitive |
| `عدد_عشري` | ✓ | ✓ | ✓ | ✓ | `trq_string_to_float_checked` | مدمج | primitive |
| `منطقي` | ✓ | ✓ | ✓ | ✓ | `trq_string_len` | مدمج | primitive |
| `نص` | ✓ | ✓ | ✓ | ✓ | `trq_int_to_string` | مدمج | primitive |
| `نم` | ✓ | ✓ | ✗ | ~ | `trq_sleep` | مدمج | primitive |
| `نوع` | ✓ | ✓ | ✓ | ✓ | `-` | مدمج | primitive |
| `وقت_أداء` | ✓ | ✓ | ✓ | ✓ | `trq_performance_now` | مدمج | primitive، **مُنفَّذ** (#389) — أول زيادة موضوعها **جسمٌ** لا اسم: كانت تقرأ ساعة الحائط، فتتراجع عند ضبط الوقت، والخلفيات الثلاث متفقةٌ على الجواب الخاطئ — وذاك ما لا يراه فحص التباين. وأصلها اليوم نقطةٌ داخل العملية تُضبَط عند أول نداء، لأن مكتبة وقت التشغيل وحدها لها خطّاف تهيئة. ونُقلت من `وقت` لأن مثال المدمجات بلا استيراد، فلم يكن للساعة المُصلَحة موضعٌ تُختبَر فيه |

#### `رياضيات` — 70

| الاسم | ن | مف | تن | أص | رمز وقت التشغيل | الحكم | ملاحظة |
|---|:-:|:-:|:-:|:-:|---|---|---|
| `أدنى` | ✓ | ✓ | ✗ | ~ | `trq_min_float` | يُحذف | alias collapse |
| `أرضية` | ✓ | ✓ | ✓ | ~ | `trq_floor` | مكتبة |  |
| `أس` | ✓ | ✓ | ✗ | ~ | `trq_exp` | مكتبة |  |
| `أسي` | ✓ | ✓ | ✗ | ~ | `trq_exp` | يُحذف | alias collapse |
| `أقصى` | ✓ | ✓ | ✗ | ~ | `trq_max_float` | يُحذف | alias collapse |
| `أقل` | ✓ | ✓ | ✗ | ~ | `trq_min_float` | مكتبة |  |
| `أقل_عدد` | ✓ | ✓ | ✗ | ~ | `trq_min_int` | يُحذف | alias collapse |
| `أكبر` | ✓ | ✓ | ✗ | ~ | `trq_max_float` | مكتبة |  |
| `أكبر_عدد` | ✓ | ✓ | ✗ | ~ | `trq_max_int` | يُحذف | alias collapse |
| `اقتطع` | ✓ | ✓ | ✗ | ~ | `trq_trunc` | مكتبة |  |
| `الى_درجات` | ✓ | ✓ | ✗ | ~ | `trq_to_degrees` | مكتبة |  |
| `الى_راديان` | ✓ | ✓ | ✗ | ~ | `trq_to_radians` | مكتبة |  |
| `باقي` | ✓ | ✗ | ✗ | ~ | `trq_mod` | مكتبة |  |
| `بذرة_عشوائي` | ✓ | ✗ | ✗ | ~ | `trq_random_seed` | مكتبة |  |
| `بذرة_عشوائية` | ✓ | ✗ | ✗ | ✗ | `-` | مكتبة | **معطّل اليوم** |
| `تقريب` | ✓ | ✓ | ✗ | ~ | `trq_round` | يُحذف | alias collapse |
| `جا` | ✓ | ✓ | ✗ | ~ | `trq_sin` | مكتبة |  |
| `جا_زائدي` | ✓ | ✓ | ✗ | ~ | `trq_sinh` | مكتبة |  |
| `جا_عكسي` | ✓ | ✓ | ✗ | ~ | `trq_asin` | مكتبة |  |
| `جتا` | ✓ | ✓ | ✗ | ~ | `trq_cos` | مكتبة |  |
| `جتا_زائدي` | ✓ | ✓ | ✗ | ~ | `trq_cosh` | مكتبة |  |
| `جتا_عكسي` | ✓ | ✓ | ✗ | ~ | `trq_acos` | مكتبة |  |
| `جذر` | ✓ | ✓ | ✓ | ✓ | `trq_sqrt` | مدمج | primitive |
| `جذر_تكعيبي` | ✓ | ✓ | ✗ | ~ | `trq_cbrt` | مكتبة |  |
| `جيب` | ✓ | ✓ | ✓ | ~ | `trq_sin` | يُحذف | alias collapse |
| `جيب_التمام` | ✓ | ✓ | ✓ | ~ | `trq_cos` | يُحذف | alias collapse |
| `جيب_تمام_زائدي` | ✗ | ✓ | ✗ | ✗ | `-` | يُحذف | alias collapse |
| `جيب_تمام_عكسي` | ✓ | ✓ | ✗ | ~ | `trq_acos` | يُحذف | alias collapse |
| `جيب_زائدي` | ✗ | ✓ | ✗ | ✗ | `-` | يُحذف | alias collapse |
| `جيب_عكسي` | ✓ | ✓ | ✗ | ~ | `trq_asin` | يُحذف | alias collapse |
| `حصر` | ✓ | ✓ | ✗ | ~ | `trq_clamp_float` | مكتبة |  |
| `حصر_عدد` | ✓ | ✓ | ✗ | ~ | `trq_clamp_int` | يُحذف | alias collapse |
| `درجات` | ✓ | ✓ | ✗ | ~ | `trq_to_degrees` | يُحذف | alias collapse |
| `راديان` | ✓ | ✓ | ✗ | ~ | `trq_to_radians` | يُحذف | alias collapse |
| `سقف` | ✓ | ✓ | ✓ | ~ | `trq_ceil` | مكتبة |  |
| `ظا` | ✓ | ✓ | ✗ | ~ | `trq_tan` | مكتبة |  |
| `ظا_زائدي` | ✓ | ✓ | ✗ | ~ | `trq_tanh` | مكتبة |  |
| `ظا_عكسي` | ✓ | ✓ | ✗ | ~ | `trq_atan` | مكتبة |  |
| `ظا_عكسي2` | ✓ | ✓ | ✗ | ~ | `trq_atan2` | مكتبة |  |
| `ظتا` | ✓ | ✓ | ✗ | ~ | `trq_cot` | مكتبة |  |
| `ظل` | ✓ | ✓ | ✓ | ~ | `trq_tan` | يُحذف | alias collapse |
| `ظل_التمام` | ✓ | ✓ | ✗ | ~ | `trq_cot` | يُحذف | alias collapse |
| `ظل_زائدي` | ✗ | ✓ | ✗ | ✗ | `-` | يُحذف | alias collapse |
| `ظل_عكسي` | ✗ | ✓ | ✗ | ✗ | `-` | يُحذف | alias collapse |
| `ظل_عكسي2` | ✗ | ✓ | ✗ | ✗ | `-` | يُحذف | alias collapse |
| `عاملي` | ✓ | ✓ | ✗ | ~ | `trq_factorial` | مكتبة |  |
| `عشوائي` | ✓ | ✓ | ✗ | ✗ | `-` | مكتبة |  |
| `عشوائي_بين` | ✓ | ✓ | ✗ | ✗ | `-` | مكتبة |  |
| `عشوائي_عدد` | ✓ | ✓ | ✗ | ~ | `trq_random_int` | يُحذف | alias collapse |
| `عشوائي_عدد_بين` | ✓ | ✓ | ✗ | ~ | `trq_random_int_range` | يُحذف | alias collapse |
| `عشوائي_عشري` | ✓ | ✓ | ✗ | ~ | `trq_random_float` | مكتبة |  |
| `عشوائي_عشري_بين` | ✓ | ✓ | ✗ | ~ | `trq_random_float_range` | يُحذف | alias collapse |
| `عشوائي_منطقي` | ✓ | ✓ | ✗ | ~ | `trq_random_bool` | مكتبة |  |
| `علامة` | ✓ | ✓ | ✗ | ~ | `trq_sign` | مكتبة |  |
| `قا` | ✓ | ✓ | ✗ | ~ | `trq_sec` | مكتبة |  |
| `قاسم_مشترك` | ✓ | ✓ | ✗ | ~ | `trq_gcd` | مكتبة |  |
| `قاطع` | ✓ | ✓ | ✗ | ~ | `trq_sec` | يُحذف | alias collapse |
| `قاطع_التمام` | ✓ | ✓ | ✗ | ~ | `trq_csc` | يُحذف | alias collapse |
| `قتا` | ✓ | ✓ | ✗ | ~ | `trq_csc` | مكتبة |  |
| `قرب` | ✗ | ✓ | ✓ | ✗ | `-` | مكتبة |  |
| `قرّب` | ✓ | ✓ | ✗ | ~ | `trq_round` | يُحذف | alias collapse |
| `قوة` | ✓ | ✓ | ✗ | ~ | `trq_pow_float` | مكتبة |  |
| `قوة_عدد` | ✓ | ✓ | ✗ | ~ | `trq_pow_int` | يُحذف | alias collapse |
| `لوغ10` | ✓ | ✓ | ✗ | ~ | `trq_log10` | مكتبة |  |
| `لوغ2` | ✓ | ✓ | ✗ | ~ | `trq_log2` | مكتبة |  |
| `لوغاريتم` | ✓ | ✓ | ✗ | ~ | `trq_log` | مكتبة |  |
| `لوغاريتم10` | ✓ | ✓ | ✗ | ~ | `trq_log10` | يُحذف | alias collapse |
| `مضاعف_مشترك` | ✓ | ✓ | ✗ | ~ | `trq_lcm` | مكتبة |  |
| `مطلق` | ✓ | ✓ | ✓ | ~ | `trq_abs_float` | مكتبة |  |
| `مطلق_عدد` | ✓ | ✓ | ✗ | ~ | `trq_abs_int` | يُحذف | alias collapse |

#### `نص` — 39

| الاسم | ن | مف | تن | أص | رمز وقت التشغيل | الحكم | ملاحظة |
|---|:-:|:-:|:-:|:-:|---|---|---|
| `احشو_يسار` | ✓ | ✗ | ✗ | ~ | `trq_string_pad_left` | مكتبة |  |
| `احشو_يمين` | ✓ | ✗ | ✗ | ~ | `trq_string_pad_right` | مكتبة |  |
| `ادخل_عدد` | ✓ | ✓ | ✗ | ~ | `trq_input_int` | مكتبة |  |
| `ادخل_عشري` | ✓ | ✓ | ✗ | ~ | `trq_input_float` | مكتبة |  |
| `ادمج` | ✓ | ✗ | ✗ | ~ | `trq_string_join` | مكتبة |  |
| `ازل_فراغات` | ✓ | ✗ | ✗ | ~ | `trq_string_trim` | مكتبة |  |
| `ازل_فراغات_يسار` | ✓ | ✗ | ✗ | ~ | `trq_string_trim_left` | مكتبة |  |
| `ازل_فراغات_يمين` | ✓ | ✗ | ✗ | ~ | `trq_string_trim_right` | مكتبة |  |
| `استبدل` | ✓ | ✗ | ✗ | ~ | `trq_string_replace` | مكتبة |  |
| `استبدل_كل` | ✓ | ✗ | ✗ | ~ | `trq_string_replace_all` | مكتبة |  |
| `اعكس_نص` | ✓ | ✗ | ✗ | ~ | `trq_string_reverse` | مكتبة |  |
| `حرف_في` | ✓ | ✗ | ✗ | ~ | `trq_string_char_at` | مكتبة |  |
| `حروف_فقط` | ✓ | ✗ | ✗ | ~ | `trq_string_is_alpha` | مكتبة |  |
| `رقمي` | ✓ | ✗ | ✗ | ~ | `trq_string_is_numeric` | مكتبة |  |
| `صغير` | ✓ | ✗ | ✗ | ~ | `trq_string_to_lower` | مكتبة |  |
| `طول_حروف` | ✓ | ✗ | ✗ | ~ | `trq_string_len_chars` | مكتبة |  |
| `طول_نص` | ✓ | ✗ | ✗ | ~ | `trq_string_len` | يُحذف | alias/dead |
| `عدد_لنص` | ✓ | ✓ | ✗ | ~ | `trq_int_to_string` | مكتبة |  |
| `عدد_مرات` | ✓ | ✗ | ✗ | ~ | `trq_string_count` | مكتبة |  |
| `عربي` | ✓ | ✗ | ✗ | ~ | `trq_string_is_arabic` | مكتبة |  |
| `عشري_لنص` | ✓ | ✓ | ✗ | ~ | `trq_float_to_string` | مكتبة |  |
| `عنوان` | ✓ | ✗ | ✗ | ~ | `trq_string_to_title` | مكتبة |  |
| `قارن_نص` | ✓ | ✗ | ✗ | ~ | `trq_string_compare` | مكتبة |  |
| `قسّم` | ✓ | ✗ | ✗ | ~ | `trq_string_split` | مكتبة |  |
| `كبير` | ✓ | ✗ | ✗ | ~ | `trq_string_to_upper` | مكتبة |  |
| `كرر` | ✓ | ✗ | ✗ | ✗ | `-` | يُحذف | alias/dead، **معطّل اليوم** |
| `كرر_نص` | ✓ | ✗ | ✗ | ~ | `trq_string_repeat` | مكتبة |  |
| `منطقي_لنص` | ✓ | ✓ | ✗ | ~ | `trq_bool_to_string` | مكتبة |  |
| `موضع` | ✓ | ✗ | ✗ | ~ | `trq_string_index_of` | مكتبة |  |
| `موضع_اخير` | ✓ | ✗ | ✗ | ~ | `trq_string_last_index_of` | مكتبة |  |
| `نص_لعدد` | ✓ | ✗ | ✗ | ~ | `trq_string_to_int` | مكتبة |  |
| `نص_لعشري` | ✓ | ✗ | ✗ | ~ | `trq_string_to_float` | مكتبة |  |
| `نص_يبدأ_بـ` | ✓ | ✓ | ✗ | ✗ | `-` | يُحذف | alias/dead |
| `نص_يحتوي` | ✓ | ✓ | ✗ | ✗ | `-` | يُحذف | alias/dead |
| `نص_ينتهي_بـ` | ✓ | ✓ | ✗ | ✗ | `-` | يُحذف | alias/dead |
| `نصوص_متساوية` | ✓ | ✗ | ✗ | ~ | `trq_string_equals` | مكتبة |  |
| `يبدأ_بـ` | ✓ | ✗ | ✗ | ~ | `trq_string_starts_with` | مكتبة |  |
| `يحتوي` | ✓ | ✗ | ✗ | ~ | `trq_string_contains` | مكتبة |  |
| `ينتهي_بـ` | ✓ | ✗ | ✗ | ~ | `trq_string_ends_with` | مكتبة |  |

#### `ملفات` — 17

| الاسم | ن | مف | تن | أص | رمز وقت التشغيل | الحكم | ملاحظة |
|---|:-:|:-:|:-:|:-:|---|---|---|
| `احذف_مجلد` | ✓ | ✗ | ✗ | ~ | `trq_dir_delete` | مكتبة | غلافٌ على `احذف_مسار` (#355)، بفرقٍ موثَّق عند الوصلة |
| `احذف_ملف` | ✓ | ✗ | ✗ | ~ | `trq_file_delete` | مكتبة | غلافٌ على `احذف_مسار` (#355)، بفرقٍ موثَّق عند الوصلة |
| `ادمج_مسار` | ✓ | ✗ | ✗ | ~ | `trq_path_join` | مكتبة |  |
| `اسم_ملف` | ✓ | ✗ | ✗ | ~ | `trq_path_filename` | مكتبة |  |
| `اقرأ_ملف` | ✓ | ✓ | ✗ | ✓ | `trq_file_read` | مكتبة |  |
| `اكتب_ملف` | ✓ | ✓ | ✗ | ✓ | `trq_file_write` | مكتبة |  |
| `الحق_ملف` | ✓ | ✗ | ✗ | ~ | `trq_file_append` | مكتبة |  |
| `امتداد_ملف` | ✓ | ✗ | ✗ | ~ | `trq_path_extension` | مكتبة |  |
| `انسخ_ملف` | ✓ | ✗ | ✗ | ~ | `trq_file_copy` | مكتبة |  |
| `حجم_ملف` | ✓ | ✗ | ✗ | ~ | `trq_file_size` | مكتبة | حقلٌ من `حالة_مسار` (#352) |
| `فاصل_مسار` | ✓ | ✗ | ✗ | ~ | `trq_path_separator` | مكتبة |  |
| `مجلد_مؤقت` | ✓ | ✗ | ✗ | ~ | `trq_dir_temp` | مكتبة |  |
| `مجلد_مستخدم` | ✓ | ✗ | ✗ | ~ | `trq_dir_home` | مكتبة |  |
| `مسار_اب` | ✓ | ✗ | ✗ | ~ | `trq_path_parent` | مكتبة |  |
| `ملف_موجود` | ✓ | ✗ | ✗ | ~ | `trq_file_exists` | مكتبة | حقلٌ من `حالة_مسار` (#352) |
| `هل_مجلد` | ✓ | ✗ | ✗ | ~ | `trq_file_is_dir` | مكتبة | حقلٌ من `حالة_مسار` (#352) |
| `هل_ملف` | ✓ | ✗ | ✗ | ~ | `trq_file_is_file` | مكتبة | حقلٌ من `حالة_مسار` (#352) |

#### `وقت` — 17

| الاسم | ن | مف | تن | أص | رمز وقت التشغيل | الحكم | ملاحظة |
|---|:-:|:-:|:-:|:-:|---|---|---|
| `أضف_أشهر` | ✗ | ✗ | ✗ | ✗ | `trq_date_add_months` | يُحذف | alias/dead |
| `أضف_أيام` | ✗ | ✗ | ✗ | ✗ | `trq_date_add_days` | يُحذف | alias/dead |
| `أيام_الشهر` | ✗ | ✗ | ✗ | ✓ | `trq_days_in_month` | مكتبة |  |
| `تاريخ_اليوم` | ✗ | ✗ | ✗ | ✗ | `trq_date_today` | يُحذف | alias/dead |
| `تاريخ_من_طابع` | ✗ | ✗ | ✗ | ✗ | `trq_date_from_timestamp` | يُحذف | alias/dead |
| `تاريخ_ووقت_من_طابع` | ✗ | ✗ | ✗ | ✗ | `trq_datetime_from_timestamp` | يُحذف | alias/dead |
| `حلل_تاريخ` | ✗ | ✗ | ✗ | ✗ | `trq_date_parse` | يُحذف | alias/dead |
| `حلل_تاريخ_ووقت` | ✗ | ✗ | ✗ | ✗ | `trq_datetime_parse` | يُحذف | alias/dead |
| `حلل_وقت` | ✗ | ✗ | ✗ | ✗ | `trq_time_parse` | يُحذف | alias/dead |
| `رقم_الأسبوع` | ✗ | ✗ | ✗ | ✓ | `trq_week_number` | مكتبة |  |
| `فرق_أيام` | ✗ | ✗ | ✗ | ✓ | `trq_date_diff_days` | مكتبة |  |
| `نسّق_تاريخ` | ✗ | ✗ | ✗ | ✓ | `trq_date_format` | مكتبة |  |
| `نسّق_تاريخ_ووقت` | ✗ | ✗ | ✗ | ✓ | `trq_datetime_format` | مكتبة |  |
| `نسّق_وقت` | ✗ | ✗ | ✗ | ✓ | `trq_time_format` | مكتبة |  |
| `وقت_الآن` | ✓ | ✓ | ✓ | ✓ | `trq_time_now` | مدمج | primitive — آخر اسم `مدمج` بقي في هذه الوحدة، بعد نقل `وقت_أداء` إلى المرتبة الأساسية في #389 |
| `يوم_الأسبوع` | ✗ | ✗ | ✗ | ✓ | `trq_day_of_week` | مكتبة |  |
| `يوم_السنة` | ✗ | ✗ | ✗ | ✓ | `trq_day_of_year` | مكتبة |  |

#### `تشفير` — 10

| الاسم | ن | مف | تن | أص | رمز وقت التشغيل | الحكم | ملاحظة |
|---|:-:|:-:|:-:|:-:|---|---|---|
| `إلى_ست_عشري` | ✓ | ✓ | ✗ | ✓ | `trq_hex_encode` | مكتبة |  |
| `احسب_بصمة` | ✓ | ✓ | ✗ | ✓ | `trq_sha256_string` | مكتبة |  |
| `بصمة_ثنائي` | ✓ | ✓ | ✗ | ✓ | `trq_sha256_bytes` | مكتبة |  |
| `بصمة_ملف` | ✓ | ✓ | ✗ | ✓ | `trq_sha256_file` | مكتبة |  |
| `ترميز_أساس64` | ✗ | ✗ | ✗ | ✓ | `trq_base64_encode` | مكتبة |  |
| `ثنائي_إلى_ست_عشري` | ✓ | ✓ | ✗ | ✓ | `trq_hex_encode_bytes` | مكتبة |  |
| `ست_عشري_إلى_ثنائي` | ✓ | ✓ | ✗ | ✓ | `trq_hex_decode_to_bytes` | مكتبة |  |
| `طابق_بصمة` | ✓ | ✓ | ✗ | ✓ | `trq_sha256_compare` | مكتبة |  |
| `فك_أساس64` | ✗ | ✗ | ✗ | ✓ | `trq_base64_decode` | مكتبة |  |
| `من_ست_عشري` | ✓ | ✓ | ✗ | ✓ | `trq_hex_decode` | مكتبة |  |

#### `ضغط` — 6

| الاسم | ن | مف | تن | أص | رمز وقت التشغيل | الحكم | ملاحظة |
|---|:-:|:-:|:-:|:-:|---|---|---|
| `اضغط` | ✓ | ✓ | ✗ | ✓ | `trq_gzip_compress_string` | مكتبة |  |
| `اضغط_ثنائي` | ✓ | ✓ | ✗ | ✓ | `trq_gzip_compress_bytes` | مكتبة |  |
| `اضغط_ملف` | ✓ | ✓ | ✗ | ✓ | `trq_gzip_compress_file` | مكتبة |  |
| `فك_الضغط` | ✓ | ✓ | ✗ | ✓ | `trq_gzip_decompress_to_string` | مكتبة |  |
| `فك_ضغط_ثنائي` | ✓ | ✓ | ✗ | ✓ | `trq_gzip_decompress_bytes` | مكتبة |  |
| `فك_ضغط_ملف` | ✓ | ✓ | ✗ | ✓ | `trq_gzip_decompress_file` | مكتبة |  |

#### `شبكة` — 47

| الاسم | ن | مف | تن | أص | رمز وقت التشغيل | الحكم | ملاحظة |
|---|:-:|:-:|:-:|:-:|---|---|---|
| `أرسل` | ✓ | ✗ | ✗ | ~ | `trq_tcp_send` | مؤجل | deferred |
| `أرسل_إلى` | ✓ | ✗ | ✗ | ~ | `trq_udp_send_to` | مؤجل | deferred |
| `أرسل_بايتات` | ✓ | ✗ | ✗ | ~ | `trq_tcp_send_bytes` | مؤجل | deferred |
| `أغلق_اتصال` | ✓ | ✗ | ✗ | ~ | `trq_tcp_close` | مؤجل | deferred |
| `اتصل_خادم` | ✓ | ✗ | ✗ | ~ | `trq_tcp_connect` | مؤجل | deferred |
| `احصل_عنوان_محلي` | ✗ | ✗ | ✗ | ~ | `trq_get_local_ip` | مؤجل | deferred |
| `احصل_ويب` | ✓ | ✗ | ✗ | ~ | `trq_http_get` | مؤجل | deferred |
| `ارتبط_منفذ` | ✓ | ✗ | ✗ | ~ | `trq_udp_bind` | مؤجل | deferred |
| `استقبل` | ✓ | ✗ | ✗ | ~ | `trq_tcp_receive` | مؤجل | deferred |
| `استقبل_بايتات` | ✓ | ✗ | ✗ | ~ | `trq_tcp_receive_bytes` | مؤجل | deferred |
| `استقبل_حتى` | ✓ | ✗ | ✗ | ~ | `trq_tcp_receive_until` | مؤجل | deferred |
| `استقبل_من` | ✓ | ✗ | ✗ | ~ | `trq_udp_receive` | مؤجل | deferred |
| `استمع` | ✓ | ✗ | ✗ | ~ | `trq_tcp_listen` | مؤجل | deferred |
| `اقبل_اتصال` | ✓ | ✗ | ✗ | ~ | `trq_tcp_accept` | مؤجل | deferred |
| `حزم_اربط` | ✗ | ✗ | ✗ | ~ | `trq_udp_bind` | مؤجل | deferred |
| `حزم_ارسل_الى` | ✗ | ✗ | ✗ | ~ | `trq_udp_send_to` | مؤجل | deferred |
| `حزم_ارسل_بايتات_الى` | ✗ | ✗ | ✗ | ~ | `trq_udp_send_bytes_to` | مؤجل | deferred |
| `حزم_ارسل_رد` | ✗ | ✗ | ✗ | ~ | `trq_udp_reply` | مؤجل | deferred |
| `حزم_استقبل` | ✗ | ✗ | ✗ | ~ | `trq_udp_receive` | مؤجل | deferred |
| `حزم_استقبل_بايتات` | ✗ | ✗ | ✗ | ~ | `trq_udp_receive_bytes` | مؤجل | deferred |
| `حزم_اغلق` | ✗ | ✗ | ✗ | ~ | `trq_udp_close` | مؤجل | deferred |
| `حل_اسم_نطاق` | ✓ | ✗ | ✗ | ~ | `trq_resolve_hostname` | مؤجل | deferred |
| `حل_عنوان` | ✗ | ✗ | ✗ | ~ | `trq_resolve_hostname` | مؤجل | deferred |
| `حمّل_ملف` | ✓ | ✗ | ✗ | ~ | `trq_http_download` | مؤجل | deferred |
| `حمّل_ويب` | ✗ | ✗ | ✗ | ~ | `trq_http_download` | مؤجل | deferred |
| `رد` | ✓ | ✗ | ✗ | ~ | `trq_udp_reply` | مؤجل | deferred |
| `رمّز_رابط` | ✓ | ✗ | ✗ | ~ | `trq_url_encode` | مؤجل | deferred |
| `طلب_ويب` | ✓ | ✗ | ✗ | ~ | `trq_http_request` | مؤجل | deferred |
| `عنوان_محلي` | ✓ | ✗ | ✗ | ~ | `trq_tcp_local_address` | مؤجل | deferred |
| `عنوان_محلي_للجهاز` | ✓ | ✗ | ✗ | ~ | `trq_get_local_ip` | مؤجل | deferred |
| `فك_ترميز_رابط` | ✓ | ✗ | ✗ | ~ | `trq_url_decode` | مؤجل | deferred |
| `فك_رمز_رابط` | ✗ | ✗ | ✗ | ~ | `trq_url_decode` | مؤجل | deferred |
| `منفذ_محلي` | ✓ | ✗ | ✗ | ~ | `trq_tcp_local_port` | مؤجل | deferred |
| `نقل_اتصل` | ✗ | ✗ | ✗ | ~ | `trq_tcp_connect` | مؤجل | deferred |
| `نقل_ارسل` | ✗ | ✗ | ✗ | ~ | `trq_tcp_send` | مؤجل | deferred |
| `نقل_ارسل_بايتات` | ✗ | ✗ | ✗ | ~ | `trq_tcp_send_bytes` | مؤجل | deferred |
| `نقل_استقبل` | ✗ | ✗ | ✗ | ~ | `trq_tcp_receive` | مؤجل | deferred |
| `نقل_استقبل_بايتات` | ✗ | ✗ | ✗ | ~ | `trq_tcp_receive_bytes` | مؤجل | deferred |
| `نقل_استقبل_حتى` | ✗ | ✗ | ✗ | ~ | `trq_tcp_receive_until` | مؤجل | deferred |
| `نقل_استمع` | ✗ | ✗ | ✗ | ~ | `trq_tcp_listen` | مؤجل | deferred |
| `نقل_اغلق` | ✗ | ✗ | ✗ | ~ | `trq_tcp_close` | مؤجل | deferred |
| `نقل_اقبل` | ✗ | ✗ | ✗ | ~ | `trq_tcp_accept` | مؤجل | deferred |
| `نقل_اقبل_مع_مهلة` | ✗ | ✗ | ✗ | ~ | `trq_tcp_accept_timeout` | مؤجل | deferred |
| `نقل_بيانات_متاحة` | ✗ | ✗ | ✗ | ~ | `trq_tcp_available` | مؤجل | deferred |
| `نقل_عنوان_محلي` | ✗ | ✗ | ✗ | ~ | `trq_tcp_local_address` | مؤجل | deferred |
| `نقل_منفذ_محلي` | ✗ | ✗ | ✗ | ~ | `trq_tcp_local_port` | مؤجل | deferred |
| `هل_متاح` | ✓ | ✗ | ✗ | ~ | `trq_tcp_available` | مؤجل | deferred |

#### `طرفية` — 1

| الاسم | ن | مف | تن | أص | رمز وقت التشغيل | الحكم | ملاحظة |
|---|:-:|:-:|:-:|:-:|---|---|---|
| `اطبع_منسق` | ✗ | ✗ | ✗ | ~ | `trq_print` | مكتبة |  |
