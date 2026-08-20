# AI Implementation Notes

Decisions and discoveries recorded by AI-assisted sessions, newest first.

## 2026-08-18 — `ثنائي_إلى_نص`, and a plan row whose contract could not be built (#333)

Increment B's fourth name, closing blocker **B9**: the byte bridge is now total in both directions.
The nine registration sites cost nothing new — `trq_sha256_bytes(*const TrqArray) -> *mut TrqString`
is this signature exactly, so like #330 the "first" was only a first for its tier. The expensive part
was the contract.

### The row asked for something no implementation can satisfy

`docs/builtins-vs-stdlib.md` §1.3 required that `ثنائي_إلى_نص` **not validate UTF-8**, so a socket or
file read would round-trip arbitrary octets. It cannot:

- The interpreter holds a string as `Value::String(Rc<String>)` — a Rust `String`, which cannot *be*
  invalid UTF-8. There is no value to construct.
- Natively there is, and `trq_print` is `if let Ok(text) = std::str::from_utf8(slice)`, so it prints
  **nothing**, with no error and exit 0.

So the non-validating version is representable in one backend and not the other, and the observable
difference is silent — the documented recurring failure mode. Honoring the clause needs a
value-representation change, forbidden by §9. The requirement is withdrawn in the document rather
than quietly mis-built, and the contract shipped as: *the string whose UTF-8 encoding is exactly
those bytes, and `""` when there is none.*

Truncating an out-of-range element to its low byte — the house convention in `trq_sha256_bytes` and
`trq_hex_encode_bytes` — was rejected for the same reason in miniature: `[300]` would answer `","`,
colliding with a legitimate `[44]`, and the caller would have no signal. Rejection keeps the failure
**detectable**: an empty result from a non-empty array can only be a rejection. Hashing has no
invalid input; decoding does, and the two families should not share a convention just because they
share a parameter type.

**Cost, recorded so it is not rediscovered:** Increment K cannot carry arbitrary socket bytes through
this name. Nothing regresses — all 23 `شبكة` names already fail today.

**Generalisable, and it is a second kind of §1.3 defect.** The known one is an expiring criterion (a):
a claim about what the language *cannot express*, which each landed increment can falsify. This is a
different failure — a **contract** that no backend pair can jointly honour. Check the contract against
the value representation, not only the criterion against the language.

### Criterion (a) expired too, making three

Re-derived rather than read off the row, as the standing rule says. Indexing over `مصفوفة<عدد>`
(#330), the seven bitwise names, and `رمز_إلى_حرف` (#326) together make UTF-8 decoding writable in
Tarqeem, so the claim had expired before the name shipped — after `بتات_نفي` and
`بتات_إزاحة_يمين_منطقية`. `test_bytes_to_string_matches_the_decoder_it_names` runs a hand-written
Tarqeem decoder beside the builtin across all three backends and asserts agreement, which is what the
rule asks for instead of repeating a stale claim.

It shipped as a primitive anyway: core tier, and §5.2 keeps a no-import name a builtin until **B12**.
One ground the earlier expiries lacked — the *validating* half stays materially harder to hand-write
(overlong forms, surrogates, truncated tails), and that is what the primitive actually buys.

### A missing return type is louder for a string than for an array

#330 measured that dropping the `register_builtin_return_types` entry for an **array** return was
nearly silent — four of five plausible assertions passed and only `نوع` caught it. Measured here by
deleting the entry for a **string** return: `اطبع` still printed «مرحبا», «hi» and «م» correctly, but
`نوع` answered `مؤشر`, `"X" + …` printed `X4340804192`, and `== "﷽"` answered `خطأ`.

Three of five caught it rather than one, because concatenation and comparison degrade visibly on a
string where indexing and printing did not on an array. The lesson is unchanged in practice — assert
`نوع`, concatenation and equality, never printing alone — but the two shapes fail at different
volumes, and the array is the quieter one.

### `مصفوفة<عدد>؟` does not parse, so the null route is different

`LANGUAGE_SPEC` §5.3 admits `نمط_اختياري := نمط '?'`, but `متغير غائب: مصفوفة<عدد>? = لا_شيء` is
`ب٠١٠١` at the `?`, and a bare `لا_شيء` is refused at the argument. So the route that made a
`Value::Null` arm load-bearing for `نص_إلى_ثنائي` — an un-narrowed `نص?` admitted by `Type::compat` —
does not exist for this name.

The arm is still required, reached through an **`أي` holder** instead: `متغير غائب: أي = لا_شيء` then
`ثنائي_إلى_نص(غائب)` answers `""` natively, and without the arm both interpreters would raise a type
error on source native runs fine. This refines #326's narrowing a third time. The rule is now: ask
whether the parameter is a pointer, **and** how a null can be written for that type at all. Three
consecutive names, three different answers.

### Two things worth keeping about method

**The array-literal-as-argument shape was probed before any fixture depended on it.** #304 (an
intercepted builtin inside an array literal segfaults natively) and #327 (the call-argument path) both
live next door, and no sibling had ever passed a literal *into* a builtin — the three before this one
only produced arrays *from* one. `ثنائي_إلى_ست_عشري([104، 105])` is correct in all three backends, so
the fixtures stand. One file, three commands, and it would otherwise have been an assumption under a
whole test section.

**One decode definition, shared, rather than a duplicated arm.** `bytes_to_string` is `pub(crate)` in
`interpreter::executor::builtins` and re-exported for the debug interpreter, following what
`Value::to_display_string` already does there. The debug interpreter is deliberately a duplicate of
the *dispatch*, but duplicating the rejection **logic** would let the two drift on which arrays are
refused. Duplicate dispatch; share rules.

Related: it must not reuse `value_to_byte` from the same file, which *errors* out of range — that
would raise a runtime error where native answers `""`, manufacturing the exact divergence this
increment exists to avoid. Two byte-reading helpers in one file with different failure modes is a
sharp edge; the new one carries a comment saying why it is not the other.

## 2026-08-17 — One runtime search, ordered so a dev build wins (#285)

### The bug was the install instructions, not just the duplication

Two `find_runtime`s had diverged, and `compile` called the weaker. But the reason
that mattered on *every* developer machine is that `install.sh` and the `Makefile`
both tell the user to export `TARQEEM_HOME` — and the CLI copy ranked it first.
So the documented way to install Tarqeem was also the way to permanently stop
linking your own `cargo build` output. Confirmed locally: the selected archive was
ten days older than the built one and lacked `trq_string_to_int_checked`.

### Resolving against the executable is what makes one order serve both cases

The fix is not "demote `TARQEEM_HOME`" — for an installed user it is the right
answer. It is that **neither copy ever probed `<exe_dir>/libtrq.a`**, though
`target/release/libtrq.a` sits directly beside `target/release/tarqeem`. Ranking
exe-relative paths high resolves dev and installed layouts with no environment
variable at all: the dev binary finds its sibling, `~/.tarqeem/bin/tarqeem` finds
`../lib`. `TARQEEM_RUNTIME_PATH` stays above everything as the explicit override.

The merge is a union — no location either copy searched was dropped, since CI's
compiled-examples job depends on the CWD `runtime/` entry and someone may rely on
beside-the-executable.

### Three defects found underneath the reported one

- **`build.rs`'s `TARQEEM_RUNTIME_PATH` never arrived.** It is emitted as
  `cargo:rustc-env`, which feeds `option_env!`; the linker read it with runtime
  `std::env::var`. That priority had never once fired outside CI. Now read via
  `option_env!` as intended.
- **The target-dir probe tried `release` before `debug` unconditionally**, so a
  debug compiler linked the release runtime whenever both existed. Now keyed to
  `cfg!(debug_assertions)`, matching the suites' own `test_profile()`.
- **`compile_to_wasm` fell back to the *native* finder**, so a missing
  `libtrq_wasm.a` made it link an aarch64 archive into a wasm build. Nothing in
  the workspace builds `libtrq_wasm.a` at all, which is why ت٠١٠٢ is scoped to
  the native path — hard-erroring on wasm would break every `--emit-wasm`.

### Purity is what made the precedence testable

`find_runtime` read process-global env and the filesystem in one pass, which is
why its two tests mutated `TARQEEM_RUNTIME_PATH` and were flaky under the full
suite (recorded below, now resolved), and why one of them asserted nothing at
all. Splitting `runtime_candidates(&RuntimeEnv, &RuntimeLayout) -> Vec<PathBuf>`
out as a pure function turns "a dev build outranks every installed location" into
a plain assertion over a vector — no `set_var`, no temp dirs, parallel-safe. The
same vector is what ت٠١٠٢ lists, so message and behaviour cannot drift apart.

### The tests were passing against the wrong runtime

`module`/`exception`/`property`/`inheritance` build the archive into
`target/<profile>/` and assume the compiler looks there. It did — but only via
the linker fallback, which is reached only when the CLI search returns `None`. On
any machine with `~/.tarqeem/lib`, the CLI found that first, so those four native
legs were locally linking a stale runtime while still going green. CI never saw
it because runners have no `~/.tarqeem`. That is the silent-wrong-output failure
mode this repo keeps hitting, hiding inside the test suite meant to catch it.

`builtins_execution_tests` was the one suite that had noticed, and worked around
it by staging a copy into a `TARQEEM_HOME`-shaped directory. That staging is now
deleted; the build-on-demand half stays.

## 2026-08-15 — Native virtual dispatch, via the prefix invariant (#280)

### Why a real vtable, not the guard the issue proposed

#280 offered an interim: refuse the shape natively rather than dispatch it. The
condition on file — definer ≠ the receiver's static class **and** some descendant
overrides — is precise for #280 and fires on nothing in `examples/أصناف.ترقيم`,
so it was tempting. It is also, measured, only half the defect. The four upcast
tests in `tests/oop_execution_tests.rs` are all *definer == static class*
(`شكل` declares `مساحة`, `مربع` overrides), so the conjunction never fires on
them, and native printed `0` where the interpreter printed `25`. That half
predates #253 and was #184's recorded cost, not this regression — but it made
the guard a fix for one shape of a two-shape bug. Dispatch fixes both.

### The invariant that makes it cheap

A subclass's vtable is its parent's **prefix**, overrides replacing entries in
place and new members appended. So the slot index a class assigns a member is the
index every descendant assigns it too, and codegen can read the index off the
receiver's *static* class while the object's own table supplies the *runtime*
implementation. Dispatch never needs to know the runtime type — which is exactly
as well, because natively it cannot: objects carried no type word at all before
this, and `trq_alloc`'s header is refcount+size.

That is why `class_own_virtuals` records members in **declaration order**.
Deriving the order from `method_return_types` — a `HashMap` — would renumber slots
per run and silently break the invariant, on a table nothing prints.

### Two defects that were invisible while the vtable stayed empty

- `emit_vtable` spelled entries `@{class}_{method}` while bodies are emitted as
  `mangle_function_name("{Class}::{method}")`. Any populated vtable would have
  referenced symbols nothing defines.
- `CallVirtual`'s lowering already loaded word 0 as a vtable pointer — a slot that
  did not exist, so it would have read the first field. Both were dead code with
  no test able to reach them. `Class.vtable` being empty is what kept them quiet.

The wiring is on `CallMethod`, not `CallVirtual`: the builder already sets
`virtual_dispatch` correctly at all three emit sites and codegen was discarding
it, so honouring the existing flag beat introducing an instruction nothing
constructs. `CallVirtual` stays unemitted.

### Scope boundaries held deliberately

`__anonymous__` object literals get **no** vtable slot: they resolve fields by
name, are never method receivers, and share `NewObject`. A class with no virtual
members emits no vtable global, so `NewObject` skips the store and word 0 stays
`trq_alloc`'s zero — nothing can load it. Interface-typed receivers still bind
statically (no `ir::Class` entry exists for a `ميثاق`), which is #209, untouched.
`الأصل.م()` keeps `virtual_dispatch: false`, and a fixture now pins that: an
override whose body super-calls would otherwise resolve back into itself.

### Verification notes

The `+1` field shift lives only in `get_field_access_info`. A sweep attributing
every `getelementptr` in `codegen.rs` to its instruction arm confirmed only
`GetField`/`SetField` index class objects; `GetElementPtr` (arrays),
`GetVariantField` (enums) and the string-constant GEPs are unaffected — worth the
check, because #249 was precisely a missed indexer. `ir/opt/inline.rs` was also
checked: it matches `Instruction::Call` only, so it cannot devirtualize a
`CallMethod` back to a static bind.

`examples/أصناف.ترقيم` gains an upcast call through a `شخص`-typed parameter, which
puts the shape under CI's `compare-backends` permanently. Note what that example
was before: the *reverted coarse guard's* false positive. The shape it was cited
for protecting now works rather than being refused.

## 2026-08-15 — Inherited method calls name the definer (#253)

### One lookup, because two lookups can disagree

`MethodId.class` and the return type were derived separately from the same wrong
key, `{receiver}::{method}`. Both missed for an inherited method, and the two
misses failed differently: the class produced an undefined symbol at link, while
the return type degraded quietly to `Ptr(Void)`, lowering an `عدد` method into a
`trq_print(ptr)` on an integer. The fix takes both from one
`resolve_instance_method` call, so a future edit cannot repair one and leave the
other. That is why the resolver returns a tuple rather than just the class.

Same defect and same shape as #249/#250, one branch over: those fixed member
*access* in `build_member`/`store_to_member`, this fixes the method-call branch
of `build_call`. The static-method branch twenty lines above already resolved up
the chain, so the file contained its own counter-example the whole time.

### The miss path stays lenient — deliberately

#249 added a strict `unknown_member_error` gate for fields. That was not copied
here. This branch also carries `أي`-typed receivers, `ClassId("")` receivers,
`__anonymous__` object literals, and interface-typed receivers — `InterfaceDecl`
is a no-op in the builder, so a `ميثاق` type never enters `class_fields` and
would look identical to a typo. A hard diagnostic would have traded a fixed bug
for a new class of false positives.

### Codegen and the interpreter were left alone, on purpose

Native always binds statically on `MethodId.class`; the IR `Class.vtable` is
initialised empty and nothing ever pushes to it, so `emit_vtable` never fires and
`CallVirtual` is never emitted. Naming the definer fixes the link error without
reopening that. The interpreter is unaffected because `resolve_virtual_method`
walks from the object's *runtime* class, not from `MethodId.class` — it only ever
consulted that field in its non-virtual fallback.

### `الأصل.م()` to a grandparent was fixed for free

A fixture written as discovery — a super call to a method declared two levels up
— failed on **all three** backends before the fix, minting the *immediate
parent's* class rather than the definer's (`infer_expr_type`'s `Super` arm yields
the immediate parent). Unlike a normal member call, a super call dispatches
non-virtually, so the interpreter could not rescue it either. It flows through
the same branch, so the chain walk carried it with no second change. Worth
remembering: super calls are the one path where an id defect is *not* native-only.

### Why nothing caught this

`tests/oop_execution_tests.rs` covers inherited dispatch thoroughly and is
interpreter+JIT only, and `examples/أصناف.ترقيم` had the one call that would have
exercised the shape commented out with a note citing this issue. A test suite and
a corpus can both be green while jointly excluding the same case. The example is
re-armed here, which puts the shape under CI's `compare-backends` job.

### Discoveries

- **#277** — `cargo fmt --check` failed on `develop` itself
  (`src/semantic/linker.rs:115`, from 9af9382): a closing paren left on the
  argument line. The `lint` job gates all of CI, so this failed every branch cut
  from `develop`, whatever it changed. Fixed here rather than deferred, because
  no branch — including this one — can go green until it lands.
- **#278** — a subclass does not inherit its parent's `منشئ`; `check` rejects it
  with `الصنف 'فرع' ليس له منشئ` even though the parent has one. Distinct from
  #211, which is the no-constructor-anywhere case.
- #211 gained a note that it also fails at link natively, not only at runtime.
- #222 and #241 gained corrections: both undercount, and #241's own diff command
  uses `@trq_[a-z_]*`, whose character class stops at the first digit — it
  invented `trq_base`/`trq_sha` while hiding `trq_base64_encode`/`trq_base64_decode`.
  A 2-for-2 swap, which is why the wrong total still looked self-consistent.

### Review follow-ups

- `infer_expr_type`'s `Call { callee: Member }` arm answered `Ptr(Void)` for
  *every* instance-method call, so a member read off a call result — `ك.احصل().س`
  — resolved its field against no class and lowered as `load ptr` on an `عدد`
  slot, which native then handed to `trq_print(ptr)` and dereferenced. Segfault
  natively, correct under the interpreter and the JIT. Fixed with this entry's
  own `resolve_instance_method`, so both paths that compute a call's return type
  now agree.
- Naming the definer trades a loud failure for a quiet one in the upcast shape:
  receiver statically typed as a class that declares nothing, runtime class
  overriding. Native used to reject the undefined `@{static}::{method}` at link;
  it now binds the ancestor's body and prints the wrong answer while the
  interpreter and the JIT print the override's. Bisected against 3c3353b to
  confirm the failure *mode* changed rather than the correctness, and filed as
  **#280** — the only known case where a fix here introduced a silent
  divergence, so it gets its own issue rather than a line inside #185.

  Recorded there and worth repeating: the coarse guard the codegen comment says
  was tried and reverted lacked one condition. Rejecting only when the definer
  **differs from the receiver's static class** *and* some descendant overrides
  would not have fired on `examples/أصناف.ترقيم`, where the definer *is* the
  static class. Untried, and cheaper than vtable dispatch if #280 needs an
  interim.

## 2026-08-13 — `blocks.last()` is never the current block (#234)

### The asymmetry

`emit()` always writes into `self.current_block`. Three body-closing checks read
`func.blocks.last()` instead — the class method and constructor branches of
`build_class_decl`, and the block-bodied lambda. The check inspected one block,
the write targeted another.

`build_match` mints `match.exit` **before** the arm blocks, so after a `تطابق`
the merge block is buried mid-vector and `blocks.last()` names an
already-terminated arm. The check saw a terminator, skipped the implicit
`Return`, and the real merge block went out bare. The interpreter treats an
unterminated block as fall-through *in vector order*, so `match.exit` fell into
`match.arm0`, whose join jump goes back to `match.exit`: an infinite loop, and
the caller never regained control. Native landed on codegen's `unreachable`.

The guard and the explanation both already existed — `mod.rs`'s script-mode
`__main__` close carries the comment verbatim, and `current_block_needs_terminator`
was added for #181 — they were simply never propagated to the class-member and
lambda paths. All three predate the module split; none was touched by #232.

### Why the loop fixture stayed green

`CLASS_WITH_BRANCHING_METHOD` ends in a straight-line `طالما`. Loops and `حاول`
also create their exit block before the body, but with a *flat* body nothing is
pushed after it, so `blocks.last()` accidentally **is** the exit block. The
existing terminator test therefore could not have caught this. Any nested
block-creating statement inside such a body breaks the coincidence — the bug was
never `تطابق`-specific, only `تطابق`-guaranteed.

### The non-void trap

Swapping in `current_block_needs_terminator()` alone regresses non-void methods.
When every `تطابق` arm returns, the merge block is dead-but-emitted; today it
stays unterminated and codegen writes a valid `unreachable`. A bare
`Return { value: None }` there is `ret void` inside an `i64` function, which LLVM
rejects. Gating on void instead (as `build_func_decl` does) is worse — it leaves
the interpreter hang alive for any non-void method with a fall-through arm.

The lambda path already had the answer: void → `Return { value: None }`,
non-void → typed-zero `Const` + `Return { value: Some(_) }`. That body moved to
`IrBuilder::emit_implicit_return`, now shared by all three sites. `build_func_decl`
keeps its own void gate; its non-void case is the same latent hazard and wants
its own change.

Tests assert IR shape rather than execution wherever possible: a regression here
**hangs** instead of failing, so `all(|b| b.has_terminator())` is what fails fast.
"All blocks terminated" alone does not catch the `ret void` regression — the
non-void fixture asserts the terminator carries a value.

## 2026-08-12 — A bool crossing into Rust was never zero-extended

### Found by adding four lines to an example

The #266 branch added `اطبع(ليس نشط)` and friends to `examples/أساسيات.ترقيم` —
no example had ever used `ليس`/`!` as an *operator*. `compare-backends` went red
immediately: native printed `صحيح`, then dumped `DW_OP_gt`, `ELR_mode`,
`deadlock`, `capacity`, `01234567` — DWARF strings and `.rodata` out of the
binary's own image — where `خطأ` belonged. 703 bytes became 2227.

Only on x86-64. A full macOS aarch64 run of both backends was byte-identical, so
the local check that "proved" the example was fine proved nothing. Two lessons
worth more than the fix: **a slice is not a diff** (the first check compared a
`sed`-filtered range, not the whole stream), and **byte-identical on one
architecture says nothing about another**.

### The mechanism

An `i1`'s upper byte bits are don't-care to LLVM, so `xor i1 %v, true` legalizes
to a byte-wide flip:

    movb   (%rax), %al      ; 0 or 1
    xorb   $-1, %al         ; 0xFF or 0xFE
    movzbl %al, %edi        ; 255 or 254
    callq  trq_print_bool@PLT

Rust's `extern "C" fn(value: bool)` is `zeroext noundef range(i8 0, 2)`. 254 is
not a `bool`, and the callee's branch arithmetic indexes outside either string
literal — hence a pointer into `.rodata` and a length to match. `true` survived
as 255 because it is merely nonzero, which is why the *first* negation printed
correctly and the second dumped memory: an accident, not a partial success.

This is also why printing a bool **variable** always worked: `load i1` compiles to
`movzbl` of a real 0/1 byte. Only a *computed* bool was affected, and `setne`
(from `!=`) happens to produce a clean 0/1 too — so of the four new lines, only
the two `ليس`/`!` ones broke. `zeroext` was absent from the entire codegen: the
grep count was zero.

### Where the fix belongs

On `map_param_type` (`src/codegen/llvm/types.rs`), not on the two call sites that
showed the symptom. That one mapper spells both `define` parameter lists and
generic call arguments, so fixing it there is what keeps a signature and its call
sites from disagreeing — and it covers `منطقي_لنص`, which reaches
`@trq_bool_to_string` through the generic path rather than the hand-written
string. The two hardcoded runtime calls were fixed as well since they bypass the
mapper. Attributes are legal in every one of the five parameter positions the
mapper feeds; none reconstructs a bare function *type*, where `zeroext` would be
invalid IR.

Returned `i1` needs nothing: Rust guarantees 0/1 outbound, and our own callees
read only bit 0.

### The guard cannot be an execution test

Nothing local executes x86-64, so the regression test asserts the attribute in
the emitted IR — on the **call site**, because that is where LLVM takes the ABI
from; a declaration carrying it alone would not fix the call. Verified red on the
pre-fix codegen and green after, via `git stash` of the two source files.

### CI was telling us as little as it could

`diff -u` collapses to "Binary files differ" the moment either stream contains a
NUL, so the only evidence in the log was that *something* differed somewhere. The
step now prints `cmp -l` and `od -c` for both streams and keeps `compare/` as an
artifact on failure — which is how the DWARF strings were identified, and how the
first byte offset (379) pointed straight at the second negation.

## 2026-08-12 — Issue #266: the Pratt loop's non-advancing fallback, fixed

### The fix is one deleted line; the reasoning is which line

`Precedence::of` scored `TokenKind::Bang` as `Precedence::Unary`, an **infix**
binding power for a token that has no `parse_infix` arm. Two directions were open —
score it `None`, or give it an arm — and scoring it `None` is the one that removes
the anomaly instead of documenting it: `Bang` was the *only* one of the 27
precedence-bearing token kinds without an advancing arm, so the table now holds a
real invariant ("everything scored here is consumed by `parse_infix`") rather than
one exception plus a comment.

Nothing derives prefix binding from `of()`. `parse_prefix`'s `Bang` arm passes the
`Precedence::Unary` **literal** (`expr_parser.rs:207`), which is why `ليس صحيح`,
`!خطأ`, `٥ != ٣` and `ليس أ و ب` are untouched — verified byte-identical between
`run` and `compile`.

`parse_infix`'s catch-all changed from `_ => Ok(left)` to an `ERR_UNEXPECTED_TOKEN`
error as well. That arm is unreachable once the table is consistent, and that is
the point: it was reachable *silently*, and the next token to gain a precedence
without an arm would have hung exactly the same way. An error there costs nothing
and converts the whole class from hang to diagnostic. No new error code was needed,
so the `docs/رموز_الأخطاء/` SOP did not apply.

### The blast radius was three times what the issue recorded

The issue named `parse`, `check`, `run` and the LSP. Measured with a `SIGALRM`
watchdog, **eight of the nine CLI commands hung** — add `fmt`, `doc`, `compile`,
`repl` and `debug`. Only `lex` survived, because it never constructs a `Parser`.
There is exactly one Pratt loop in the repo, so every mode failed through one call
site. Post-fix all eight exit 1 in ~0s with `ب٠١٠١ متوقع '؛'` — the same
diagnostic `س في`, `س كـ`, `س ->` and `س من` already produced, which is the
consistency the `None` score buys. Inside a call it is `ب٠٠٠٢ متوقع ')'`.

Also corrected, since the issue's file map implies otherwise: `src/cli/commands/`
has no `parse.rs`/`check.rs`/`run.rs`/`fmt.rs` — those are functions in `mod.rs`.
And `ليس` is not a separate keyword token: `keywords.rs:76` maps it to
`TokenKind::Bang`, so `ليس` and `!` were never two defects.

### The LSP failure mode is a wedge, not a slow request

Worth recording because it outlives this fix. tower-lsp 0.20 `join!`s its
read/process/write halves onto one task, so a handler future that never returns
from poll stops the server reading stdin **at all** — no hover, no diagnostics for
any file, not even `shutdown`. `src/lsp/analysis/document.rs:144` parses
synchronously with no debounce, cancellation, timeout or `catch_unwind`, and 12 of
the 13 handlers reach it (`textDocument/formatting` is line-based and immune). This
change removes the only known trigger, not the fragility; filed separately.

### Testing a hang without hanging the suite

The repo had no timeout pattern at all — no `mpsc`, `recv_timeout` or
`catch_unwind` anywhere, and the CLI harnesses' `Command::output()` blocks forever.
So the guard is std-only, needing no new dependency: one worker thread **per
input** plus `recv_timeout(10s)`. Per-input matters — a single thread running all
seven cases times out without naming which one regressed, and naming it is the
whole point. Verified red before the fix (failed in 10s naming `س ليس`) and green
after (0.00s).

The behavioural test asserts `س ليس` and `س في` produce the *same* code rather
than hardcoding `ب٠١٠١`, so the two cannot drift apart.

No `.ترقيم` fixture was committed. `examples/` and `stdlib/` are each walked by
four unbounded guards, so a hanging fixture on disk turns all four into hangs —
which is precisely how this bug was found (#265).

### CI could not have told us

No workflow sets `timeout-minutes`, and the two *corpus* steps were the only loops
in `examples.yml` not wrapped in `timeout` — the run steps all were, because
runtime hangs were anticipated and parse hangs were not. A single bad corpus file
would have hung six jobs (`check-examples`, `check-stdlib`, `test-modules (fmt)`,
`test-modules (parser)`, `integration-tests`, `test-full`) for six hours each, with
`fail-fast: false` preventing early cancellation. Both steps now use `timeout 30s`;
both jobs are `ubuntu-latest`, so GNU `timeout` is present — it is *not* on macOS,
which is why the local repro needed a `perl` `alarm` wrapper.

### Noticed, not fixed

`tarqeem fmt` prints a raw `Diagnostic { … }` debug dump on a parse error instead
of the rendered bilingual diagnostic every other command emits. Pre-existing and
unrelated to this change.

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
evidence is removed. Verified the divergence is exactly four lines: the two
`طول` prints over the Arabic text (448 vs 784) and the `نسبة` line computed from
them (17% vs 9%). `طول` over the compressed bytes agrees, and so do the digests
and the gzip round-trip.

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

### The house style for examples, and why it is not cosmetic

Consolidating exposed that the corpus had no single style: semicolons in some
files and not others (the spec makes them optional, and README/LANGUAGE_SPEC
snippets omit them), comment-banner widths of 35/39/43/59/63, Arabic-Indic digits
in one half of a merged file and Latin in the other, and `//` headers where the
rest used `///`. A learner reading two files in a row has no way to tell which
differences are meaningful, so every one of them teaches something false.

The convention all ten now follow:

- `بسم_الله`, blank line, then a `///` file doc: one-line summary, blank `///`,
  a two-or-three-line description, blank `///`, `@منذ`.
- Section banners are `// ` + exactly 43 `═`. Files short enough to have no
  sections (`مرحبا`, `حاسبة`) have none.
- No ASCII `;`. The Arabic `؛` stays — it is the `لكل` separator, not a
  terminator.
- Latin digits throughout. `صياغة` keeps one short labelled block proving
  `٤٢ == 42`, so the corpus still witnesses Arabic-Indic literals somewhere.
- `///` on declarations, `//` for inline explanation. Function docs are
  imperative per `.claude/rules/arabic-philosophy.md` ("أنشئ شبكة" not
  "دالة لإنشاء شبكة"); type and enum docs stay noun phrases.

**The file doc needs the first declaration to carry its own `///`.** Otherwise it
attaches to that declaration and `tarqeem doc` emits a module page with no
description — which is what five of the ten did at first, `مرحبا` included. It is
not enough for *some* later declaration to be documented. `مرحبا` additionally
had its first item be an executable statement, so its `دالة تحية` was moved above
the print statements (output order unchanged) to give the file doc something
documented to sit in front of. Verified by generating markdown for all ten.

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

> **Resolved 2026-08-17 (#285).** Both tests are gone. Discovery now splits into
> a pure `runtime_candidates` over a gathered `RuntimeEnv`, so precedence is
> asserted without touching process env at all.

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

## Rules hardening (2026-08-13)

Four rule files stated things that were not true: `error-codes.md` linked a
path that resolves inside `.claude/`, `testing.md` prescribed English-keyword
tests and a `tests/integration/` tree that never existed, and
`bug-tracking.md` listed six labels none of which had been created — so every
`gh issue create` using them failed silently. Rules that lie are worse than no
rules; they get followed.

Decisions worth keeping:
- **`code-quality`** is the label for comment bloat and readability debt.
  Spotted bloat gets *filed*, not fixed inline — a drive-by comment refactor
  inflates an unrelated diff, which is the problem it would claim to solve.
- **Beta Roadmap board** gates work selection. `Planned` empty ⇒ shortlist
  five related issues and stop for the user, rather than picking one and
  starting. Board IDs are discovered at runtime so the rule survives the board
  being replaced (Beta → Alpha).
- **Mermaid**: vendored `WH-2099/mermaid-skill` (MIT, dependency-free) at
  project level. Diagrams are capped at 12 nodes and validated before
  shipping. Mixed Arabic/Latin labels get reordered by the bidi algorithm
  (`ت٠٣٠٣` renders reversed) — keep each label in one script.
- `comments.md` / `rust-style.md` globs were `src/**/*.rs`, so neither applied
  to `runtime-rs/` or `benches/`. Widened.

## #222 — core builtins that disagreed between backends

Six core builtins type-checked and ran interpreted but failed to compile
natively; `طول_مصفوفة` and `الحق` failed the other way. One function explains
all eight: `mangle_function_name` (`codegen.rs`) is a name-only lookup applied
symmetrically to call sites *and* definitions. A miss falls through to mangling
the Arabic identifier, producing a call to a symbol nothing declares — which is
an LLVM IR **parse** error, not a link error, since an undeclared symbol never
reaches `ld`. The same symmetry is #257, in the opposite direction.

Decisions worth keeping:

- **Builtins that need argument types are lowered in the IR builder, not the
  codegen table.** The table is name-only: it can express neither type dispatch
  (`عدد` is declared over `أي`) nor arity (`تأكد` has one parameter, `trq_assert`
  takes two). The builder is the only layer holding `var_types` and the only one
  that can synthesise the missing operand. `اطبع`/`طول`/`نص` were already there,
  which is exactly why they were the three that worked.
- **`نوع` folds to a constant.** `IrType` has no dynamic variant, so the answer
  is known at build time; no runtime type tag is needed, and none exists.
- **Builtin wins over a same-named user function — pinned, not chosen.**
  **REVERSED — see "Built-ins are the last tier" below. Kept for the reasoning,
  which still explains why a one-layer change fails.** A shadowing guard was
  written first, to protect `stdlib/اختبار/توكيدات.ترقيم`'s own `تأكد`. It
  *created* a divergence: native bound the user's function while the interpreter
  still ran the builtin, because `is_builtin` precedes `call_function` in the
  executor. The semantic layer also rejects a top-level redefinition outright,
  so shadowing does not work today in any case. The interception is now
  unconditional and a cross-backend test pins it. Whether builtins *should* be
  shadowable is #262.
- **`عدد("أبجد")` must fail, not yield 0.** `trq_string_to_int` returns 0 via
  `unwrap_or` — correct for the stdlib's lenient parsers, wrong here, and it
  would have replaced a loud link error with a silent wrong answer. Hence the
  `_checked` variants. A conversion fix that only tests valid input tests the
  wrong half.
- **`BoolToInt` exists because `Bitcast` is pointer-only** — codegen emits
  `bitcast ptr … to i64`, and the interpreter ignores `to_ty` entirely, so it
  silently returned the bool unchanged. Widening needed a real instruction
  beside `IntToFloat`/`FloatToInt`.

Two traps for the next person touching this:

- **There are two `find_runtime` functions.** `codegen::linker::find_runtime`
  honours `TARQEEM_RUNTIME_PATH` and `target/<profile>/`; the one `compile`
  actually calls (`cli/commands/mod.rs`) honours neither, so a freshly built
  `target/release/libtrq.a` is never found and native links silently bind a
  stale `~/.tarqeem/lib/libtrq.a`. A new runtime symbol then reads as
  `Undefined symbols: _trq_…` no matter how often you rebuild. Tests must stage
  the archive where the CLI looks; `builtins_execution_tests.rs` does.

  > **Resolved 2026-08-17 (#285).** There is now one `find_runtime`, in
  > `codegen::linker`, and it prefers the archive beside the compiler over any
  > installed copy. The staging this recommended has been deleted from
  > `builtins_execution_tests.rs`; building the runtime is enough.
  > `tarqeem compile -v` names the archive chosen.
- **`nm -g` reads the release archive as empty** — `lto = true` leaves bitcode
  members. Check symbols against a debug build of the runtime.

The guard test is execution-based on purpose: these lowerings live in the IR
builder and are invisible to any static check of `get_runtime_function_name`.
`register_core_builtins` now iterates a list so the set can be enumerated at
all; three names (`ادخل`, `ادخل_رسالة`, `اطبع_خطأ`) stay uncovered because they
block on stdin or write to stderr, and the test says so rather than implying
coverage it does not have.

### Postscript: what the review of #222 caught

The first pass traded a loud link error for several quiet wrong answers, and
the new test suite did not catch any of them. Worth remembering *why*:

- **Every probe used a well-behaved argument.** The lowerings dispatch on type,
  so the bugs all lived in the arms no probe reached — `عدد` on an array
  (segfault), `منطقي(لا_شيء)`, `نص` on a `نص`, `الحق` into a float array. A
  type-directed fix has to be tested on the types it does *not* expect,
  including the builder's `Ptr(Void)` "unknown".
- **A catch-all `_ =>` is the dangerous arm.** Both conversion builtins ended
  theirs by handing an arbitrary pointer to a parser that casts it to
  `TrqString`. Unmatched now means a build-time bilingual error, not a guess.
- **`assert_fails` could not distinguish a compile failure from a runtime one** —
  same exit code, same empty stdout. A lowering that regressed into an LLVM
  parse error would have kept the suite green, which is the failure mode the
  suite exists to prevent. It now demands the compile succeed first.
- **stdout-only comparison has a blind spot**: `اطبع_خطأ` writes to stdout
  interpreted and stderr natively (#286). No cross-backend check in the repo —
  including `compare-backends` — can see a defect whose only symptom is the
  stream chosen.
- **Four backends, not three.** The debug interpreter has its own builtin
  registry, so an IR lowering that emits a new `trq_*` symbol breaks
  `tarqeem debug` while `run`, `--jit` and `compile` all pass (#223).

## #185 — native divergences: طول, optionals, float display, null narrowing

The issue bundled six items against `main`. Two were already fixed by the time
the work started (`نوع` by #222, the stdlib-import segfault), and the remaining
four sat in three different layers. Re-running every repro before planning was
what established that; the issue text alone would have sent two agents after
bugs that no longer existed.

Four root causes, one shape: **codegen cannot see runtime types, and each fix
had to recover a type the layer already had.**

- **`طول`** is lowered with no type dispatch at all (`expr_builder.rs`, the arm
  computes `arg_ty` and ignores it), so it becomes `ArrayLen`. Every interpreting
  backend makes that instruction polymorphic; codegen emitted `trq_array_len`
  unconditionally. It was *silent* rather than a crash because `TrqString` and
  `TrqArray` are both `#[repr(C)]` with `len` first — the load succeeds and
  returns the byte count. The linker even folds the two functions to one address,
  so disassembly appears to show a `trq_string_len` call that codegen never made.
- **Comparisons** dispatched on `Binary.ty`, which is the *result* type and so
  always `Bool`. Integers worked by coincidence — that arm spells `i64`. The
  `(Eq/Ne, Ptr(_))` arms existed but were reachable only from `build_truthiness`,
  which passes the *operand* type as `ty`. Two booleans could not be compiled at
  all, which no issue had recorded.
- **`trq_print_float`** wrote whole floats as `value as i64` under a "%g style"
  comment. No other backend followed that convention.
- **Narrowing** was simply absent: `analyze_if` type-checked the condition and
  then analysed both branches knowing nothing about what it proved.

Decisions worth keeping:

- **Every backend fix stayed in codegen, deliberately.** The alternative for
  `طول` was to dispatch in the IR builder, which emits a new symbol and so must
  be taught to *both* interpreter registries or `tarqeem debug` breaks (#223).
  One file beats three, and `trq_string_len_chars` was already declared in the
  prelude, so nothing needed restaging past the `find_runtime` trap (#285).
- **`trq_float_to_string` was left alone on purpose.** Concatenation already
  agreed across backends on `5`; "fixing" it to match `اطبع` would have
  manufactured a divergence. The language is left internally inconsistent —
  `اطبع(5.0)` is `5.0`, `اطبع("" + 5.0)` is `5` — and a test now pins that, so
  the inconsistency is a decision rather than an accident.
- **Boxing shipped with the icmp fix, not after it.** Optionals lower to a
  pointer and a scalar was stored raw into that slot: valid LLVM under opaque
  pointers, so nothing rejected it, and `عدد? = 0` compared equal to `لا_شيء`.
  Landing the comparison fix alone would have replaced a build error with a
  silent wrong answer — the exact trade #222's postscript warns about.
- **Narrowing is withdrawn when the branch assigns to the variable.** Knowing
  *where* an assignment invalidates the proof needs a flow pass the analyzer
  does not have. Narrowing nothing is sound; narrowing a variable the branch
  then sets to `لا_شيء` is not.
- **`قاموس` was split out rather than bundled.** The literal `{ … }` parses and
  type-checks as `Type::Map`, which is why `ش["اسم"]` fails at *runtime* rather
  than compile time — but below the semantic layer there is no dictionary at
  all: no `IrType::Map`, no `Value::Map`, no `trq_map_*`, and `قاموس<م،ق>` is
  erased to `Ptr(Void)`. That is a feature, not this bug.

Traps for whoever touches this next:

- **Boxing has seven coercion sites, not one.** Local store, global store, global
  *initializer*, call argument, return, `SetField`, and `CallMethod` argument.
  Each was found by a failing test after the previous one passed, and the last
  two only by deliberately going looking for sites the earlier passes had
  missed — they compiled cleanly and answered `فارغ` for a present `0`. Two
  details that cost time: a global initializer cannot allocate, so it defers to
  program start the way string globals already do; and a method's declared
  parameter 0 is the receiver, which is *not* in `CallMethod.args`, so argument
  `i` is declared at `i + 1`.
- **The unit tests passed while the examples still diverged.** `"…" + مخزون`
  inside a narrowed branch printed the box's address, because the runtime's
  scalar-to-string conversions take the scalar and nothing unboxed for them.
  Only the example corpus caught it. Cross-backend fixtures over single
  expressions are not a substitute for running a real program.
- **Script mode and function mode take different paths.** A `متغير` at top level
  is a *global*; the same declaration inside `دالة رئيسية()` is an alloca. Three
  optional tests passed by hand and failed in the suite for exactly that reason.
- **`نص?` is `Ptr(String)`, not `String`.** A narrowed optional string has to be
  recognised as a string operand or `طول` goes back to counting its bytes.
- **The JIT proves less than its column suggests.** Cranelift compiles none of
  these instruction shapes and delegates to the interpreter, so a `--jit` leg
  agrees without compiling anything (#215).

`KNOWN_DIVERGENT` in `examples.yml` is now empty: `تشفير_وضغط:native` was its
last entry, and all ten examples agree across all three backends.

Verified afterwards, since the plan called for it and no automated test reaches
it: `tarqeem debug` (the fourth backend, #223) gives `5` for `طول("مرحبا")`,
`5.0` for `اطبع(5.0)`, `موجود` for `عدد? = 0` against `لا_شيء`, and `6` for the
narrowed `س + 1`. All four backends agree.

## #262 + #257 — built-ins are the last tier of the lookup order

**This reverses the "builtin wins" decision recorded above.** It is an owner
decision about language semantics, not a bug fix, so it is written down here
rather than inferred from the diff — the earlier note is still in this file and
would otherwise read as current policy.

The rule, first match wins:

```
local → enclosing scopes → module-level → imported → built-ins
```

Naming a function after a built-in is legal. Built-ins are a fallback, not
reserved words.

- **Why the first attempt failed, and this one does not.** The reverted guard
  changed only the IR builder, so native called the user's function while the
  interpreter still ran the built-in. The lesson is *"change every backend
  against one predicate"*, not *"user-wins is wrong"*. That predicate is "does
  the program declare a function of this name?", answered from `ir::Module`
  (interpreter, debug interpreter, codegen) and from `IrBuilder::function_names`
  in the builder, which runs before `Module` exists. The two sets come from the
  same top-level `FuncDecl` pass.

- **The semantic half is the half that was missing.** Two populations failed
  differently: the 18 `core_builtins()` are `define`d into global scope, so
  `دالة اطبع(…)` was a hard `د٠١٠١` and no backend ever ran; the ~180
  import-registered names were accepted silently and only died in codegen. A
  backend-only change is a no-op for the names people actually want to shadow.

- **`Scope::builtin_names`, not a `Symbol` field.** `define` needed to tell a
  built-in it may displace from a user symbol it may not. A flag on `Symbol`
  would have forced `is_builtin: false` into ~20 `Symbol { … }` literals in the
  analyzer for no gain. It must *not* be inferred from `span == Span::default()`:
  `هذا` shares that span and would become overwritable.

- **One arm in `define` fixed six sites.** Variables, nested functions, classes,
  interfaces, enums and top-level functions all report through the same
  refuse-on-duplicate path, and every import `define` already discarded its
  `bool` — so imports began shadowing built-ins with no call-site edit. Call
  type-checking followed for free: `infer_call_expr` has no built-in branch, it
  types the callee from whatever `lookup` returns.

- **#257 dissolved rather than being fixed.** `mangle_function_name` applied the
  runtime table to *definitions* as well as call sites, so a user's `مطلق` was
  emitted as `define @trq_abs_float` beside the `declare` of the same name and
  LLVM rejected the IR while parsing — never reaching the linker, despite the
  error text. Once a declared name keeps its own symbol there is nothing to
  collide. The function had to take the declared-name set because it is free,
  and `Module` is a parameter of `generate` that is never stored on `self`.

- **#262 scenario (b) is not a defect.** `متغير عدد = ٧` then `عدد("٥")` failing
  is correct: the local legitimately makes the built-in unreachable. Only the
  wording is arguable.

- **What has no in-language answer yet.** A same-name delegating wrapper —
  `دالة مطلق(س) { أرجع مطلق(س) }` — is now unconditional recursion, with no way
  to reach the built-in. No such wrapper exists in the repo; the six that did
  were deleted for this reason (see "Renaming Latin stdlib identifiers is not
  mechanical"). The `مدمج.` escape hatch is deferred to its own issue, and it is
  not the one-liner it looks like: `resolve_import_ref` rewrites `ns.prop` to a
  bare identifier, which under this rule re-resolves to the user's function.

Verified in all four backends, including `tarqeem debug`, which no automated
test reaches: `دالة مطلق → ٩٩٩` prints `999` and `دالة طول(س: نص) → ٤٢` prints
`42` — the latter having been a compile error before. All 10 `examples/` agree
across interpreter, JIT and native.

### The visibility trap this fix walked into first

The first version of the shadowing guard asked *"is this name declared?"* of the
**linked** AST, and that is the wrong question. `Analyzer::analyze` runs on
**main's** AST and registers only main's declarations plus the names it
imported; `linked_ast` then merges every declaration of every imported module
into one flat namespace under its bare name. So the two layers disagreed about
what "declared" means, and the backends took the linker's answer.

The result was worse than the bug being fixed. A module exporting `شيء` *and*
declaring `اطبع` captured `اطبع` in any program that imported only `شيء`:
semantic type-checked the call against the built-in's `أي` signature, the
backends called the module's `(نص)` function, and an i64 landed in a
`TrqString*` slot — wrong output interpreted, **SIGSEGV** natively. Before the
shadowing work the same program failed *loudly* at compile (that was #257), so
this traded a loud failure for a silent one. `stdlib/طرفية` has exactly this
shape: it exports `اطبع`, `ادخل` and `ادخل_رسالة`, all core builtins.

The fix is `Analyzer::visible_names` → `IrBuilder::with_visible_names` →
`Module::shadowing_names`: the question is answered **once**, from semantic's
own scope, and the backends read the answer instead of recomputing it. That also
retires the altitude problem the first version had — one rule re-derived in four
places from three sets that do not agree (`IrBuilder::function_names` is
top-level `FuncDecl`s of the linked AST, `Module::functions` additionally holds
methods and lifted lambdas, and the semantic scope holds neither).

Codegen needs **both** sets, and conflating them is a bug in either direction:
definitions mangle against *all* user functions, so every one keeps a symbol of
its own and #257's collision cannot return; call sites mangle against
`shadowing_names`, so a call the semantic layer bound to a built-in still
reaches `trq_*` even when some merged module declares that name.

Two smaller things the same review caught: a *variable* holding a function value
(`ثابت طول = (س: نص) => ٤٢`) is tier 1/3 and must suppress the interception too
— it type-checks since this change, and was still lowering to `ArrayLen`; and
§٤.٩'s claim that a module-level name shadows an import is false for file
modules, where `linker::record_origin` rejects the pair with و٠١٠١.

Guard verified by breaking it: with `shadows_builtin` forced to `true` the new
`test_an_unimported_module_declaration_does_not_shadow_a_builtin` fails on the
interpreter leg, so it is not vacuous.

## #241 — symbols codegen declares that nothing defines

Third instance of one drift class, after #185 (native `طول`/`نوع` diverging) and
#222 (names missing from the mapping table). All three surfaced as
`undefined value '@trq_…'` at link: an internal symbol, no source location, no
Arabic diagnostic.

### The issue's own list was wrong, in both directions

Its diff used `@trq_[a-z_]*` — no digits — so `trq_base64_encode`/`_decode`
truncated to a phantom `trq_base` and `trq_sha256*` to a phantom `trq_sha`, and
it listed `trq_sleep`, which is defined. Redone with digits, the real count is 21
(19 after excluding `trq_throw`/`trq_get_exception`, which belong to #238).

### Six tables must agree, not two

Worth writing down, because the next drift will be between two of these:

| # | Table | Location |
|---|-------|----------|
| 1 | declared externs | `codegen.rs::emit_runtime_declarations` |
| 2 | Arabic name → symbol | `codegen.rs::get_runtime_function_name` |
| 3 | definitions | `runtime-rs/src/*.rs` |
| 4 | semantic registry | `scope.rs::core_builtins` + `get_stdlib_builtin` |
| 5 | interpreter dispatch | `interpreter/executor/builtins.rs` |
| 6 | debug interpreter dispatch | `debug/interpreter/builtins.rs` |

For the `وقت` family these held 19, 19, 1, 2, 2 and 0 entries respectively.

### The two live symbols were broken three ways at once

`وقت_الآن` and `وقت_أداء` are the only two of the 21 reachable by documented
syntax. Defining them in `runtime-rs` fixes just one of three faults:

1. Codegen declared `ptr @trq_time_now()` while table 4 typed it `عدد` and table
   5 returned `Value::Int` — now `i64`, agreeing with both.
2. Neither was in `register_builtin_return_types`, so the call result carried the
   `Ptr(Void)` sentinel. That is the `جذر` bug: define the symbol and the link
   error becomes a segfault instead. **Defining a runtime symbol without
   registering its IR return type is never a complete fix.**
3. Table 6 had neither, so `tarqeem debug` aborted on both.

### Why the guard is static, not an execution test

No `examples/*.ترقيم` calls any of the 21, so `compare-backends` never links
them — a symbol only reaches the linker if some program happens to call it.
`tests/runtime_symbols_tests.rs` diffs tables 1 and 3 as source text instead.
Codegen never assembles `@trq_` names dynamically (the one wildcard match is a
doc comment), so the scan has no blind spot. Verified by breaking it: renaming
`trq_time_now` makes it fail and name the symbol.

Its `KNOWN_UNDEFINED` allow-list carries the `examples.yml` rule — entries are
removed as issues close, never added to silence fresh drift — and two further
tests fail if an entry becomes defined, or stops being declared.

### Deliberately out of scope

The nine date constructors return a field-bearing object (`بيانات.سنة`) that has
no representation below the semantic layer (#287), so they cannot be written at
all. Filed as #298 with the full mapping. The 10 that could be built were
implemented but **not** registered in tables 4–6, which is why native now
succeeds where the interpreter errors on the phantom-import path — recorded in
#298 rather than silently absorbed.

`runtime-rs/src/date.rs` fixes semantics no interpreter can be diffed against, so
those choices are now the contract: weekday 0 = الأحد, ISO-8601 weeks, Arabic
`DDD`/`MMM`, `i64::MIN` for impossible dates, epoch milliseconds for both clocks.
The WASM/JS shim disagrees on the last one (`performance.now()` is page-load
relative), in the family of #288.

---

## Builtin / stdlib boundary — Phase 1+2 (research and plan only)

Branch `feature/builtins-stdlib-boundary`. Two documents, no code changes:
[`builtins-inventory.md`](builtins-inventory.md) (census) and
[`builtins-vs-stdlib.md`](builtins-vs-stdlib.md) (the boundary decision).

### Why the census had to come first

The "builtin count" was ambiguous until the two planes were separated. `runtime-rs`
exports 218 symbols, but 22 are ABI plumbing the compiler emits for ordinary
operators and allocation, and 28 have no caller at all. Counting those as builtins
doubles the apparent surface and makes any target size meaningless. The
language-visible surface is 183 declared names; the runtime symbols reachable from
one are 154.

### The finding that set the plan's shape

`استورد … من "رياضيات"` never touches disk — `stmt_analyzer.rs:1122-1128`
short-circuits 7 specifiers to the native table and `modules.rs:299` skips loading
them. But `مجموعات` genuinely loads from disk and type-checks, and the `استثناء`
prelude injects an AST-backed declaration visible with no import across all three
backends. So the loader and the no-import mechanism both already exist; this is a
migration between two live paths, not new infrastructure.

Empirically (probe, Phase 2): the *same* computation segfaults natively when routed
through the builtin table and returns the correct answer on all three backends when
its body is Tarqeem source in a disk module. Migration *fixes* #185 per name moved,
and closes the 78-name interpreter hole for free.

### Decisions recorded so they are not relitigated

- **`اطبع` cannot become a stdlib wrapper over an fd-write primitive**, which the
  task specification assumed. Three independent blockers: native codegen refuses any
  user function with an `أي` parameter (ت٠٣٠١); `نوع` folds at build time and cannot
  dispatch; generic *free functions* do not parse. The escapes are a value-representation
  change or a syntax change, both forbidden. `اطبع`, `اطبع_خطأ`, `نص`, `منطقي`, `عدد`,
  `عدد_عشري`, `نوع` stay compiler intrinsics. Corollary: no alias of an `أي`-signature
  builtin can be demoted to a wrapper — those are keep-or-remove, never wrap.
- **Category 1 and 5 of the target surface are legitimately zero.** Object construction
  is syntax; reference identity already works (`==` on objects compares identity, verified);
  maps are not a runtime type (`IrType` has no map variant) and are already self-hosted.
- **Bitwise arrives as functions, not operators**, because the refactor may not change
  syntax. The IR variants (`BitAnd`/`BitOr`/`BitXor`/`BitNot`/`Shl`/`Shr`) already exist
  with arms in the interpreter, debug interpreter, both JIT tiers, LLVM codegen and the
  const folder — only the frontend is missing, so seven primitives cost two files.
- **A stdlib function may never wrap a still-registered builtin of the same name.**
  It self-recurses; the native binary compiles clean and then segfaults with no
  diagnostic. Delete the registration in the same commit that defines the replacement.
- **Prelude tier ≠ builtin tier.** Builtins are shadowable (#262); prelude and module
  names collide fatally (ص٠٦٠٢ / و٠١٠١). Moving a no-import name to the prelude would
  turn documented working shadowing into a compile error, so no name moves until
  `linker.rs` learns to treat prelude-origin declarations as displaceable.

### Blockers found that are not part of this refactor

Generic type-parameter substitution is broken (`جديد قائمة<عدد>()` does not bind `ن`;
same failure for a locally declared generic and with an explicit annotation), and merely
*importing* a module containing a generic class breaks native codegen with an unsized
`%class.__anonymous__`. Together these mean `مجموعات` is importable but unusable, which
contradicts the assumption that it is in production use. Both need issues.

---

## #302 — بتات_و, the first bitwise primitive

Branch `feature/302-bitwise-and-builtin`. Increment A of
[`builtins-vs-stdlib.md`](builtins-vs-stdlib.md) §6.1, one name of seven.

### The cost estimate held

Two source files: a `Scope::core_builtins()` entry `(عدد، عدد) -> عدد`, and an arm in
`build_core_builtin_call` emitting `BinaryOp::BitAnd` over `IrType::Int`. No `runtime-rs`
work and no runtime symbol, because the IR variant already had arms in the interpreter, the
debug interpreter, both JIT tiers, LLVM (`and i64`), the constant folder and CSE.

### Two registration sites the plan's rule 5 does not require here

Rule 5 asks for a `register_builtin_return_types` entry and interpreter + debug-interpreter
arms. Neither applies to a builtin intercepted in the IR builder:

- The interception emits `Instruction::Binary`, never `Instruction::Call`, and `is_builtin`
  is only consulted on a `Call`. The existing `BitAnd` arms serve it.
- `function_return_types` is read to type a `Call` result. The arm inserts `IrType::Int`
  into `var_types` directly, so the destination is typed at the point it is created.

The rule is written for the symbol-mapped path; an IR-lowered primitive is the cheaper
shape, and the seven bitwise names are all of that shape.

### Why bitwise names are functions

Tarqeem has no `&` token, and the refactor may not change syntax. `بتات_` prefixes the
family because `و`/`أو` are keywords and so cannot *be* a name on their own (an identifier
may still begin with either — `وقت`, `أولوية`), and because `ثنائي`
already means "byte array" here (`بصمة_ثنائي`). A lexer test pins that `بتات_و` scans as one
identifier rather than `بتات_` followed by the logical-and keyword.

### What the example is for

`examples/مدمجات.ترقيم` is the home for builtin coverage in the CI backend-diff, and it
asserts *composition* — the result is added, compared, passed to a typed function, and run
through `نوع` — not just printed. A destination carrying the `Ptr(Void)` sentinel prints
plausibly and composes wrongly; printing alone would pass against that bug.

### Environment note, not a defect in this change

Four unrelated examples failed the local three-backend diff until `TARQEEM_HOME` pointed at a
freshly built `libtrq.a`. `find_runtime` never searches `target/<profile>/` (#285), so a
local `compile` links `~/.tarqeem/lib/libtrq.a` — blocker B14. An `ld` undefined-symbol error
means a stale archive; a clang IR-parse error would mean a real bug.

> **No longer applies as of #285.** The archive beside the compiler now outranks
> `TARQEEM_HOME`, so a local `compile` picks up `target/<profile>/libtrq.a`
> without the workaround. `-v` prints the archive actually chosen.

---

## #306 — بتات_أو, the second bitwise primitive

Branch `feature/306-bitwise-or-builtin`. Increment A of
[`builtins-vs-stdlib.md`](builtins-vs-stdlib.md) §6.1, two names of seven.

### The estimate held a second time

Same two source files as #302, and no third. `BinaryOp::BitOr` already had arms in the
interpreter (`executor/mod.rs:976`), the debug interpreter (`debug/interpreter/operations.rs`),
both JIT tiers (`bor`), LLVM (`or i64`), the constant folder and CSE — so the whole primitive
is a `Scope::core_builtins()` entry plus an IR-builder arm.

### One arm for the family, not one per name

The `بتات_و` arm became `"بتات_و" | "بتات_أو"` with the op chosen by name, following the
`"تأكد" | "تأكد_رسالة"` arm in the same function. The remaining four single-op names
(`بتات_أو_حصري`, and the two arithmetic shifts plus `بتات_نفي`, which is `Unary`) extend the
same shape; `بتات_إزاحة_يمين_منطقية` is the only one that composes rather than emitting one op.

### Why OR is a primitive and not a convenience

AND can only inspect bits; it cannot build a value. Replacing a packed field needs both —
`بتات_أو(بتات_و(ق، معكوس_القناع)، حقل)` — which is the operation every bit-packing routine in
the migration set performs. Until `بتات_نفي` lands, the inverse mask is written as a negative
literal (`-256` clears the low byte), and both the example and the execution tests use exactly
that form so it is covered rather than assumed.

### #304 reproduces here, and is not this change's

`ثابت م = [بتات_أو(12، 10)، …]` prints correctly interpreted and exits 139 natively —
identical to `بتات_و` and to `طول_مصفوفة`, which is why #304 is filed against the interception
path rather than any one name. Verified rather than assumed before saying so in §6.1. Nothing
in this change touches it; self-hosted stdlib built on these primitives must avoid an
intercepted call inside an array literal until it is fixed.

### Verification

Interpreter, JIT, native and the DAP debug interpreter all produce byte-identical output for
`examples/مدمجات.ترقيم`, and the regenerated `examples/متوقع/مدمجات.خرج` diff is purely
additive — no other example's committed output moved.

---

## #309 — بتات_أو_حصري, the third bitwise primitive

Branch `feature/309-bitwise-xor-builtin`. Increment A of
[`builtins-vs-stdlib.md`](builtins-vs-stdlib.md) §6.1, three names of seven.

### The estimate held a third time

Same two source files as #302 and #306, and no third. `BinaryOp::BitXor` already had arms in
the interpreter (`executor/mod.rs:981`), the debug interpreter
(`debug/interpreter/operations.rs:117`), both JIT tiers (`bxor`), LLVM (`xor i64`), the
constant folder and CSE. The shared arm became a three-way `match name`, and the outer
pattern grew one alternative.

### The mandated lexer check passed, and the risk was different from #302/#306

§6.1 singled this name out. In the two earlier names the keyword was the **suffix**, so only
a greedy scan was at issue. Here `أو` sits **mid-name**, where a split would not have failed
loudly — `بتات_` `أو` `_حصري` is a well-formed logical-or between two identifiers, so it
would have type-checked as `منطقي` and produced a confusing error far from the cause. The
scan handles it, and the existing lexer test was widened (and renamed from `ending_in` to
`containing`) to pin all three spellings.

### Why XOR is a primitive and not a convenience

It is not derivable from the two that landed: `أ ^ ب` = `(أ | ب) & ~(أ & ب)` needs a
complement, and until this name there was none. Two properties earn it the slot:

- **Self-inverse.** `بتات_أو_حصري(بتات_أو_حصري(س، ق)، ق) == س`. AND and OR are both
  absorbing; neither can undo itself, so masking needed a saved copy to round-trip.
- **`بتات_أو_حصري(س، -1)` is the bitwise complement.** `-1` is all-ones in a signed 64-bit
  `عدد`. This retires the hand-written `-256` inverse mask that #306 left in the example with
  a «إلى أن تصل بتات_نفي» note — the example now computes it as `متمم(255)` and asserts the
  two agree.

Consequence for the plan: `بتات_نفي` is now a *spelling* for an expressible operation, not a
missing capability. It should still land for readability, but it no longer gates anything.

### #304 reproduces here too, and is not this change's

`ثابت م = [بتات_أو_حصري(12، 10)، 1]` prints `6` interpreted and exits 139 natively —
identical to `بتات_و`, `بتات_أو` and `طول_مصفوفة`. Confirmed by running it rather than
assumed, as #306 did. Nothing here touches it, and the example file avoids that shape.

### One authoring trap worth recording

The first draft of the example used `ثابت الأصل = 4660` and failed to parse with ب٠٢٠١.
`الأصل` is the parent-class reference keyword. `.claude/rules` already warns off `ك` and `و`
as loop identifiers; `الأصل` belongs on that list, and it is easy to reach for because it is
the natural Arabic word for "the original" in a round-trip test.

### Verification

Interpreter, JIT, native and the DAP debug interpreter all produce byte-identical output for
`examples/مدمجات.ترقيم`, and the regenerated `examples/متوقع/مدمجات.خرج` diff is purely
additive — 25 insertions, no other example's committed output moved. The DAP leg was checked
with `printf 'r\nc\nq\n' | tarqeem debug`, stripping the `ترقيم> ` prompt and `[DEBUG]`
lines before comparing.


---

## #312 — بتات_نفي, the fourth bitwise primitive and the first unary one

Branch `feature/312-bitwise-not-builtin`. Increment A of
[`builtins-vs-stdlib.md`](builtins-vs-stdlib.md) §6.1, four names of seven.

### The estimate held a fourth time, and this time on a new shape

Same two source files as #302, #306 and #309, and no third. What made this run worth doing as
its own test of the estimate is that the three before it were all `Instruction::Binary`;
`بتات_نفي` is the family's only `Unary` member, and the estimate had never been exercised
there. `UnaryOp::BitNot` already had arms in the interpreter (`executor/mod.rs:1021`), the
debug interpreter (`debug/interpreter/operations.rs:162`), both JIT tiers (`bnot`), LLVM
(`xor i64 %x, -1`) and the constant folder; CSE keys `Unary` too. So the shape difference cost
nothing but a second `match` arm — it could not share the binary one, which reads `args.get(1)`.

The estimate held for the two files, but the *backend* claim did not: the real binary/unary
asymmetry in codegen is not the destination type — `Instruction::Unary` and `Instruction::Binary`
both take it from the instruction's own `ty` field (`codegen.rs:1097` and `:1083`) — it is
**operand unboxing**. `emit_binary` loads a narrowed optional back out of its box (#185);
`emit_unary` did not, so `بتات_نفي(س)` after `إذا (س != لا_شيء)` emitted `xor i64 %ptr, -1` and
clang rejected the whole module, while the interpreter and JIT both answered correctly. Fixed by
mirroring `emit_binary`'s unbox in `emit_unary`, which also repairs `-س` and `ليس س` on a
narrowed optional; pinned by `test_bitwise_not_over_a_narrowed_optional`. Whoever lands the
shifts should assume the *unary* path is the less-travelled one and probe a boxed operand first.

Still open there: unary `Neg` over a narrowed `عدد_عشري?` emits `sub i64` because the IR builder
types that `Unary` as `Int`, so the unbox gate (pointee must equal the instruction's `ty`) does
not fire. Pre-existing and unrelated to the bitwise family, which is `Int` only.

### It is a spelling, not a capability — and the tests say so

#309 already made the complement reachable: `بتات_أو_حصري(س، -1)` flips every bit. This name
was landed anyway, per its unchanged §1.3 verdict, for call-site readability and registry
completeness — `بتات_نفي(٢٥٥)` states the operation, while `بتات_أو_حصري(٢٥٥، -١)` requires
the reader to already know that `-1` is all-ones.

Because it duplicates a reachable operation, the discriminating probe is **agreement**, not
truth-table correctness: `test_bitwise_not_agrees_with_xor_against_all_ones` asserts
`بتات_نفي(س) == بتات_أو_حصري(س، -1)` over positive, zero and negative operands. That is what
pins the new arm to `BitNot` specifically — a wrong unary op would still return an integer and
still print plausibly. The two's-complement identity `بتات_نفي(س) == -س - ١` is the second
such probe, and it is also what would catch an operand narrower than i64.

### The stale note in the example is now closed

#306 left `examples/مدمجات.ترقيم` writing the inverse low-byte mask as the literal `-256` with
a «إلى أن تصل بتات_نفي» comment; #309 half-answered it with a `متمم` helper. The example now
computes the mask as `بتات_نفي(255)` and asserts all three forms agree, so the literal that
remains in `استبدل_البايت_الأدنى` is there deliberately as the contrast rather than as a
placeholder.

### The lexer check applied here too

`في` is a keyword (`TokenKind::In`, `keywords.rs:28`) and `بتات_نفي` ends in it — the same
suffix position as `و` in `بتات_و` and `أو` in `بتات_أو`, and less dangerous than
`بتات_أو_حصري`'s mid-name `أو`, since a split here fails loudly rather than parsing as a
different expression. `test_identifier_containing_a_keyword_stays_one_token` now covers all
four spellings and its comment names all three keywords.

### Verification

Interpreter, JIT, native and the DAP debug interpreter all produce byte-identical output for
`examples/مدمجات.ترقيم` — the DAP leg compared after stripping its three-line banner and the
trailing `انتهى البرنامج` line. The regenerated `examples/متوقع/مدمجات.خرج` diff is purely
additive: 19 insertions, and no other example's committed output moved. Full suite green and
`cargo clippy --all-targets` clean.

**What that four-backend agreement did *not* cover, and the lesson.** Every operand in the
example and in the first six probes was an unboxed `عدد` — a literal, a `ثابت`, or a typed
parameter. So the diff was byte-identical across four backends while the native backend could
not compile the same call over a *narrowed optional* at all. Backend-diff on an example is
necessary and it is not sufficient: it only samples the operand shapes the example happens to
use. `.claude/rules` §11 rule 5 already says to test composition rather than printing; the
sharper form is to test composition **over each operand representation** the type can take.

### #304 — retested during review

The three prior runs each confirmed by hand that an intercepted builtin segfaults natively
inside an array literal, and this run initially took that as established rather than re-running
it. It was then reproduced: `ثابت م = [بتات_نفي(255)، 1]` prints `-256` interpreted and exits
139 natively, so §6.1's "confirmed unchanged by #306, #309 and #312" is accurate.

---

## #317 — بتات_إزاحة_يسار, the fifth bitwise primitive and the first chain

Branch `feature/317-bitwise-shl-builtin`. Increment A of
[`builtins-vs-stdlib.md`](builtins-vs-stdlib.md) §6.1, five names of seven.

### The range question the three prior runs deferred

#312 deferred all three shifts for one stated reason: shift amounts outside 0-63 "diverge three
ways (the interpreter errors, LLVM poisons, Cranelift masks)". Re-verified against source it is
**four**, and the fourth is the one that mattered — the constant folder's `wrapping_shl`
(`const_fold.rs:180`) masks, and `Optimizer::optimize` runs only in `compile`
(`cli/commands/compile.rs:167`). So a bare `Shl` would have made native disagree with the
interpreter *and with itself*, depending on whether the amount was a literal or a variable.

The decision recorded in §6.1 and in LANGUAGE_SPEC §8.6: **the operation is total, and an amount
outside 0-63 yields 0.** Two alternatives were weighed and rejected.

- **Abort, matching the interpreters' existing `Shl` arms.** It is the loud option, but it cannot
  be gated the way §11 rule 4 requires: the backend-diff jobs compare stdout of runs that
  *succeed*, and the abort text itself already differs between backends. It also needs
  branch-to-panic block machinery inside `build_core_builtin_call`, which no lowering has.
- **Mask the amount mod 64**, which is what C, Cranelift and `wrapping_shl` do. One instruction,
  but it makes `بتات_إزاحة_يسار(١، ٦٤) == ١` — a transliterated behaviour rather than a described
  one, which is what `arabic-philosophy.md` rule 1 exists to refuse.

`٠` is not a sentinel; it is the arithmetic answer. Shifting a 64-bit value by 64 or more moves
every bit out of the word.

### The estimate held a fifth time, on a chain rather than one op

Still the same two source files. What is new is that this is the first lowering that is **not**
one instruction: the guard is `ن >> ٦` (zero exactly on 0-63), `high | -high` to carry the sign
whenever `high` is non-zero, an arithmetic shift by 63 to spread it into a -1/0 mask, `BitNot` to
invert it, `ن & ٦٣` to keep the amount inside every backend's own accepted range, then the shift
and a final mask.

Two properties of that chain were chosen deliberately and are worth keeping if the right shifts
copy it:

1. **No `BoolToInt`.** The obvious guard is `valid = high == ٠` widened to an integer, which is
   two fewer instructions. `Instruction::BoolToInt` has arms in the interpreter, the debug
   interpreter and LLVM — and in **neither JIT tier**, which would have made this the one builtin
   that cannot be JIT-compiled the day tiering is switched on. Every op the chain does use
   (`Shr`, `Sub`, `BitOr`, `BitNot`, `BitAnd`, `Shl`) has an arm in all six consumers including
   the constant folder, so the whole chain folds away when both arguments are literals.
2. **No subtraction of the amount itself.** `٠ - (ن >> ٦)` is safe for every `عدد` because
   `high` spans `[-2^57, 2^57-1]`; `٦٣ - ن` or `٠ - ن` would overflow at `i64::MIN` and panic the
   interpreter in a debug build. `test_left_shift_handles_the_most_negative_amount` pins it,
   reaching that amount as `بتات_إزاحة_يسار(1، 63)` because the literal cannot be written.

### #318 — reading one operand twice is what found it

The chain needs the amount twice, and that is a shape no previous lowering had. Natively it
produced `%v27 = and i64 %v18, %v19` where `%v18` is a pointer: `emit_binary` unboxes a narrowed
optional by binding the load to a **fresh** SSA name and then rewriting `var_types` for the
variable, so the second use of the same `VarId` sees `Int`, skips the unbox, and emits the
original pointer name.

It is not caused by this builtin — `س + س` inside `إذا (س != لا_شيء)` reproduces it on `develop`,
printing `10` interpreted and failing clang natively — so it is filed as #318 rather than fixed
here. The obvious fix is a trap worth recording: rebinding the variable's *name* to the unboxed
value would break a later `س != لا_شيء` in the same branch, which still needs the pointer and does
not unbox.

The workaround in the lowering is one instruction, `أ | ٠`, which reads the amount once through a
copy. It is free after either optimizer.

Two separate *statements* using the same optional are fine (`اطبع(س + 1)` then `اطبع(س + 2)`
compiles and runs) because the builder emits a fresh load per statement. That is why the
narrowed-optional fixtures from #312 pass — none of them puts one `VarId` in two operand
positions. Same lesson as #312's, one level further in: backend-diff on an example samples only
the operand shapes the example happens to use, and now also only the *reuse patterns* it happens
to use.

### The lexer test was checked and deliberately not extended

`بتات_و`, `بتات_أو`, `بتات_أو_حصري` and `بتات_نفي` each embed a keyword (`و`, `أو`, `في`) and are
pinned in `test_identifier_containing_a_keyword_stays_one_token`. `بتات_إزاحة_يسار` embeds none,
so adding it would have diluted what that test is for. Recorded here because "the previous four
all touched it" is exactly the kind of pattern that gets copied without checking.

---

## #320 — بتات_إزاحة_يمين, and correcting a contract the family was told to inherit

Branch `feature/320-bitwise-ashr-builtin`. Increment A of
[`builtins-vs-stdlib.md`](builtins-vs-stdlib.md) §6.1, six names of seven.

### The interesting part is not the code

The lowering needed no new mechanism, and once the guard was shared it needed no extra
instruction either — both tails are three ops over the same six-op guard. What this change
is actually about is that **#317 wrote its conclusion into the plan as a family-wide rule, and the
conclusion did not generalise while its reasoning did.**

#317's contract: *"an amount outside 0-63 yields `٠`, and the whole family inherits it verbatim."*
Its justification, quoted from the issue: *"`٠` is the arithmetic answer, not a sentinel. Shifting
a 64-bit value left by 64 or more moves every bit out; the low 64 bits are zero."* Both true, and
the rejection of C's mask-mod-64 rested on the second sentence — `arabic-philosophy.md` rule 1,
describe rather than transliterate.

An **arithmetic** right shift refills the vacated high end from the sign, so shifting everything
out leaves the sign rather than zero. Carrying the constant across would have produced:

```
بتات_إزاحة_يمين(-١، ٦٢)  → -١
بتات_إزاحة_يمين(-١، ٦٣)  → -١
بتات_إزاحة_يمين(-١، ٦٤)  → ٠     ← nothing about the operand changed
```

That cliff is precisely the sentinel #317 refused. So the number changed and the rule did not:

> An amount outside 0-63 is a complete shift, and the vacated bits are filled the way that shift
> always fills them.

Zeros for `يسار` (#317 unchanged), the sign for `يمين`, zeros again for the pending
`بتات_إزاحة_يمين_منطقية` — so **one of the three names moves and the seventh is pre-committed to
nothing.** The generalisation also buys the right shift the counterpart of the identity the spec
already documents for the left one: a left shift is multiplication by powers of two bounded by the
sign bit, a right shift is *floor* division by them, and under the corrected rule that holds at
every `ن ≥ ٠` rather than stopping at 64.

**The lesson worth keeping is about the plan document, not about shifts.** A decision recorded as
"the whole family inherits this" is safe only when what is inherited is the *reasoning*. #317
recorded both, and the two diverged one name later. Where a future increment writes a rule for
names it has not implemented, state the criterion, not the answer.

### Floor versus truncation is the property the tests had to pin

`/` truncates toward zero and this shifts toward negative infinity, so on negative operands they
are different operations:

```
بتات_إزاحة_يمين(-٧، ١)  → -٤
-٧ / ٢                   → -٣
```

`test_right_shift_floors_where_division_truncates` asserts the disagreement rather than only the
agreement, which is what stops a later "simplification" into a division. The companion risk is a
backend wired to `lshr` instead of `ashr`: every such fixture would print a large positive integer,
plausible enough that no arithmetic assertion catches it, which is why
`test_right_shift_propagates_the_sign` exists as its own case with negative operands only.

### The guard is now shared, which is the whole diff in `expr_builder.rs`

Both shifts need the same six-op range guard (over three constants) and differ only in the
three-op tail:

- `يسار` masks the **result** to zero out of range — `shifted & keep`.
- `يمين` saturates the **amount** to 63 — `guard.amount | (٦٣ & out_of_range)`, which needs no
  select because the guard's masked amount already fits in those six bits, and 63 is exactly the
  amount at which the value is already fully shifted out.

Extracting `emit_shift_range_guard` was not tidying: the guard is subtle (a `-1/0` mask built
without `BoolToInt`, which **neither JIT tier implements**, and without subtracting the amount
itself, which would overflow at `i64::MIN`), and two copies of it would have been two places to
get it wrong. The left shift emits the same instructions as before — only the `BitNot` moved, out
of the shared guard and into the arm that consumes it — which its nine existing fixtures verify.

**That relocation is not cosmetic, and the reason generalises to every shared lowering.** The
first cut left `keep` in the guard, where the right shift never reads it. `Optimizer::optimize`
runs **only** in `compile` (`cli/commands/compile.rs:167`), so `tarqeem run` and both JIT tiers
execute unoptimized IR: the dead `BitNot` was not folded away, it ran, on every call, in the two
backends people actually use by default. Anything a shared helper emits that only one caller
consumes belongs in that caller.

### The estimate held a sixth time, and that is now the finding

Two source files, no `runtime-rs` work, no runtime symbol, no interpreter or debug-interpreter
arm, no `register_builtin_return_types` entry. #312 had extended the estimate to `Unary` and #317
to a multi-instruction chain; #320 exercised nothing new, which is the point — the interception
path is now characterised rather than merely observed.

`Shr` was verified arithmetic in all six consumers before anything was written: `ashr i64`
(`codegen.rs:2626`), `sshr` (both Cranelift tiers), `*a >> *b` on `i64` (both interpreters), and
`wrapping_shr` (`const_fold.rs:181`). Had any one of them been logical, this would have been a
`runtime-rs` change instead.

### The lexer test was checked and deliberately not extended, again

`بتات_إزاحة_يمين` embeds no keyword — neither `إزاحة` nor `يمين` contains one — so
`test_identifier_containing_a_keyword_stays_one_token` was left alone, for the same reason #317
left it alone. Noted a second time because the trap is the pattern, not the name.

---

## #322 — بتات_إزاحة_يمين_منطقية, and a justification that expired before it shipped

Branch `feature/322-bitwise-lshr-builtin`. Increment A of
[`builtins-vs-stdlib.md`](builtins-vs-stdlib.md) §6.1 — **seven of seven, complete**.

### The finding is that the plan's own case for this name had gone stale

§1.3 justifies `بتات_إزاحة_يمين_منطقية` under criterion (a), inexpressible: *"every backend's `Shr`
is arithmetic, so a self-hosted xorshift64 or DEFLATE bit reader silently produces wrong numbers
consistently across all three backends without it."* Every clause of that is still true. The
conclusion is not — **#320 made the operation composable from the six names that already existed**:

```tarqeem
بتات_أو(
    بتات_إزاحة_يمين(بتات_و(س، 9223372036854775807)، ن)،
    بتات_و(بتات_إزاحة_يمين(س، 63)، بتات_إزاحة_يسار(1، 63 - ن)))
```

That is asserted rather than asserted-about, by
`test_logical_right_shift_matches_the_composition_it_names`, including out of range where the
composition's two inner guards happen to agree with the single guard.

So the name shipped on different grounds than the ones written for it, and the honest grounds are
worth stating because two of them are new:

1. **`بتات_نفي`'s precedent (#312)** — call-site readability. That case is stronger here: the
   composition rests on three separate non-obvious facts (that `9223372036854775807` is the
   sign-cleared mask, that a non-negative operand makes an arithmetic shift behave logically, and
   that `٦٣-ن` stays in range for every in-range `ن`). Getting any one wrong yields a plausible
   large integer.
2. **There was nowhere else to put it.** §5.2 keeps a no-import name a compiler builtin until the
   linker treats prelude declarations as displaceable (**B12**), and this family is core tier. The
   choice was primitive or nothing — which is *not* true of the names Increment C onward will move.

**The generalisable part:** an inexpressibility claim is a statement about the language at the time
of writing, and each landed increment changes what is expressible. Two of the 21 `new` rows have now
had that claim expire under them, both inside one increment. Re-derive criterion (a) at the start of
each increment rather than reading it off §1.3. The verdicts are still right; the *reasons* decay.

### The range contract predicted this name and was not adjusted for it

#320 replaced #317's constant with a criterion — *"an amount outside 0-63 is a complete shift, and
the vacated bits are filled the way that shift always fills them"* — and wrote that it left the
seventh name at `٠`, unchanged from #317's number. It shipped at `٠`, and that is the criterion's
own test rather than a coincidence: it was written before the name it predicted, and it produces `٠`
here for a **negative** operand where the sibling produces `-١`.

```
بتات_إزاحة_يمين(-١، ٦٤)          → -١
بتات_إزاحة_يمين_منطقية(-١، ٦٤)   → ٠
```

The two right shifts agree on the rule and disagree on the number, out of range exactly as in
range. A rule stated as a criterion survived one name further than the rule stated as an answer.

### The plan's implementation sketch was wrong, and the correct shape is cheaper

§1.3's cost note sketched `(أ >> ١) & 0x7FFF…FFFF` then `>> (ن-١)`, *"with `ن==٠` returning `أ`"*.
That parenthetical is a select, and there is no `Select` in the IR — building one out of masks
costs four instructions plus a zero-detect, on top of the guard. Separating the sign bit instead
needs no special case at all:

```
keep  = ~oob                    the guard's flag, complemented
value = س & keep                zero out of range — and the #318 copy, in one instruction
low   = value & ٩٢٢٣٣٧٢٠٣٦٨٥٤٧٧٥٨٠٧
lowsh = low >> amount           non-negative operand, so Shr behaves logically
sign  = value >> ٦٣
dest  = lowsh | (sign & (١ << (٦٣ - amount)))
```

Nine instructions over the shared six-op guard — the last line above is four of them, which is how
the first draft of this entry came to call it eight. `ن = ٠` falls out: `١ << ٦٣` is `i64::MIN`,
so the sign term restores the operand's top bit and `low` supplies the other 63 — which is why
`بتات_إزاحة_يمين_منطقية(بتات_إزاحة_يسار(١، ٦٣)، ٠)` is `i64::MIN` and not `٠`. That input is the
lowering's tightest spot, since clearing the sign bit leaves exactly zero and the entire answer
comes from the sign term; it has its own fixture.

`٦٣ - amount` reads the guard's **masked** amount, not the raw one, so it stays in `٠..٦٣` and
cannot overflow — the same discipline #320 recorded for `٠ - (ن >> ٦)`.

### Folding the #318 workaround into work the arm already does

This is the first *shift* to read the **value** twice, so it needed the copy #317 introduced for
the amount. It did not need a second `أ | ٠`: it also owed the out-of-range answer a zero, and
`س & keep` is one instruction that does both — it is the value's first scalar use, which is where
codegen unboxes a narrowed optional, and zeroing the value zeroes every term below it.

Generalisable: where an arm already needs a mask or a copy, fold the #318 workaround into it rather
than prepending `أ | ٠`. Either form is load-bearing and neither is an optimizable identity — a
peephole for `x | 0` **or** `x & -1` would silently restore #318, natively only.

Note where the mask goes differs across all three shifts, and each position is forced: `يسار` masks
the *result*, `يمين` saturates the *amount* to 63, and this one masks the *value*. Only the third
position both zeroes the answer and unboxes the operand.

### The lexer test was extended this time, and that is the point

#317 and #320 each recorded that they checked
`test_identifier_containing_a_keyword_stays_one_token` and deliberately left it alone, because
neither `إزاحة` nor `يمين` nor `يسار` contains a keyword. `منطقية` does — `منطقي` is the `منطقي`
type keyword — and it sits there in a shape none of the four existing cases covers: followed by a
*letter* (`ة`) rather than by `_` or the end of the name, so a scan resuming after a keyword match
would split a word rather than a separator.

Recorded because two consecutive entries reached the opposite conclusion, which is exactly how a
"we already checked this" pattern gets copied without checking. The check is per name; the answer
changed on the seventh.

### The estimate held a seventh time, over the longest tail yet

Two source files, no `runtime-rs` work, no runtime symbol, no interpreter or debug-interpreter arm,
no `register_builtin_return_types` entry. #312 extended the estimate to `Unary`, #317 to a
multi-instruction chain, #320 to a second chain over a shared helper; #322 is the longest tail of
the three (nine ops versus three) and still added no mechanism. Every op used — `Shr`, `Shl`,
`Sub`, `BitAnd`, `BitOr`, `BitNot` — was confirmed to have an arm in all six consumers before
anything was written, and the chain folds to a constant when both arguments are literals.

Verified in all four executing backends, DAP debug interpreter included
(`printf 'r\nc\nq\n' | tarqeem debug FILE`).


## #324 — حرف_إلى_رمز, and the first increment where the two-file estimate did not apply

First name of Increment B, the character/byte bridge, and the first **new symbol-mapped** core
builtin in the sequence. Signature `حرف_إلى_رمز(س: نص) -> عدد`: the Unicode scalar value of a
string's first codepoint, `-1` when there is none.

### The template changed, and the previous seven increments point at the wrong file

Seven consecutive increments landed in two files, and `src/ir/builder/expr_builder.rs` was the
whole native story for all of them. Here it needed **no** edit. A core builtin absent from
`build_core_builtin_call` falls through to `Instruction::Call { func: FuncId(arabic_name) }` by
itself, and that `FuncId` carries the *Arabic* name all the way to codegen — the `trq_*`
substitution happens only inside `mangle_function_name`. Two consequences worth stating because
both are easy to get backwards:

- Both interpreters key their arms on `"حرف_إلى_رمز"`, not on `"trq_string_char_code"`. The
  `trq_*` arms that `عدد` and `طول` carry exist only because the IR builder rewrites *their*
  `FuncId` via `emit_call`; nothing rewrites this one.
- `register_builtin_return_types` is the **only** thing that types the result. Unregistered it
  takes the `Ptr(Void)` sentinel from `type_helpers.rs`, and codegen then emits `call ptr` against
  a `declare i64` — the same defect the `جذر` comment beside it records.

The working template is `توقف` / `تأكد`: core tier, symbol-mapped, native. Not `بتات_و`. Grepping
every occurrence of `"توقف"` in `src/` produced exactly the site list this change needed and added
nothing, which is a cheaper way to find the surfaces than reasoning about them.

Nine sites, and the count in `docs/builtins-inventory.md` §0 was right all along — it just had
never been paid, because every increment since it was written took the two-file path.

### Three findings the plan did not contain

**An un-narrowed optional reaches a concrete parameter.** `Type::compat` accepts
`Optional(inner)` into `t` whenever `inner` is compatible, so `متغير س: نص? = لا_شيء` followed by
`حرف_إلى_رمز(س)` type-checks. Native lowers it to `ptr null`, where the runtime's null guard
answers `-1`; an interpreter arm keyed only on `Value::String` would have raised a type error and
exited non-zero. Two ordinary lines, a silent cross-backend divergence, and nothing in the
existing suite shape would have caught it — `assert_prints` compares stdout, and one side was
going to abort. Both interpreters therefore carry `Value::Null => Ok(Value::Int(-1))`, which is the
same "no first character" contract the native guard already implements.

Generalisable: **any symbol-mapped primitive whose runtime function guards null needs a matching
`Value::Null` arm in both interpreters.** The guard is not an implementation detail of the native
leg; it is part of the contract, and the other backends have to honour it.

**`as_str` trims.** `runtime-rs/src/string.rs`'s convenience accessor ends
`.map(|text| text.trim())`, which is right for the number parsers it was written for and wrong for
anything char-level: `حرف_إلى_رمز(" أ")` would have answered `1571` natively and `32` interpreted.
The char-aware family's own convention — raw `from_raw_parts` plus the private `utf8_char_len` — is
the one to follow, and reusing that helper also keeps the whole family agreeing on what "one
character" means.

**Decode the first character's bytes, not the buffer.** `std::str::from_utf8` over the whole slice
fails when *any* later byte is invalid, which would discard a perfectly decodable first character.
Since `ثنائي_إلى_نص` is specified *not* to validate, that input is coming. So the lowering takes
`utf8_char_len(bytes[0]).min(bytes.len())` bytes and decodes only those — the `.min` is not
defensive padding, it is what keeps a truncated multi-byte tail from panicking inside an
`extern "C"` fn, which would abort the process rather than return.

### `-1` had to break the family's convention

`trq_string_len_chars` returns `0` for null and empty, and copying that would have been wrong here:
U+0000 is a real codepoint, so `0` cannot distinguish "empty" from "the NUL character". One
sentinel, `-1`, meaning "no first character" — and it covers an undecodable first character too,
rather than inventing a second sentinel for a case unreachable from source today.

Worth noting because the convention was the default and the default was wrong. A sentinel is
only safe when it is outside the value domain, and for a codepoint accessor `0` is inside it.

### `حرف_في` is the shape this deliberately is not

`حرف_في` and `طول_حروف` are registered in `scope.rs` and `codegen.rs` and **nowhere else** — no
interpreter arm, no debug arm, no test, no example. They are native-only, and the stdlib tier they
sit in segfaults natively on import (#185). That is why this name went to the **core** tier even
though `حرف_في` makes the `نص` tier look like the natural home: the core tier is the only one the
registry guard and the cross-backend sweep police.

The `نص`-tier registry is documented as having 78 names with no interpreter arm. `حرف_في` being
two of six surfaces is not a new discovery, but it is the concrete instance next door to this
change, and the reason not to copy it.

### Verified in all four executing backends, and the fourth needed a new test

`tests/builtins_execution_tests.rs` drives `run`, `run --jit` and `compile`; nothing in it reaches
`src/debug/interpreter/`. So the debug arm got an assertion in that file's own `mod tests`, and
unlike the existing `test_time_builtins_are_dispatchable` — whose doc comment claims both lists are
checked while the body only checks `is_builtin` — it constructs a `DebugInterpreter` and calls
`call_builtin`, covering the `Null` case as well. `DebugInterpreter::new(Module, DebugContext)` is
cheap enough that the overstatement was never necessary.

### The lexer check was run, and this time it needed nothing

`حرف_إلى_رمز` embeds no keyword: `إلى`, `حرف` and `رمز` are absent from `keywords.rs`, only
`وإلا`/`والا` exist, and the name contains no `و`, no `في`, no `ك` and no `منطقي`. So
`test_identifier_containing_a_keyword_stays_one_token` was deliberately left alone, as in #317 and
#320 rather than #322. Recorded because the check is per name and the answer has now gone both ways.

### Criterion (a) was re-derived and held — the first time under #322's rule

#322 ended with "re-derive criterion (a) at the start of each increment rather than reading it off
§1.3", after two of the 21 `new` rows had it expire. This is the first application, and the claim
**holds**: `نص_إلى_ثنائي` does not exist (**B9**), `س[i]` and `لكل ح في س` still yield an untyped
`Ptr(Void)` (**B6**), and nothing turns a character into a number. The rule is not "the claim has
always gone stale" — it is that the claim must be checked, and here the answer was different.

### Small observation, not fixed

`runtime-rs/src/lib.rs`'s `pub use string::{…}` block omits `trq_string_to_int_checked` and
`trq_string_to_float_checked`; they link anyway, because `#[no_mangle] pub extern "C"` exports the
symbol from the staticlib regardless of Rust-level visibility. `trq_string_char_code` was added to
the block since a new export belongs in it, but the two older omissions were left alone rather than
folded into an unrelated change.

## #326 — رمز_إلى_حرف, and a sibling's lesson that did not generalize

Second of the five names in Increment B, and the inverse of #324. Signature
`رمز_إلى_حرف(رمز: عدد) -> نص`: the one-character string holding `رمز`, and `""` when `رمز` is not
a Unicode scalar value.

The nine-site path #324 measured held exactly, and finding nothing new about it is the result.
The interesting part was elsewhere.

### The `Null` arm was the wrong thing to copy

#324 closed with a rule: *any symbol-mapped primitive whose runtime function guards null needs a
matching `Value::Null` arm in both interpreters.* Copying it here would have been wrong, and the
probe is what showed it.

`Type::compat` admits an un-narrowed `عدد?` into an `عدد` parameter exactly as it does `نص?` into
`نص`, so `رمز_إلى_حرف(غائب)` type-checks. But the two cases diverge underneath. For `نص?` the
argument is already a pointer, native passes `ptr null`, and `trq_string_char_code`'s own guard
answers `-1` — a **designed** answer that the interpreter arm mirrors. For `عدد?` there is no
null to guard: codegen turns `لا_شيء` into integer `0` *above* the runtime, and `0` is a perfectly
valid codepoint, so the runtime cannot tell it apart from a real call. Mirroring that would have
written "`لا_شيء` means U+0000" into the language.

So this name has **no** `Null` arm, and both interpreters keep the `type_error` fallthrough —
which is what `نم` and `بتات_نفي` already do on the identical source.

The narrowed rule: check whether the mechanism that produced the sibling's answer exists for this
name before mirroring its edge-case arm. A guard is a contract only where there is something to
guard.

### The probe found a bug that is not this name's, and is worse than #318

Chasing the above produced **#327**. A *narrowed* optional passed as a call argument is never
unboxed natively:

```tarqeem
دالة هوية(س: عدد) -> عدد { أرجع س }
متغير موجود: عدد? = 1605
إذا (موجود != لا_شيء) {
    اطبع(هوية(موجود))      // 1605 مفسَّراً، 39040583984 أصلياً
    اطبع(بتات_نفي(موجود))  // -1606 في الثلاثة
    اطبع(موجود + 0)        // 1605 في الثلاثة
}
```

A plain user function reproduces it, so it is the call-argument path, not a builtin defect.
Arithmetic unboxes and an IR-intercepted builtin unboxes; only the call argument does not — and
it is the one position where the wrong value provokes no type mismatch in the emitted IR, so
clang accepts the module and the binary runs to completion printing a pointer. That makes it
strictly worse than #318, which at least fails to compile. Filed rather than fixed: it predates
this change, this name neither depends on it nor works around it.

The un-narrowed half of the same site is recorded there too — `نم(غائب)` prints `نام` and exits 0
natively while both interpreters raise a type error, and `بتات_نفي(غائب)` segfaults.

### Why `""` and not an abort

The rejection convention was the one genuine design decision, and the spec fixed only that
rejection must happen, not how. `""` was chosen over `reject_unparsable`'s `exit(1)` because it
mirrors `حرف_إلى_رمز`'s `-1` and closes the round trip in both directions: `حرف_إلى_رمز("")` is
`-1`, so every rejected code maps to `""` and back to `-1` rather than into a hole. It also
matches every other string constructor in `string.rs`, which returns an empty `TrqString` on bad
input and reserves raw null for allocation failure alone. `عدد`'s abort contract is explicitly
the *checked* half of a documented checked/lenient pair (D5); this name has no lenient sibling
to be the checked half of.

### The range check has to be on the `i64`

`char::from_u32(code as u32)` alone would be wrong: `4294967361` truncates to `65` and answers
`"A"`. `u32::try_from` first sends that and every negative through the same rejection as a
surrogate. It is pinned at both levels — a `runtime-rs` unit test and a cross-backend one — since
a cast that silently succeeds is exactly the shape that passes a printing test.

### Rejections are asserted through `طول`, never by printing

`Output::lines()` in the execution harness trims, so a printed empty string is indistinguishable
from a printed newline and an `assert_prints` on `""` could not fail. Every rejection case in
both the tests and `examples/مدمجات.ترقيم` goes through `طول` or the round trip. The same reason
keeps `رمز_إلى_حرف(٠)` out of the printed output: it is a real one-character string, but what a
NUL byte does on stdout is a question about the terminal, not about the contract.

### The lexer check was run, and needed nothing

`رمز`, `إلى` and `حرف` are absent from `keywords.rs`, and the name contains no `و`, `أو`, `في`,
`ك` or `منطقي`. So `test_identifier_containing_a_keyword_stays_one_token` was left alone, as in
#317 and #320 rather than #322. Recorded because the check is per name and the answer has now
gone both ways twice.

### Criterion (a) re-derived, and it held a second time

`نص(٦٥)` formats the digits `"65"`; `ثنائي_إلى_نص` does not exist (**B9**); `س[i]` and
`لكل ح في س` are still `Ptr(Void)` (**B6**); and `حرف_في` reads a character out of a string that
already exists rather than making one. With this name **B9** is half-closed the other way round:
char↔code works in both directions, and only the string↔bytes bridge remains.

---

## #330 — نص_إلى_ثنائي, and a "first" that was only a first for its tier

Increment B's third name (3 of 5), and the string→bytes half of the byte bridge: `(نص) -> مصفوفة<عدد>`,
the UTF-8 octets of a string, one element per byte. `ثنائي_إلى_نص` and `قص_حروف`'s repair (**B7**)
remain.

### The expensive-looking part was already paid for

This was expected to be the costly name in the increment, because no **core** builtin had ever
returned an array — `طول_مصفوفة` and `الحق` are both declared over `أي`, so this is the first core
entry using `Type::Array` at all. In fact it cost the same nine sites as #324 and #326, plus one
lexer test, and **zero new mechanism**.

The reason is that the *stdlib* tier had already paid for array returns and nothing about the
mechanism is tier-specific: `اضغط` is `(نص) -> مصفوفة<عدد>` — this name's exact signature — with
`IrType::Array(Box::new(IrType::Int), 0)` registered since #241, and `examples/تشفير_وضغط.ترقيم`
composes such a result with `طول` and arithmetic across all three backends in CI today. Four
stdlib names register that same return type.

Generalisable, and the mirror image of Increment A's lesson: **when something looks like a first,
check whether it is a first for the *mechanism* or only for the *tier*.** Here it was only the
tier, and the whole cost estimate followed from that one distinction. The check is cheap — grep the
return-type map for the shape before budgeting for it.

### A missing return type is quieter for an array than for a scalar

§1.1 rule 5 and `register_builtin_return_types`' own `جذر` note describe the consequence of a
missing entry as a **signature mismatch** — `call ptr` emitted against a `declare i64`, which fails
loudly. That description does not hold for an array return, and the difference is not academic.

`IrType::Ptr(Void)` and `IrType::Array(..)` both map to LLVM `ptr`, so there is no mismatch to
catch: the module is valid, links, and runs.

Verified by deleting the entry and running the suite. Indexing still answered `65`, arithmetic still
answered `66`, `==` still answered `صحيح`, and `اطبع` still printed `[104، 105]`. The **only**
assertion that failed was `نوع(…)`, which returned `مؤشر` instead of `عدد`. So four of the five
lines in the composition test would have passed on a broken build.

That is why the test asserts `نوع` at all, and it is worth stating because a composition test
written without it — printing, concatenating, comparing — would have looked thorough and caught
nothing. For a scalar return the mismatch fails at clang; for an array the type map is the *only*
witness.

### `نص(<array>)` — a source trace that was wrong in the direction that matters

The plan carried a warning, derived from reading `convert_to_string`, that `نص(نص_إلى_ثنائي(س))`
would **fail native compilation** while both interpreters printed `[104، 105]`. The reasoning was
sound: there is no `Array` arm, so the argument falls through to `trq_int_to_string`, whose declare
takes `i64` while the argument lowers to `ptr`.

Measured, it is the reverse. Both interpreters raise «خطأ في النوع: متوقع عدد، وُجد array», and
**native compiles, runs, prints `4353416272` and exits 0.** clang accepts the module because the
`ptr` is simply dropped into the `i64` slot — `runtime_scalar_param`'s unboxing fires only for
`Ptr(Int)` — so the pointer is formatted as a decimal integer.

So the divergence is real but it is the *silent* kind, which is strictly worse than the predicted
build failure, and the mitigation is the same either way: nothing in the tests or the example uses
`نص` on an array. Filed as **#331** and documented as a current limitation in `LANGUAGE_SPEC.md`
§8.6, with the note corrected in `docs/builtins-vs-stdlib.md` §6.2.

Method note, since this cost nothing to catch and would have shipped a wrong claim in the language
spec: **a source trace is a hypothesis about behaviour, not a record of it.** Running this took one
three-line file and three commands. Trace to form the hypothesis; run to write it down.

### The `Value::Null` arm was required, and #326's narrowing predicted it

#324 stated the rule broadly, #326 narrowed it to *pointer* parameters whose runtime guard is a
designed answer. This name is the first test of the narrowed form, and it lands on the **yes** side:
the parameter is `نص`, so an un-narrowed `نص?` lowers to `ptr null` and the runtime answers an empty
array.

With `Value::Null => Ok(Value::array())` in both interpreters, all three backends print `0` and exit
0. Without it they would abort where native succeeds — the exact shape #324 found.

One deviation from the plan, in the safe direction: the plan confined the `لا_شيء` contract to unit
tests, citing #327. But #327 is about **narrowed** optionals, and the un-narrowed shape was measured
to agree in all three backends, so it is covered cross-backend instead
(`test_string_to_bytes_accepts_an_absent_optional_in_every_backend`). The narrowed shape is still
untested and still #327.

### The keyword-embedding check gave a third distinct answer

`نص_إلى_ثنائي` is the first name in either family whose embedded keyword **opens** the name: `نص` is
`TokenKind::TypeString`. Every case already in `test_identifier_containing_a_keyword_stays_one_token`
has the keyword in suffix position (`بتات_و`, `بتات_نفي`) or mid-name (`بتات_أو_حصري`,
`بتات_إزاحة_يمين_منطقية`).

The leading position is its own shape because of how it would fail. A scan preferring the longest
keyword prefix would emit `TypeString` followed by the identifier `_إلى_ثنائي` — a *plausible* token
pair, since a type name followed by a name is ordinary syntax. It would not fail at the name; it
would fail somewhere later, with a message pointing at the wrong place.

The test was extended. Three consecutive names, three different answers: #317/#320 needed nothing,
#322 extended it for a keyword followed by a letter, #330 for a keyword in leading position. "The
last one needed nothing" remains worthless as evidence.

### The contract, and why the empty array is not a sentinel

`""` and `لا_شيء` both answer an **empty array**. Unlike `حرف_إلى_رمز`, which needed `-1` because
`0` is a real codepoint and could not distinguish "empty" from "the NUL character", there is nothing
here for a sentinel to disambiguate: a string with no bytes has exactly one encoding, and the empty
array *is* it. Raw null stays reserved for allocation failure, as in every other constructor in
`string.rs`.

Bytes are copied verbatim with no validation — `س` is UTF-8 by construction, and `ثنائي_إلى_نص` is
specified to round-trip arbitrary bytes, so rejecting anything here would break a round trip that is
meant to hold.

### Criterion (a) re-derived, and it held a third time

Encoding a string byte-by-byte in Tarqeem requires reaching the i-th character, and no
backend-portable way to do that exists: `قص_حروف` has no interpreter arm and no registered return
type (**B7**, open), `حرف_في` is native-only, `حرف_إلى_رمز` reads the first codepoint only, and
`س[i]` / `لكل ح في س` are still `Ptr(Void)` (**B6**). Even with the whole bitwise family landed the
operation is unreachable from source.

**B9** is now closed in one direction: a string's octets can be read, but not assembled back into a
string. That waits on `ثنائي_إلى_نص`.

### The review finding: an empty result reached a zero-capacity growth loop

Caught by code review, not by any test here, and it is the most serious thing in this change.

`helpers::allocate_array` sets `cap = len`, so `نص_إلى_ثنائي("")` returns an array with `cap == 0`.
`trq_array_ensure_capacity` grew by doubling from `current_cap`, and `0 * 2` is `0`, so the loop
never terminated. `الحق(نص_إلى_ثنائي("")، ٥)` printed `1` in both interpreters and **hung the native
binary indefinitely** — reproduced directly: SIGKILL after an 8-second budget, exit 137.

The path is new to this change. `trq_array_new` floors capacity at `ARRAY_INITIAL_CAP` and so never
produces `cap == 0`; the `[]`-literal route is blocked by a separate codegen type error; and
`ست_عشري_إلى_ثنائي`, the other `helpers::allocate_array` caller with an Arabic name, is not
importable. So an empty byte array was the first zero-capacity array a program could actually get.

Fixed in `trq_array_ensure_capacity` rather than in `trq_string_to_bytes`, because the empty array is
a *correct* return value and the defect is in the growth loop. That also closes it for
`ثنائي_إلى_نص` and for the `compress`/`crypto` callers of the same helper, which can all return
`cap == 0`. `realloc(NULL, n)` is well-defined as `malloc(n)`, and `old_size` stays derived from the
real `current_cap`, so the whole new buffer is zeroed.

**Two generalisable points.**

The first is about where a "return an empty collection instead of null" convention lands. Choosing
the empty array over a raw null was right for the contract, and it handed downstream code a value
whose *capacity* no previous array had. A convention chosen for the contract's sake can still be a
new input shape for every consumer of that type — enumerate the consumers, not just the callers.

The second is a CI gap this exposed. The fix's natural home for a regression test is
`runtime-rs/src/array.rs`, and **CI never runs those tests** — every CI `cargo test` is root-package
scoped, so a `runtime-rs` unit test is documentation, not a guard. The durable anchor is
`test_appending_to_an_empty_byte_array_grows_it_in_every_backend` in
`tests/builtins_execution_tests.rs`, which CI does run. Its own failure mode is worth stating in the
test, and is: on regression the native leg **hangs** rather than failing, so the signal is a stuck
job, not a red assertion.

### One claim in this change contradicted another

The spec and the example both said `نص_إلى_ثنائي` is «أول مدمجة تُرجع مصفوفة» while the section above
in this same file explains it is only a first for its *tier*. Both were written here, one PR, two
incompatible sentences — the careful version and the slogan. Narrowed to «أول مدمجة أساسية» with the
stdlib precedent named. Worth recording as a failure mode rather than a typo: the summary sentence is
where a qualified finding tends to lose its qualifier.

## #336 — قص_حروف, and a half-wired name that was worse than a missing one

Increment B's fifth and last name, and the only one of the five that already existed. `قص_حروف` was
declared in `get_stdlib_builtin("نص")`, mapped in `get_runtime_function_name`, declared in LLVM, and
implemented correctly in `runtime-rs` — with **no interpreter arm, no debug arm, and no registered IR
return type**. So it worked natively, aborted «دالة غير معرّفة» in the interpreter and the JIT, and
its native result carried the `Ptr(Void)` sentinel. Blocker **B7**, now closed.

### The cheaper half of the nine-site path

#324 measured the full path for a *new* symbol-mapped builtin: `runtime-rs` function, `Scope` entry,
return-type entry, `is_builtin` and a dispatch arm in both interpreters, an LLVM `declare` and a
`get_runtime_function_name` entry. This name needed only the second half — the semantic and
interpreter sites — because the first half already shipped.

That is not a special case. 216 names are already mapped in `get_runtime_function_name`, and every
`~` row in `docs/builtins-inventory.md` is exactly this shape: lowered natively, unregistered
everywhere else. **Repairing one costs six sites, not nine**, and the repair is what makes a name
work in the backend most users actually run.

### The missing return type is not "loud or quiet" — it depends on struct layout

#330 measured that a missing `register_builtin_return_types` entry for an **array** was caught by one
assertion out of four (`نوع`), and #333 measured three out of five for a **نص**. The natural
generalisation is a loud/quiet dichotomy by return type. It is wrong.

Measured here by deleting the entry and compiling: **four of five** caught it. `نوع` answered `مؤشر`,
`"X" + …` printed `X4341079168`, `== "رح"` answered `خطأ` — and `طول` answered **6 where 3 was
right**. Only `حرف_إلى_رمز` still agreed, by accident. The binary exited 0 throughout.

`طول` is the new one, and #333's `نص` did not have it. The sentinel routes `ArrayLen` to
`trq_array_len`, which reads `TrqArray.len` at offset 0; a `TrqString`'s field at offset 0 is its
**byte** length. The two layouts make that a clean misread rather than a crash, so the specific
failure of dropping this entry is that **the codepoint slicer silently starts counting bytes** — the
one thing the name exists not to do, and invisible on ASCII.

So the rule is not by return type. It is: **work out which assertion catches a missing entry from
what the sentinel's struct misreads**, and write that assertion. Printing still passed here, as it has
every time.

### Sharing the dispatch, not the kernel

#333 shared `bytes_to_string` — the decode — and let each interpreter keep its own argument checks,
which for a one-parameter builtin is nearly all of it. `قص_حروف` takes three parameters, and its
contract is mostly *in* the checks: which arguments are integers, and that exactly one of the three
gets a `Value::Null` arm. Duplicating that is duplicating the contract.

So `call_substring_by_chars` is `pub(crate)` and returns `RuntimeResult<Value>`, and each
interpreter's arm is one line. **Share at the widest point where the two backends must agree.** For a
kernel-shaped builtin that is the kernel; here it was the whole dispatch.

The `Value::Null` question resolved the way #326's rule predicts and #333's refinement demands:
the `نص` parameter is a pointer, `Type::compat` lets an un-narrowed `نص?` through, native lowers it
to `ptr null`, and the runtime guard answers `""` — so the arm mirrors a designed contract. The two
`عدد` parameters get no arm, because there native's `0` is #327's artifact.

### Criterion (a) expired, and the two halves of the claim came apart

§1.3 justified this name as "the only way self-hosted Tarqeem can reach the i-th character at all —
`س[i]` and `لكل ح في س` both yield an untyped `Ptr(Void)`". Re-derived before implementation, as
§6.1 requires:

- The **first** half expired. `نص_إلى_ثنائي` (#330) and `ثنائي_إلى_نص` (#333), with indexing over
  `مصفوفة<عدد>` and the bitwise family, make a codepoint slicer writable in Tarqeem. Probed before
  writing any fixture: a hand-written slicer agrees with the builtin in all three backends on every
  case, out of range included, and that probe is now
  `test_substr_chars_matches_the_slicer_it_names`.
- The **second** half did not. **B6** is still open; `س[i]` is still `Ptr(Void)`.

**Three of Increment B's five held** — #324, #326 and #330 — and two expired, #333 and this one.
Doc-wide it is the fourth expiry, after `بتات_نفي` (#312) and `بتات_إزاحة_يمين_منطقية` (#322).
Worth separating from the count: the two halves of a single justification can expire at different times, and here the operation
became expressible while the idiomatic route to it stayed broken. Reading one as evidence for the
other would have retired **B6** by mistake.

It shipped anyway on `بتات_نفي`'s grounds — core tier, and §5.2 keeps a no-import name a builtin
until **B12** — plus one earlier expiries did not have: this was a **repair**, so the alternative to
shipping was not "leave it in stdlib" but "leave it half-wired".

### Removing قص_نص fixed the document's own example

`قص_نص`, the byte-indexed slicer, was removed in the same change — an owner decision that deviates
from §1.3's "both names survive" and §1.1 rule 3's one release of `م`-warnings, and is recorded as a
deviation in the plan document rather than argued.

What made it worth noting: `stdlib/نص/اساسي.ترقيم:22` declared a parameter named `عدد_احرف` and passed
it to the *byte* slicer, so `قص("مرحباً بالعالم"، ٠، ٦)` returned three Arabic characters. §1.3 cites
that exact line as its motivating example of the byte/char trap. Removing `قص_نص` left the line
nothing to call but `قص_حروف`, so the argument for a uniformly codepoint-indexed primitive surface
carried itself out instead of being restated. **A removal can be a repair when the only remaining
callee is the correct one** — worth looking for, because it costs nothing and the alternative is a
deprecation window during which the wrong behaviour stays checked in.

`trq_string_substr` went with the name: `rg` found exactly two references under `src/`, the `declare`
and the map, and no operator path. `trq_string_substr_chars` stays regardless of any Arabic name,
because `trq_string_char_at` calls it and codegen emits *that* for the `س[i]` operator — standing rule
3 applied in both directions in one change.

### `ك` is unusable as a loop variable, and the error does not say so

The hand-written slicer's inner loop was first written with `ك` as the counter. It fails at parse:
«ب٠٢٠١: متوقع اسم المتغير», caret on the `=`. `ك` is the contextual alias keyword, and the diagnostic
names neither the keyword nor the alias.

§6.6 already warns about this for Increment F. It fired first here, in a **test fixture** — which is
where it will keep firing, since fixtures are where short identifiers get used. The warning belongs
next to the diagnostic, not only in the increment that expects it.


## #338 — متغير_بيئة, and the first criterion that could not expire

`متغير_بيئة` — `(نص) -> نص`, `getenv(3)` — is the Category 8 environment reader from the 40-name
registry, and the first name outside Increments A and B. Core tier, no import.

### A (b) criterion is a different kind of claim from an (a) criterion

§6.1 made re-derivation a standing rule because four §1.3 rows had criterion (a) expire under them —
`بتات_نفي`, `بتات_إزاحة_يمين_منطقية`, `ثنائي_إلى_نص`, `قص_حروف`. This is the first name checked whose
criterion is **(b)**, and the re-derivation is one sentence: nothing in Tarqeem reads the process
environment, and nothing that lands later can change that, because the capability lives in the
operating system rather than in the language.

**Criterion (a) is a statement about the language, which every increment edits. Criterion (b) is a
statement about the kernel, which no increment can reach.** So the standing rule should be read as
applying to the (a) rows; for the (b) rows what needs checking is the *contract*, which is the second
defect class #333 identified.

That check paid off, though not by finding a defect. `trq_env_get` was read rather than trusted,
because this document's other orphan precedent is `trq_performance_now` — implemented, linkable, and a
verbatim copy of `trq_time_now`, so it lies about being monotonic. `trq_env_get` is honest: all five of
its paths (null pointer, null data, empty name, invalid UTF-8, unset variable) already return
`trq_string_new(null, 0)` — an empty `TrqString`, not a null pointer — so §1.3's `""`-when-unset clause
was satisfied by code predating the row. **Read the orphan. The two this document leans on disagree
about whether they work.**

### The fourth cost shape, and what actually discriminates them

Eight sites, not nine and not six: everything on the path except the `runtime-rs` function, which
already existed. The four measured shapes are now

| Shape | Cost | Example |
|---|---|---|
| IR-intercepted | 2 files | `بتات_و` (#302) |
| Symbol-mapped, new runtime function | 9 sites | `حرف_إلى_رمز` (#324) |
| Symbol-mapped, symbol already exists | **8 sites** | `متغير_بيئة` (#338) |
| Repair of a half-wired name | 6 sites | `قص_حروف` (#336) |

The discriminator is not the tier, and not the return type — #330 already showed a "first" that was
only a first for its tier. **It is which half of the path already exists.** Here the runtime half did
and the codegen half did not, which is the mirror image of #336.

### The missing return type: predicted from the struct layout, and the prediction held

#336 asked for this to be predicted from the return type's layout rather than sorted into loud or
quiet, and doing that in advance produced the right answer. Measured natively with the
`register_builtin_return_types` line deleted, for the value «مرحبا»:

| Assertion | Without the entry | Right |
|---|---|---|
| `اطبع(…)` | `مرحبا` | `مرحبا` — **passes either way** |
| `نوع(…)` | `مؤشر` | `نص` |
| `"X" + …` | `X4321175728` | `Xمرحبا` |
| `== "مرحبا"` | `خطأ` | `صحيح` |
| `طول(…)` | **10** | **5** |

Four of five, `قص_حروف`'s profile rather than `ثنائي_إلى_نص`'s three. `طول` catches it for the same
reason it did there: the sentinel routes `ArrayLen` to `trq_array_len`, which reads offset 0, and a
`TrqString`'s field at offset 0 is its byte length.

**The Arabic test value is load-bearing, and this is the generalisable part.** On an ASCII value the
byte count and the character count agree, so `طول` would pass with the entry deleted and the gate
would be a three-catcher instead of a four. A `نص`-returning builtin whose tests use only ASCII
silently gives up one of its assertions.

### A cross-backend harness cannot set an environment variable in-process

`متغير_بيئة` is the first builtin whose answer depends on the environment, and `std::env::set_var` is
unusable in the harness: cargo runs tests as threads in one process, so setting a variable races every
other test.

Every backend leg was already a child process — `tarqeem run`, `tarqeem run --jit`, and the compiled
binary — so the variable goes on the child. `tarqeem_with_env`, `execute_with_env` and
`assert_prints_with_env` were added, and the three existing helpers became one-line wrappers over them,
so all 147 existing call sites are untouched. One detail that is easy to get wrong: the native leg must
put the variables on the **executed binary**, not on `compile` — the compiler reads no environment on
that path, so setting them there compiles fine and then answers `""`.

The absent-variable cases need none of this and are covered by plain `assert_prints`; only the
exact-value, set-but-empty, and no-trimming cases need injection. **Set-but-empty is reachable no other
way**, which is why it is a separate test rather than a row in the totality test.

### Two smaller findings

**The name must be read raw.** `trq_env_get` deliberately does its own null/len/UTF-8 checks instead of
going through `string.rs`'s `as_str`, which trims — the trap #324 recorded. An interpreter arm using a
trimming accessor would answer «مرحبا» for `متغير_بيئة(" PATH ")` where native answers `""`, on source
that reads like a typo rather than a bug. `test_env_var_does_not_trim_the_name` pins both halves.

**The lexer check found a ninth shape, and it is the first whose failure would not look like a lexer
failure.** `متغير_بيئة` opens with `متغير` (`TokenKind::Let`). Position-wise that matches
`نص_إلى_ثنائي` (#330), but `نص` is a type name already legal as an identifier, while `متغير` opens a
*statement*: a longest-keyword-prefix scan would emit `Let` then `_بيئة`, which is a **well-formed
variable declaration**, so it would surface as a missing `=` somewhere unrelated — or not surface. Nine
names, nine shapes; the check stays per-name, and this is the fifth consecutive name where "the last
one needed nothing" would have been the wrong inference.

### Example collision worth knowing about

`examples/مدمجات.ترقيم` is now 990 lines with twelve builtin sections, and top-level names are shared
across all of them: `متغير غائب: نص? = لا_شيء` already existed at line 560 from the `حرف_إلى_رمز`
section, so re-declaring it failed with `د٠١٠١`. Renamed to `اسم_غائب`. As that file grows, a new
section's locals need a section-specific prefix — the failure is loud, but it is not obvious from
inside the section being written.

---

## #342 — `أنهِ_البرنامج`: terminate with an explicit exit status

Category 6 of the primitive registry, criterion (b). Landed ahead of Increment G because `exit(2)`
composes with nothing — it needs none of that increment's syscall primitives. The full record is in
`docs/builtins-vs-stdlib.md` §6.7.1; what follows is the part that changed how the *next* name should
be approached.

### A `فراغ` primitive must NOT register its return type — the standing rule is wrong for it

`docs/builtins-vs-stdlib.md` §1.1 rule 5 says a primitive needs a `Scope` entry **and** a
`register_builtin_return_types` entry **and** interpreter arms, and calls any two of the three a
landmine. That rule protects a *value*: unregistered, a call carries the `Ptr(Void)` sentinel and
something downstream misreads it. `أنهِ_البرنامج` returns nothing, and both halves were measured
rather than assumed:

- **The entry buys nothing observable.** Unregistered, codegen emits
  `%v3 = call ptr @trq_exit(i64 %v2)` beside `declare void @trq_exit(i64)` — and **clang accepts
  it**. Under opaque pointers a direct call carries its own function type, so a signature mismatch
  is no longer a parse error. Same stdout, same status, both ways. The prediction going in was a
  loud `جذر`-style refusal, and it was wrong: `جذر` is loud because `اطبع` *dereferences* its
  result. **Predict this failure mode from the use site, never from the declare.**
- **The entry costs cross-backend agreement.** With it, `متغير س = أنهِ_البرنامج(٣)` fails native
  compilation (`ت٠٠٠١: متغير غير معروف: %3`) while both interpreters exit 3. Codegen's `is_void`
  branch emits the call and creates no value for `dest`, while the IR still references that `dest`
  downstream. Unregistered, all three backends agree on every shape probed.

So the name ships with three of the four sites, and the omission is documented *where the entry would
have gone* — silence there would read as an oversight to the next person, since the rule tells them to
add one. Filed the underlying defect as **#343**, which reproduces with no builtin involved: a plain
`دالة ف() { }` with `متغير س = ف()` runs interpreted and fails native compilation, because a user
function's missing return type *is* an `IrType::Void`. Add the entry once #343 lands.

Generalisable: this is the **third** defect class this project has found in its own design rows, after
expiring criterion-(a) claims (#312, #322, #333, #336) and unimplementable contracts (#333). A rule can
be right about the mechanism it was written for and wrong about one that had not appeared yet.

### The interpreter cannot honour an arbitrary status by itself

`src/main.rs` maps every `Err` to status 1, and the interpreter runs in-process, so the status travels
as `ErrorKind::ProgramExit(i32)` and is honoured in `src/cli/commands/mod.rs` — at three sites
(interpreter, JIT, REPL), and **before** the «Runtime error» report in each. Order matters more than it
looks: reporting first both loses the status and prints to stderr where the native binary prints
nothing, and `compare-backends` diffs stdout only, so CI would not have caught it. The execution helper
asserts empty stderr for that reason.

`process::exit` inside the builtin arm was the obvious alternative and was rejected twice over: it
would end the test binary for any in-process debug-interpreter assertion, and it would let a builtin
terminate a host process it does not own (the REPL, the DAP server). `توقف` gets away with an `Err`
because its status is always 1 — which the error path already produces — so it is not a template for a
status the program chooses.

Uncatchability came free: `take_propagating_exception` routes only `ErrorKind::UnhandledException` to a
frame's `try_stack`, so an exit signal walks past every `حاول`. Asserted anyway — it is one `matches!`
away from `التقط` swallowing an exit interpreted while native still terminated.

### The composition gate inverts, and the first attempt at one was confounded

Every primitive since #324 has been gated on *composing* its result, because printing a sentinel-typed
result passes while concatenating it is silently wrong. A `فراغ` name has no result, and the natural
substitute — "assert that using it as a value is rejected" — failed twice: the call exits before
anything can be observed, and the analyzer does not reject a `فراغ` result bound to a variable at all
(#343). The replacement asserts a **non-zero** status through a bound call, so only the call actually
running can produce the answer. Choose the assertion so that exactly one behaviour produces it.

### Two spellings, one primitive

`أنه_البرنامج` is registered alongside `أنهِ_البرنامج` (owner's decision). The kasra marks the dropped
ya of the imperative and Arabic writers routinely omit it, which is why the **keyword** table already
pairs `ارمِ`/`ارم`, `أرجع`/`ارجع` and four more; `normalize_name` is NFC only and does not strip
tashkeel, so one entry cannot serve both. Consequence to carry: the registry's *name* count (33 core)
and its *capability* budget (40 primitives) now differ by one on purpose. Both are recorded in the
guard test and in §1.3 so a later increment does not "fix" the difference.

### The lexer check found a tenth shape — a diacritic, not a keyword

The name embeds no keyword (checked against the full list), so
`test_identifier_containing_a_keyword_stays_one_token` was deliberately left alone. But it is the first
builtin name carrying a **diacritic**, and the position is what matters: the kasra sits between a letter
and the `_`, so a scan ending an identifier at any non-letter would yield `أنه` — a perfectly good
identifier one invisible codepoint short of the right one, failing later as an undefined function.
`test_identifier_with_a_diacritic_stays_one_token` pins that, and pins that the two spellings are
distinct tokens, which is *why* both are registered.

### The example can only demonstrate status ٠

Every job in `.github/workflows/examples.yml` fails on a non-zero exit — `expected-output`, the three
`run-*` matrices and `compare-backends` alike — so `examples/مدمجات.ترقيم` calls `أنهِ_البرنامج(٠)` and
the non-zero half lives in the unit tests. The unreachable `اطبع` placed after the call earns its keep:
its absence from the committed `examples/متوقع/مدمجات.خرج` is the truncation proof. That section must
stay **last** in the file, and says so — anything appended after it would never run, and the expected
output would look correct.

## #347 — `اكتب_مجرى`: `write(2)`, and the first cost estimate that transferred whole

Category 7 of the primitive registry (`docs/builtins-vs-stdlib.md` §1.3), criterion (b), and the
first of Increment G's seven I/O primitives. Before it the language had no byte-level output at
all: `اطبع` and `اطبع_خطأ` are compiler intrinsics whose lowering picks a print symbol off the
static `IrType`, and §9.1 records why they can never be anything else — a polymorphic print needs
an `أي` parameter, which native codegen refuses with `ت٠٣٠١`.

### The cost shape was predicted correctly, and that is the result

Five shapes had been measured before this: 2 files for an IR-intercepted name, 9 sites for a new
symbol under a new name (#324), 8 when the symbol already existed (#338), 6 to repair a half-wired
one (#336), and 11 for a `فراغ` effect (#342). §6.7 named the discriminator — *which half of the
path already exists* — and #342 added the caveat that it does not cover a **new kind of effect**.

Applied here in advance: neither half existed, and writing bytes to a stream is not a new effect
(`trq_print` has always done it). Predicted nine, cost nine, plus the one-line B15 fix this
primitive's own contract requires.

**Not the first estimate that held — the first that was *forecast*.** #320 and #326 each cost what
their predecessors cost and found nothing new about the path, and both recorded that as the result.
Neither was a prediction: the discriminator was only named at #338, so those two agreed with the
estimate in retrospect. Here the number was written down before the work, and the work agreed with
it. Worth separating, because "the estimate held" is only informative when the estimate came
first — and because claiming a first the docs already record is exactly the kind of error the next
increment would have to correct.

### A scalar's missing return type cannot be assembled, let alone misread

The load-bearing measurement, taken by deleting the `register_builtin_return_types` entry and
recompiling, as #330/#333/#336/#338 each did. The progression so far had been read as a loudness
gradient: one caught assertion for an array, three for a `نص`, four for `قص_حروف` and
`متغير_بيئة`. This name does not sit on that scale.

- `اكتب_مجرى(١، []) == ٠` and `... + ١` **fail native compilation** — `ت٠١٠١`, clang:
  «'%v13' defined with type 'i64' but expected 'ptr'». A scalar return has no struct for the
  `Ptr(Void)` sentinel to misread; `icmp`/`add` on a `ptr` is simply not valid IR, so the module is
  never assembled.
- `نوع` answers `مؤشر`, as it has for every name.
- `اطبع` is **quieter than in any previous name**: it prints *nothing at all* for the count, taking
  the pointer path. `ثنائي_إلى_نص` at least printed a pointer in decimal; `قص_حروف` printed a wrong
  length. Here the value vanishes.

So #336's "predict from the struct layout" generalises one level up: predict from the return type's
**representation**. A pointer-shaped return degrades silently and needs the composition test; a
scalar one fails the build on any arithmetic and cannot be printed wrong because it cannot be
printed at all. Two names, two opposite ends, and across the five names where the entry has
been deleted and measured — #330, #333, #336, #338 and this one — printing has caught it zero
times.

### The withdrawn clause is a third defect class for §1.3 rows

The row promised "returns bytes written so short writes stay visible". Not unimplementable the way
#333's no-validation clause was — **unreachable**. `write_all` loops until the payload is out or an
error stops it, so a short write is never in a state that could be reported; the honest answer is
the full count or `-١`. Reporting partial progress would mean a single `write` returning `n`, which
silently truncates a large payload and moves the loop into every caller.

After expiring criterion-(a) claims (#312, #322, #333, #336) and contracts no implementation can
satisfy (#333), that is a third class: **a clause that is implementable, satisfiable, and describes
a state the operation cannot enter.** Check a row's promises against the shape of the call, not only
against the language and the value representation.

### The type-confusion guard is the range check, and it was free

A `TrqArray` carries no element-kind tag — `مصفوفة<عدد>` and `مصفوفة<نص>` are both `elem_size == 8`
and indistinguishable at runtime (`runtime-rs/src/types.rs:94-151`) — so an `أي` holder can land a
string array, or a `TrqString` itself, on the byte parameter. Nothing new was needed for either:

- A `مصفوفة<نص>` element read as an `i64` is a pointer value, far outside `٠`-`٢٥٥`, so the
  byte-range rejection already refuses it. The check written for `[٣٠٠]` covers *that* type
  confusion for free.
- A `TrqString` is refused on `elem_size` **before `data` is read**, the order
  `trq_string_from_bytes` established and for its reason: the string is 24 bytes, `elem_size` sits
  at offset 16 inside it, and `data` sits at offset 24 — one past the end. Reversing the two checks
  is a heap over-read, not a wrong answer.

**But "covers type confusion for free" is not general, and the exception is measured.** A
`مصفوفة<عدد_عشري>` is also `elem_size == 8`, and its slots are IEEE-754 bit patterns — so an
element whose pattern happens to land in `٠`-`٢٥٥` passes the range check. `٠.٠` is exactly that:
its pattern is all zeroes, so it reads as the byte `٠`.

```tarqeem
متغير ف: أي = [0.0]
اطبع(اكتب_مجرى(١، ف))   // المفسّر: -1 — والترجمة الأصلية: 1، وتكتب بايت NUL
```

The interpreter answers `-١` because `Value::as_int` is strict on `Value::Float`; native answers `١`
and puts a NUL byte on the stream. **This is not new and not specific to this primitive** — the same
source through `ثنائي_إلى_نص` answers `٠` interpreted and `١` natively, so the hole belongs to every
`مصفوفة<عدد>` parameter reached through an `أي` holder, and predates #347. It is recorded here
because this bullet is where a later increment would look for the guarantee and find it overstated.

Fixing it in the runtime is not possible — there is no element-kind tag to read, which is the
premise of this whole subsection. The fix belongs in the semantic layer: widening `أي` to
`مصفوفة<عدد>` is what makes the confusion reachable at all, and narrowing that widening would close
it for every name at once rather than one primitive at a time. Until then, treat the range check as
refusing *pointers*, not as refusing *non-`عدد` arrays*.

Rejection is total for a second reason worth separating from correctness: the array is validated
**before the first byte goes out**, so `[٦٥، ٣٠٠]` writes nothing rather than writing `A` and then
failing. A partial write with a `-١` answer would be unrecoverable — the caller cannot know how much
landed.

### Truncation was rejected again, on #333's grounds

`[٣٠٠]` answers `-١`, not the comma. Truncating to the low byte would make it indistinguishable
from `[٤٤]`, so a rejected array and an accepted one would produce identical output and there would
be no way to tell them apart. Same reasoning as `ثنائي_إلى_نص`, and it is the second name to face
it — the house convention `trq_sha256_bytes` still follows (truncate) is the one being displaced.

### The interpreter and the runtime agree on descriptor `٣`+ for a reason, not by construction

Both answer `-١`. The runtime looks the descriptor up in `FILE_HANDLES` and finds nothing; the
interpreter has no table at all. They agree because **nothing in the language opens a handle yet** —
the streaming API in `io.rs` is orphaned and no Arabic name maps to it. That agreement is
load-bearing and temporary: `افتح_ملف` must give the interpreter a handle table in the same
increment it lands, or the two diverge the moment a handle exists. The runtime's handle path is
implemented and unit-tested now (`trq_file_open_write` → `trq_write_stream` → `trq_file_close`), so
the contract will not shift under the opener when it arrives.

### B15 was fixed here because this is the change that made it reachable

`NEXT_FILE_HANDLE` moved from 1 to 3. The blocker was filed as "collides with stdout once streams
unify", and this is the unification: descriptor `١` now means stdout, so a handle numbered 1 would
have sent a file write to the terminal — silently, since both succeed. Nothing depended on the old
numbering: `0` was never a valid handle (every `trq_file_open_*` returns it on failure) and every
existing test asserts `handle > 0` rather than `== 1`. A new assertion pins `handle >= 3`.

### Raw bytes constrain the CI example, not just the tests

Bytes that are not valid UTF-8 reach stdout intact — that is the primitive's point, and it is what
no print builtin can do (`trq_print` is `if let Ok(text) = from_utf8`, so a lone `٢٥٥` prints
nothing with no error). Two consequences for the example:

- Writing `٢٥٥` there would commit a golden file that is not text.
- `scripts/جدد_المتوقع.sh` captures `2>&1`, so a descriptor-`٢` write in the example would make the
  committed output depend on stdout/stderr interleaving.

Both rows are covered in tests that read the streams apart instead. The rule for the rest of
Increment G: **the example demonstrates the contract's text rows; its byte and stream rows belong
where the streams can be read separately.**

One test bug is worth recording because it is how the property was found: the first draft asserted
`[٢٥٥]` through stdout and failed with `left: ["...", "\u{FFFD}1"]`. The primitive was correct; the
test was comparing bytes as text.

### The lexer check found no new shape, and was run anyway

`اكتب_مجرى` embeds `ك` — `TokenKind::As`, the alias specifier — inside `اكتب`, with a letter on each
side. That is the same shape `قص_حروف`'s `و` has. It was added to
`test_identifier_containing_a_keyword_stays_one_token` regardless, because five of the nine cases
already there were added for a shape their predecessors did not cover — mid-name (#309), keyword
followed by a letter (#322), keyword opening the name (#330), a keyword inside a word with letters
on both sides (#336), and a *statement* keyword opening it (#338) — so "the last one needed nothing"
has been worthless as evidence every time it was tried. `ك` is also the shortest entry in the keyword
table, which makes it the likeliest to fall inside an ordinary Arabic word by accident.

### Smaller findings

- **B14 fired immediately, and the plan's discriminator identified it.** The first native attempt
  failed with `ld: symbol(s) not found`, not a clang parse error — a stale `libtrq.a`, because
  `cargo build --release` alone does not rebuild the runtime crate. `--workspace` does. Also worth
  knowing: `nm -g` reports nothing for this archive on macOS (it finds `trq_env_get` zero times
  too), so it is useless as a presence check — `strings` works.
- **`اكتب_مجرى` does not participate in output capture.** The main interpreter's `capture_output`
  and the debug interpreter's `context.add_output` both mirror `اطبع`; this writes straight to the
  process stream in both. Deliberate: the descriptor names the *process's* stream, so interposing a
  host buffer would change what the program observably did. The cost is that a DAP console does not
  mirror these bytes, which is recorded rather than worked around because the debug output path
  needs its own pass either way (#346).
- **A failed flush answers `-١`, and the module's convention was the wrong one to inherit.** Every
  `trq_print*` here discards the flush result, and the first draft copied that. But those functions
  **return nothing** — they have no answer to falsify. `Stdout` is line-buffered, so a payload with
  no trailing newline sits in the buffer and a closed pipe fails at the *flush*, not at the
  `write_all`: reporting the count there claims bytes reached the descriptor when none did. Changed
  in both the runtime and the interpreter together, since a split would make the two disagree about
  a closed pipe. Generalisable: **a convention adopted from void functions does not transfer to one
  that returns a count.** The handle path still does not flush, matching `trq_file_write_line` — a
  `BufWriter` exists to batch and `trq_file_flush` is how a caller asks — so the count means
  "accepted by the stream" for a handle and "left for the descriptor" for a console stream.
- **No `Value::Null` arm for the descriptor**, and one for the array. #326's narrowing predicted
  both: the descriptor is an `عدد` with no pointer for a runtime guard to answer, so `لا_شيء` is a
  type error; the array is a pointer whose null answer is designed, so it answers `٠`.

---

## #350 — `اقرأ_مجرى`: `read(2)`, and what a missing return type actually costs

`اقرأ_مجرى` — `(عدد، عدد) -> مصفوفة<عدد>` — is the second Increment G primitive and the read half of
the byte-level stream pair `اكتب_مجرى` (#347) opened. Contract, scope and precedents are in
`docs/builtins-vs-stdlib.md` §1.3 (category 7), its correction blockquote, and §6.7.3.

### The cost forecast held, for the second consecutive name

§6.7's discriminator — *which half of the path already exists* — was applied before the work:
neither half did, so the #324 nine. It cost nine, plus the harness change the contract requires.
#342's caveat was checked and does not apply, because reading bytes from a stream is not a new kind
of effect: `trq_input` has always done it. Two forecasts, two hits, so the discriminator is now worth
trusting rather than re-deriving each time.

### The measurement that contradicted the plan

The plan predicted a **quiet** missing `register_builtin_return_types` entry, on #330's finding for
`نص_إلى_ثنائي` — "only `نوع` catches it". Measured by deleting the entry and running seven use sites
across all three backends, that is wrong, and the shape of the wrongness is the useful part:

| use | interpreters | native |
|---|---|---|
| `اطبع(بايتات)` | correct | prints **nothing** — silent wrong output |
| `طول(بايتات)` | correct | correct (`ArrayLen` → `trq_array_len` regardless) |
| `ثنائي_إلى_نص(بايتات)` | correct | correct (a `ptr` parameter takes the sentinel unchanged) |
| `نوع(بايتات)` | `مؤشر` | `مؤشر` |
| `اطبع(بايتات[٠])` | correct | **run-time abort** — «misaligned pointer dereference … 0x41» |
| `بايتات[٠] + ١` | correct | **compile failure**, ت٠١٠١ |
| `بايتات[٣] == ٦٨` | correct | **compile failure**, ت٠١٠١ |

Three modes at once — silent, fatal at run time, fatal at build time — for one return type. The abort
is the instructive row: with `Ptr(Void)` the *element* is a pointer too, so `trq_print` dereferences
the byte value `65` as an address.

**So the loudness ranking this file has been building since #330 is the wrong abstraction.** One
catcher for an array, three for a `نص`, four for `قص_حروف` and `متغير_بيئة`, fatal for a scalar —
each of those was a real measurement, but the quantity does not belong to the return type. It belongs
to the **use site**, which is what `builtins-vs-stdlib.md` §1.1's own note says and what the ranking
kept obscuring. Two names with the *same* return type disagree, because #330's array was only counted
and printed while this one's elements are indexed and added.

Practical consequence for the next primitive: do not ask "how loud is this return type". Ask which of
the caller's operations *cannot be assembled* if the result is a pointer. Those are the assertions to
gate on.

### The composition gate has a trap when the empty answer is a contract row

Every primitive since #324 is gated on composing its result. The convenient fixture here is a
descriptor the primitive refuses — it needs no stdin, so it is far easier to write. It is also
worthless: an empty array cannot be indexed and `طول` answers `0` either way, so all three assertions
pass on a sentinel. The gate has to run over bytes actually read, which is what forced the harness
change to land first rather than beside the tests.

Generalises to any primitive whose refusal answer is a *legitimate value* of the return type.

### The harness gained stdin, and the default turned out to be a contract row

`cargo` runs tests as threads in one process, so a test can no more redirect its own stdin than it can
`set_var` (#338). All three backend legs are child processes, so the bytes go on the child: one shared
innermost driver plus `_with_stdin` peers, existing call sites untouched. Three things worth keeping:

- The parameter is `&[u8]`, not `&str` — one contract row is a byte sequence that is not text.
- `Command::output`'s default stdin is **null**, not inherited. So the EOF row is assertable through
  the plain `assert_prints` with no piping at all, which is why there is no `_with_stdin` variant of
  the empty-stream test.
- The native leg pipes to the **executed binary**, never to `compile` — #338's environment lesson
  transposed unchanged.

A speculative `tarqeem_with_stdin` wrapper was written and then deleted: nothing called it, and a
dead helper is a warning the crate did not have before.

### An input primitive's CI example is worse off than an output one's

#347 found that an output primitive's *byte* rows cannot go in the example, because the golden file is
a `2>&1` capture. The inverse holds here and bites harder: the golden is generated with stdin inherited
from a terminal, so any positive-count read on descriptor `٠` waits for input and never finishes. The
example therefore demonstrates only refusals — every row it covers answers zero or `[]`.

The general rule for the rest of Increment G: **an example can only exercise a primitive whose inputs
the example itself can supply.** `افتح_ملف` and `حالة_ملف` will be able to; this one cannot.
(#352 renamed that name to `حالة_مسار` and found the prediction half right — see below.)

One thing that *did* become available: `اطبع` on an empty array was probed across all three backends
before the fixtures were written (#333 finding 3's habit) and all three print `[]`. No committed
example printed an empty array before, so the avoidance in the `نص_إلى_ثنائي` section turns out to
have been caution rather than a known divergence.

### Smaller findings

- **The keyword-embedding check was run mechanically for the first time**, against all 69 keywords
  harvested from `src/lexer/keywords.rs` rather than by eye. `اقرأ_مجرى` embeds none; its sibling
  `اكتب_مجرى` embeds `ك`. Per #317/#320 it therefore gets **no** row in
  `test_identifier_containing_a_keyword_stays_one_token`. A generic "add-a-builtin" checklist calls
  that site mandatory; it generalises from #347, whose name does embed a keyword, and the precedent
  wins.
- **A new lexer shape was probed and passed.** The name carries a precomposed hamza (`أ`, U+0623)
  whose NFD form is two codepoints, so source written decomposed must still resolve. It does — the
  lexer normalises the file to NFC before tokenising. Not pinned in a test, because the
  normalisation is a whole-file property rather than anything about this name.
- **No `Value::Null` arm anywhere**, the first primitive since #324 with none: both parameters are
  `عدد`, so there is no pointer for a runtime guard to answer and codegen turns `لا_شيء` into `0`
  above the runtime (#326, #327). The debug-interpreter test asserts a `TypeError` for both
  positions, so nobody adds one by pattern-matching from `اكتب_مجرى`'s array parameter.
- **The `≥٣` note from §6.7.2 now covers both halves of the pair.** The interpreter has no handle
  table and the runtime's is provably empty from Tarqeem source, so both answer empty for the same
  reason. `افتح_ملف` must give the interpreter a handle table in the increment it lands, or two
  primitives diverge at once. The runtime's handle path is implemented and unit-tested now
  (`trq_file_open_read` → `trq_read_stream` → `trq_file_close`, plus a writer, a closed handle and a
  >64 KiB file that makes the read loop run more than once).
- **`runtime-rs`'s export count in `docs/builtins-inventory.md` was two low before this change.**
  Recounted to 223 from source. The row says recount rather than increment; it earned that wording
  again.

---

## #352 — `حالة_مسار`: `stat(2)`, and a fold claim that needed a fourth value

**Increment G's third primitive** (`docs/builtins-vs-stdlib.md` §6.7.4), after `اكتب_مجرى` (#347) and
`اقرأ_مجرى` (#350). `(نص، عدد) -> عدد`: `حقل ٠` answers what is at a path — `٠` absent, `١` file,
`٢` directory, `٣` exists and is neither — and `حقل ١` the byte length of a regular file. Anything
else answers `-١`.

### Two decisions taken before the work

- **`حالة_مسار`, not the registry's `حالة_ملف`.** The operation reports on a path, which may hold a
  file, a directory or neither, and a directory is not a `ملف`. Its own category-7 sibling
  `احذف_مسار` already uses `مسار` for the identical scope. Recorded as a §1.3 correction on the #302
  precedent rather than changed silently.
- **A directory answers `-١` for its size.** `trq_file_size` answers the OS `st_size` there — 4096 on
  ext4, 64–96 on APFS — and a number that changes with the filesystem cannot be asserted in a test or
  a golden file. So size is the byte length of a regular file and `-١` for everything else. The future
  `حجم_ملف` wrapper inherits the delta.

### What it found

- **A fold claim needs enough *range*, and this row did not have it — a fifth §1.3 defect class.**
  The row promised to fold four names with three kind values. It cannot: `ملف_موجود` is
  `Path::exists()` and answers **true** for `/dev/null` while `هل_ملف` answers false for the same
  path. Hence `٣`, and hence `ملف_موجود` reduces to `!= ٠` rather than `== ١`. The check that found it
  was to read each folded name's implementation one at a time — the other three map onto
  `١`/`٢`/size directly, so a plausible reading of the row would have missed exactly the one that
  breaks it.
- **A scalar's missing-return-type mode is predictable across names; an array's is not.** #347's
  measurement for `اكتب_مجرى` transferred here exactly — `اطبع` prints nothing and exits 0, `نوع`
  answers `مؤشر`, and `+ ١` / `== ٢` fail native compilation with ت٠١٠١ — where #330's array
  measurement did not survive #350's second array. A small new detail: the two compile failures
  report the mismatch in **opposite directions**, because in the comparison the typed operand is the
  literal.
- **A *contextually* reserved keyword needs a parser check, not only the lexer row.** `حالة_مسار`
  opens with `حالة` = `TokenKind::Case`, reserved in exactly one construct. The mechanical sweep over
  all 69 keywords found `حالة` and nothing else; the lexer row proves the name stays one token, and
  `test_path_status_is_callable_inside_a_match` proves the parser accepts it inside `تطابق`, in the
  scrutinee and in an arm body. Both passed — no defect to file — but the next contextual keyword
  (`احصل`, `عيّن`, `ك`) needs the same second half.
- **The example's input capability splits along invariance, not supply.** #347 could not put byte rows
  in the example (the golden is a `2>&1` capture) and #350 could not put success rows there (the
  golden is generated with stdin inherited). Here `"."` and an absent path work perfectly and a
  *regular file* does not — nothing in the language creates one, and a relative repo path would make
  the golden depend on the working directory. `/dev/null` is out for a third reason: Unix-only, and
  the golden is regenerated on a developer machine. So the rule is **rows whose inputs are invariant
  under where and on what the program runs**, which is narrower than "inputs the example can supply".
- **The first primitive whose kernel is duplicated across the crate boundary.** The kind/size mapping
  exists in `trq_path_status` and in `call_path_status`, because the root crate does not depend on
  `tarqeem-runtime` and an `extern "C"` function taking a `*const TrqString` could not read a `Value`.
  Two copies by construction; what holds them together is that every row × both fields is asserted
  cross-backend, not in either implementation's own unit tests. #336's share-the-dispatch rule still
  applies *within* the compiler — one `pub(crate)` dispatch for both interpreters — which keeps it at
  two copies rather than three.
- **Landed ahead of `افتح_ملف` on purpose.** The opener needs an interpreter handle table in the same
  change, which is two primitives' work under a one-per-change rule. The remaining Increment G names
  are not equally sized, and the path-taking ones can land alone.

### Cost

**Nine registration sites**, forecast from §6.7's discriminator before the work — neither half of the
path existed — and the third consecutive forecast to hit. Fourteen files, which is #350's seventeen
minus the four docs plus the lexer test. One additive harness helper (`assert_prints_with_files`,
absolute fixture paths, because the native leg inherits no working directory), following the
`_with_env` / `_with_stdin` precedent so all existing call sites stayed untouched.

**Nothing removed.** The four folded names are `مكتبة`, not `يُحذف`, and B16 makes the `ملفات` flip
all-or-nothing, so they stay registered and mapped until Increment G flips the module. `trq_file_size`
and the three predicates keep their own symbols under standing rule 3.

The 12 new `unnecessary unsafe block` warnings in `runtime-rs`'s tests are the pre-existing class #310
tracks (98 before this change) and follow the neighbouring tests' `trq_release` pattern verbatim;
`runtime-rs` is outside CI's clippy coverage and #310 will sweep all of them mechanically. The main
crate is clippy-clean.

Surfaced and filed rather than fixed here: `trq_string_to_path` does not guard a negative `len` before
`from_raw_parts` (#353). It is the shared reader for every path function in the module, so it belongs
in its own change.

---

## #355 — `احذف_مسار`: `unlink(2)`/`rmdir(2)`, and a row that named the wrong syscall

`docs/builtins-vs-stdlib.md` §1.3 category 7, the fourth of eleven to land, after `اكتب_مجرى` (#347),
`اقرأ_مجرى` (#350) and `حالة_مسار` (#352). `(نص) -> منطقي`, folding `احذف_ملف` and `احذف_مجلد`.

### Two decisions taken before the work

- **Which name.** §6.7.4's ordering result picked it: the path-taking primitives land alone, and the
  ground confirmed why the alternatives do not. `افتح_ملف` still owes the interpreter a handle table
  in the same change, and every shared interpreter helper is a *stateless free function* taking
  `&[Value]`, so that table would be the first cross-interpreter mutable state in the codebase.
  `معاملات_البرنامج` is a new kind of effect and there is no CLI syntax to pass a program's arguments
  at all — no `trailing_var_arg` anywhere in `src/cli/mod.rs`. `احذف_آخر` is **B10**, and
  `trq_array_pop` returns a *borrowed pointer into the array buffer*, so a name→symbol mapping cannot
  even use it.
- **`lstat`, not `stat`.** Taken before writing any code, by reading the two folded implementations.
  See below.

### What it found

- **The row named the wrong syscall — a sixth §1.3 defect class.** §1.3 said the choice between
  `unlink` and `rmdir` is "chosen by stat". Reading `trq_file_delete` (`remove_file`, which unlinks a
  symlink whatever it points at) against `trq_dir_delete` (`remove_dir`, which refuses one) shows it
  cannot be: following the link sends a symlink-to-directory to `rmdir` and answers `خطأ` where
  `احذف_ملف` answers `صحيح` today. **And a `stat`-based selector could never delete a broken symlink
  at all**, because `حالة_مسار` reads one as absent — it would strand every dangling link
  permanently. Adjacent to #352's fifth class and distinct: there the *range* of the return could not
  reproduce all N folded names, here the **dispatch** is wrong. The check that found both is the
  same — *read each folded name's implementation, one at a time* — which is the first time a check
  from a previous increment has paid for itself twice.
- **The fold is approximate, and the edge cannot be closed today.** `احذف_مسار` is more permissive
  than either name it folds, so the wrappers need a kind check, and the only kind available comes
  from `حالة_مسار`, which follows symlinks. So `احذف_ملف` refuses a symlink-to-directory where
  `remove_file` succeeds, and `احذف_مجلد` accepts one where `remove_dir` fails. One edge, two faces,
  documented rather than papered over — the move #352 made for `حجم_ملف` on directories. Blast radius
  is nil: neither name has an interpreter arm, so neither ever worked outside native compilation.
- **A `منطقي` return loses the arithmetic catcher entirely, and that is a property of the *semantic*
  type rather than the IR representation.** Every missing-return-type measurement since #347 has used
  `+ ١` as a catcher. It is unreachable here: `منطقي + عدد` is refused in the semantic analyzer, which
  never sees the IR return type, so the row cannot be written at all. `ليس` replaces it. Measured with
  the entry deleted, `اطبع` prints nothing natively and exits 0, `نوع` answers `مؤشر` in all three,
  and `== خطأ` and `ليس …` each fail native compilation with ت٠١٠١ — in **opposite directions**
  («'%v2' … 'i1' but expected 'ptr'» for the comparison, where the typed operand is the literal, and
  «'%v1' … 'ptr' but expected 'i1'» for the negation). So #347's scalar prediction transferred a
  second time, and #350's rule sharpens: predict from the use site, and check which use sites the
  *semantic* layer even admits before planning the measurement.
- **The fixture harness could not express this primitive's rows, in two independent ways, and both
  were found by reading it rather than by a red test.** `assert_prints_with_files` wrote fixtures
  once, *before* the backend loop — invisible for a primitive that only reads, fatal for one that
  deletes, since the interpreter leg consumes the fixture and the other two then see an absent path.
  And `fs::write` makes plain files only, so the directory and symlink rows had nowhere to live, and
  the program cannot create them itself because `انشئ_مجلد` has no interpreter arm. One additive
  `assert_prints_with_tree` fixed both: a `File`/`EmptyDir`/`Symlink` spec re-materialized per leg,
  with `assert_prints_with_files` becoming one line over it. **Generalisable: ask what the harness
  does *between* backend legs, not only what it can create.**
- **A destructive primitive's CI example is more constrained than #347's or #350's, and not where
  §6.7.4 predicted.** That section generalised the limit to *rows whose inputs are invariant under
  where and on what the program runs*, and predicted `افتح_ملف` would move the line by making a file
  creatable. For a destructive name the line does not move at all: an example must not delete
  anything, whatever the language can create. Every row it covers is a refusal.
- **Two example-file traps, caught only by running it.** `متغير غائب` was already declared in the
  `متغير_بيئة` section — `د٠١٠١` at parse time, because `examples/مدمجات.ترقيم` is one flat scope
  1200 lines long — and an `أغلفة` banner would have duplicated `حالة_مسار`'s in the golden. Neither
  is a language defect; recorded because the file's single-scope shape makes both inevitable again,
  and the next section should suffix its own names the way `تركيب الناتج (حذف)` already does.

### Cost

**Nine registration sites**, forecast from §6.7's discriminator before the work — neither half of the
path existed — and the **fourth consecutive** forecast to hit. Fourteen code files plus four docs.
One additive harness helper, which is the fourth consecutive increment whose own contract forced one:
env on the child (#338), stdin on the child (#350), fixture files (#352), and a fixture tree
*restored per leg* here.

**Nothing removed.** `احذف_ملف` and `احذف_مجلد` are `مكتبة`, not `يُحذف`, and **B16** makes the
`ملفات` flip all-or-nothing, so both stay registered and mapped until Increment G. `trq_file_delete`
and `trq_dir_delete` keep their symbols under standing rule 3.

The registry counts were **recounted** rather than incremented, per that row's own instruction, and
the recount corrected the method as well as the number: a regex over `scope.rs` undercounts, because
rustfmt wraps the longer `core_builtins()` entries across lines. The authoritative figures are the two
ratchet lists in `tests/builtin_registry_guard_tests.rs`, which a passing test checks against `Scope`
mechanically — 37 core + 163 stdlib = **200**. The debug interpreter is **37**, and this time every
`is_builtin` name was checked for a dispatch *mention* rather than the two sizes being compared, since
equal sizes can hide two offsetting errors.

`runtime-rs` clippy is unchanged at **79** warning lines under `--all-targets` (measured against
`develop`, not assumed — the first claim written here said 41→41 and was wrong on both numbers). It
would have been 80: the new test helper picked up an `unnecessary unsafe block` by copying the
neighbouring `trq_release` pattern verbatim. Dropping the block is the right copy to *not* make —
#310 is going to sweep all 55 of the existing ones, so new code should not add to the sweep. The main
crate is clippy-clean at `--all-targets`. Full suite green: 1453 unit tests and 193 builtin
execution tests, zero failures, and `examples/مدمجات.ترقيم` byte-identical across interpreter, JIT and
native.
