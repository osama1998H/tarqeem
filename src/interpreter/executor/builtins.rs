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

/// Milliseconds since the UNIX epoch, backing both `وقت_الآن` and `وقت_أداء`.
///
/// Shared with the debug interpreter so the two registries cannot drift; the
/// native runtime mirrors it in `runtime-rs/src/runtime.rs` (#241).
pub(crate) fn epoch_millis() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

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

/// `len` codepoints of `text` starting at codepoint `start`, or `""` where
/// there are none to take.
///
/// Shared with the debug interpreter rather than copied, the way `bytes_to_string`
/// below is, so the totality contract cannot drift between the two.
///
/// Every answer here is `trq_string_substr_chars`'s, not a chosen one: a negative
/// `start`, a non-positive `len`, and a `start` past the end all give `""`, and a
/// `len` past the end clamps. `chars()` matches the runtime exactly because it
/// walks lead bytes with `utf8_char_len`, which agrees with codepoint iteration on
/// every valid encoding — and a `Value::String` is a Rust `String`, so it cannot
/// hold an invalid one.
pub(crate) fn substring_by_chars(text: &str, start: i64, len: i64) -> String {
    if start < 0 || len <= 0 {
        return String::new();
    }

    text.chars()
        .skip(start as usize)
        .take(len as usize)
        .collect()
}

/// `قص_حروف`'s whole dispatch, shared so the two interpreters cannot answer
/// differently — the argument checks drift as easily as the slicing does.
pub(crate) fn call_substring_by_chars(args: &[Value]) -> RuntimeResult<Value> {
    let text = args.first().ok_or_else(|| {
        RuntimeError::invalid_operation("قص_حروف() تتطلب ثلاثة معاملات: نص، بداية، عدد أحرف")
    })?;
    let start = int_argument(args, 1)?;
    let len = int_argument(args, 2)?;

    match text {
        Value::String(s) => Ok(Value::string(substring_by_chars(s, start, len))),
        // The parameter is a pointer, so this mirrors a designed answer rather
        // than an artifact: `Type::compat` lets an un-narrowed `نص؟` into a `نص`
        // parameter, native lowers it to `ptr null`, and the runtime's guard
        // answers `""`. The two `عدد` parameters get no such arm — there native
        // turns `لا_شيء` into `0` only as a side effect of passing a null pointer
        // in an i64 slot, and encoding that would make the contract worse to
        // close a gap this name does not own (#327). `رمز_إلى_حرف` is the same.
        Value::Null => Ok(Value::string("")),
        _ => Err(RuntimeError::type_error("نص", text.type_name())),
    }
}

/// The `عدد` argument at `index`, or a type error naming what arrived instead.
fn int_argument(args: &[Value], index: usize) -> RuntimeResult<i64> {
    let value = args.get(index).ok_or_else(|| {
        RuntimeError::invalid_operation("قص_حروف() تتطلب ثلاثة معاملات: نص، بداية، عدد أحرف")
    })?;

    value
        .as_int()
        .ok_or_else(|| RuntimeError::type_error("عدد", value.type_name()))
}

/// The bytes of `values` decoded as UTF-8, or `None` when they are not a UTF-8
/// encoding — an element outside 0-255, or an invalid sequence.
///
/// Deliberately not `value_to_byte` above: that one *errors* out of range, which
/// would raise a runtime error here where native answers `""`. A divergence in
/// exactly the class `ثنائي_إلى_نص` exists to avoid.
pub(crate) fn bytes_to_string(values: &[Value]) -> Option<String> {
    let mut bytes = Vec::with_capacity(values.len());
    for value in values {
        bytes.push(u8::try_from(value.as_int()?).ok()?);
    }
    String::from_utf8(bytes).ok()
}

/// `أنهِ_البرنامج`'s whole dispatch, shared with the debug interpreter so the
/// masking cannot drift between them.
///
/// Returns `Err` rather than exiting: the interpreter runs inside whatever
/// process hosts it, and only that host knows whether ending it is right. See
/// `ErrorKind::ProgramExit`.
///
/// The status is `حالة & ٢٥٥`, mirroring `trq_exit`. Masking in both backends
/// instead of handing the value to the OS is what makes `أنهِ_البرنامج(٣٠٠)`
/// answer 44 everywhere rather than 44 on POSIX and 300 on Windows.
///
/// No `Value::Null` arm, and that is a decision rather than an omission: the
/// parameter is an `عدد`, so there is no pointer for a runtime null guard to
/// answer and codegen turns `لا_شيء` into `0` above the runtime. Mirroring one
/// would encode that artifact as contract — the narrowing #326 recorded for
/// `رمز_إلى_حرف`, which diverges identically on the same source (#327).
pub(crate) fn call_exit_program(args: &[Value]) -> RuntimeResult<Value> {
    let status = args.first().ok_or_else(|| {
        RuntimeError::invalid_operation("أنهِ_البرنامج() تتطلب معاملاً واحداً: حالة الخروج")
    })?;

    match status {
        Value::Int(code) => Err(RuntimeError::program_exit((code & 0xFF) as i32)),
        other => Err(RuntimeError::type_error("عدد", other.type_name())),
    }
}

/// The three stream descriptors `اكتب_مجرى` names without anything being opened,
/// mirroring `runtime-rs/src/io.rs`.
const STREAM_STDIN: i64 = 0;
const STREAM_STDOUT: i64 = 1;
const STREAM_STDERR: i64 = 2;

/// `اكتب_مجرى`'s failure answer. Collision-free, since a byte count is never
/// negative.
const WRITE_FAILED: i64 = -1;

/// `اكتب_مجرى`'s whole dispatch, shared with the debug interpreter the way
/// `call_substring_by_chars` is: three parameters' worth of contract, of which
/// the argument checks are most.
///
/// The descriptor is resolved before the array is read, so a write to nowhere is
/// refused whatever it was going to carry. `١` is stdout and `٢` is stderr;
/// `٣` upward names a file handle, and since no Arabic name opens one yet, the
/// table is provably empty and every such descriptor answers `-١` here **and**
/// natively. The two agree today for the same reason, not by coincidence — when
/// `افتح_ملف` lands, this arm needs a handle table of its own.
///
/// An element outside `0..=255` is not a byte, and the whole call is refused
/// rather than the value truncated: `[٣٠٠]` would otherwise be indistinguishable
/// from `[٤٤]`, the reason `ثنائي_إلى_نص` rejects too. Validation completes
/// before the first byte goes out, so a refused call leaves the stream untouched.
///
/// No `Value::Null` arm for the descriptor, for the reason `call_exit_program`
/// has none: it is an `عدد`, so there is no pointer for a runtime guard to
/// answer and codegen turns `لا_شيء` into `0` above the runtime (#326, #327). The
/// array is a pointer, so it does get one, and answers `٠` — the same count an
/// empty array answers, which loses nothing because both mean nothing was
/// written.
///
/// A failed flush answers `-١`, where the prints in this file discard it: that
/// convention belongs to functions returning nothing, which have no answer to
/// falsify. Kept identical to `trq_write_stream`, or the two backends would
/// disagree about a closed pipe.
///
/// The bytes reach the process's own streams even when the host is capturing
/// `اطبع` (the REPL's `capture_output`, the debugger's output events). That is
/// deliberate: the descriptor names the *process's* stream, so interposing a
/// host buffer would change what the program observably did. The cost is that a
/// DAP console does not mirror these bytes, recorded rather than worked around
/// because the debug output path needs its own pass either way (#346).
pub(crate) fn call_write_stream(args: &[Value]) -> RuntimeResult<Value> {
    let descriptor = args.first().ok_or_else(|| {
        RuntimeError::invalid_operation("اكتب_مجرى() تتطلب معاملين: المجرى والبايتات")
    })?;
    let bytes = args.get(1).ok_or_else(|| {
        RuntimeError::invalid_operation("اكتب_مجرى() تتطلب معاملين: المجرى والبايتات")
    })?;

    let stream = match descriptor {
        Value::Int(fd) => *fd,
        other => return Err(RuntimeError::type_error("عدد", other.type_name())),
    };

    if stream == STREAM_STDIN || stream < 0 {
        return Ok(Value::Int(WRITE_FAILED));
    }

    let payload = match bytes {
        Value::Array(arr) => match stream_bytes(&arr.borrow()) {
            Some(payload) => payload,
            None => return Ok(Value::Int(WRITE_FAILED)),
        },
        // Reached through an `أي` holder, as `ثنائي_إلى_نص`'s is: `مصفوفة<عدد>؟`
        // does not parse (ب٠١٠١) and a bare `لا_شيء` is refused at the argument.
        Value::Null => Vec::new(),
        _ => return Err(RuntimeError::type_error("مصفوفة", bytes.type_name())),
    };

    let written = payload.len() as i64;

    match stream {
        STREAM_STDOUT => {
            let mut out = io::stdout();
            if out.write_all(&payload).is_err() || out.flush().is_err() {
                return Ok(Value::Int(WRITE_FAILED));
            }
            Ok(Value::Int(written))
        }
        STREAM_STDERR => {
            let mut err = io::stderr();
            if err.write_all(&payload).is_err() || err.flush().is_err() {
                return Ok(Value::Int(WRITE_FAILED));
            }
            Ok(Value::Int(written))
        }
        // No handle can exist: nothing in the language opens one yet.
        _ => Ok(Value::Int(WRITE_FAILED)),
    }
}

/// `اقرأ_مجرى`'s whole dispatch, shared with the debug interpreter the way
/// `call_write_stream` above is.
///
/// The descriptor is settled before anything is read, the order its sibling
/// checks in: a read from nowhere is refused whatever it was going to hold. `٠`
/// is stdin; `١` and `٢` carry bytes the other way, so reading them answers
/// nothing. `٣` upward names a file handle, and since no Arabic name opens one
/// yet, the table is provably empty and every such descriptor answers an empty
/// array here **and** natively. The two agree today for the same reason, not by
/// coincidence — when `افتح_ملف` lands, this arm needs a handle table of its own.
///
/// **The read loops until `count` bytes or EOF**, mirroring `write_all` on the
/// other side. A single read answers whatever a pipe happens to hold, so the
/// length would depend on buffering and one program would answer differently
/// between runs and between backends. Kept identical to `trq_read_stream`, or the
/// two backends would disagree about a slow pipe.
///
/// **An empty array cannot be told apart from a refusal, deliberately.** A byte
/// count could use `-١` because a count is never negative, but every array is a
/// legitimate answer, so EOF, an unreadable descriptor and a zero `count` all
/// answer the same thing. `runtime-rs` already conflates the first two —
/// `trq_file_read_line` answers `""` for EOF and for an unknown handle alike.
///
/// **No `Value::Null` arm at all**, which makes this the first primitive since
/// #324 with none. Both parameters are `عدد`, so there is no pointer for a
/// runtime guard to answer and codegen turns `لا_شيء` into `0` above the runtime
/// (#326, #327). Do not add one by pattern-matching from `call_write_stream`,
/// whose *array* parameter is a pointer and so does get one.
///
/// Reads the process's own stdin even when the host has its own input path, for
/// the reason `call_write_stream` writes to the process's own streams: the
/// descriptor names the *process's* stream, so interposing a host buffer would
/// change what the program observably did. Bytes go through `io::stdin()`, the
/// shared buffered handle `ادخل` uses, so anything that call has already
/// buffered is not stepped past and lost.
pub(crate) fn call_read_stream(args: &[Value]) -> RuntimeResult<Value> {
    let descriptor = args.first().ok_or_else(|| {
        RuntimeError::invalid_operation("اقرأ_مجرى() تتطلب معاملين: المجرى وعدد البايتات")
    })?;
    let count = args.get(1).ok_or_else(|| {
        RuntimeError::invalid_operation("اقرأ_مجرى() تتطلب معاملين: المجرى وعدد البايتات")
    })?;

    let stream = match descriptor {
        Value::Int(fd) => *fd,
        other => return Err(RuntimeError::type_error("عدد", other.type_name())),
    };
    let wanted = match count {
        Value::Int(n) => *n,
        other => return Err(RuntimeError::type_error("عدد", other.type_name())),
    };

    // Stdin is the only readable stream there is, so one comparison covers every
    // refusal the runtime spells out separately: `١` and `٢` carry bytes the
    // other way, a negative descriptor names nothing, and `٣` upward names a
    // handle that cannot exist while nothing in the language opens one. Written
    // as one test rather than four so it does not read as more than it is —
    // **`افتح_ملف` must split the `≥٣` case out**, and that is the whole change
    // this arm needs when handles arrive.
    if stream != STREAM_STDIN || wanted <= 0 {
        return Ok(Value::array_from(Vec::new()));
    }

    let payload = fill_from_stdin(wanted);
    Ok(Value::array_from(
        payload.into_iter().map(|b| Value::Int(b as i64)).collect(),
    ))
}

/// Reads until `wanted` bytes have arrived or stdin ends.
///
/// Kept byte-for-byte equivalent to `fill_from` in `runtime-rs/src/io.rs`,
/// including the chunk bound: `wanted` is an `i64`, so allocating it up front
/// would let a typo'd `١٠**١٢` reserve a terabyte before a byte arrived.
fn fill_from_stdin(wanted: i64) -> Vec<u8> {
    const READ_CHUNK: usize = 64 * 1024;

    // Saturating rather than `as usize`: `عدد` is 64-bit but `usize` is 32 on a
    // wasm32 target, where `as` would truncate `٢**٣٢ + ٤` to `٤` and answer four
    // bytes for a request of four billion. Saturating reads to EOF instead, which
    // is the honest answer to "more bytes than this machine can address".
    let wanted = usize::try_from(wanted).unwrap_or(usize::MAX);
    let mut payload = Vec::new();
    let mut chunk = vec![0u8; wanted.min(READ_CHUNK)];
    let stdin = io::stdin();
    let mut source = stdin.lock();

    while payload.len() < wanted {
        let room = (wanted - payload.len()).min(chunk.len());
        match source.read(&mut chunk[..room]) {
            Ok(0) => break,
            Ok(read) => payload.extend_from_slice(&chunk[..read]),
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(_) => break,
        }
    }

    payload
}

/// Reads a `مصفوفة<عدد>`'s elements as bytes, or `None` if any is not one.
///
/// Split out so the whole array is validated before anything is written. Shaped
/// like `bytes_to_string`, and rejecting on the same two grounds: an element
/// that is not an `عدد`, and one outside a byte's range.
fn stream_bytes(values: &[Value]) -> Option<Vec<u8>> {
    let mut payload = Vec::with_capacity(values.len());
    for value in values {
        payload.push(u8::try_from(value.as_int()?).ok()?);
    }
    Some(payload)
}

/// `حالة_مسار`'s kind answers and its no-answer value, mirroring
/// `runtime-rs/src/io.rs`.
///
/// The fourth kind is load-bearing: a device exists and is not a file, so
/// `ملف_موجود` and `هل_ملف` disagree about it and three values could not fold
/// both names.
const PATH_KIND_ABSENT: i64 = 0;
const PATH_KIND_FILE: i64 = 1;
const PATH_KIND_DIR: i64 = 2;
const PATH_KIND_OTHER: i64 = 3;
const STAT_FIELD_KIND: i64 = 0;
const STAT_FIELD_SIZE: i64 = 1;
const STAT_NO_ANSWER: i64 = -1;

/// `حالة_مسار`'s whole dispatch, shared with the debug interpreter the way
/// `call_env_var` below is: two parameters' worth of contract, of which the
/// argument checks and the kind mapping are all of it.
///
/// The mapping is **duplicated** in `trq_path_status` and cannot be shared — the
/// compiler crate does not depend on `tarqeem-runtime`, and an `extern "C"`
/// function taking a `*const TrqString` could not read a `Value` anyway. So every
/// row of the contract is pinned cross-backend rather than only here, because
/// nothing but a test stops the two copies from drifting.
///
/// The path is read **raw**, like `متغير_بيئة`'s name: a filename with a leading
/// or trailing space is a legitimate filename, and `trq_path_status` does not
/// trim either.
pub(crate) fn call_path_status(args: &[Value]) -> RuntimeResult<Value> {
    const ARITY: &str = "حالة_مسار() تتطلب معاملين: المسار والحقل";

    let path = args
        .first()
        .ok_or_else(|| RuntimeError::invalid_operation(ARITY))?;
    let field = args
        .get(1)
        .ok_or_else(|| RuntimeError::invalid_operation(ARITY))?;
    let field = field
        .as_int()
        .ok_or_else(|| RuntimeError::type_error("عدد", field.type_name()))?;

    // The field is settled before the path: a question with no field has no
    // answer whatever the path holds, and an unknown one never touches the
    // filesystem. `trq_path_status` checks in the same order.
    if field != STAT_FIELD_KIND && field != STAT_FIELD_SIZE {
        return Ok(Value::Int(STAT_NO_ANSWER));
    }

    let path = match path {
        Value::String(text) => Some(text.as_str().to_string()),
        // A pointer parameter, so this mirrors the runtime's null guard the way
        // `call_env_var`'s arm does rather than encoding an artifact. Reached by
        // an un-narrowed `نص؟` through `Type::compat` and by an `أي` holder; the
        // `عدد` field gets no such arm (#327).
        Value::Null => None,
        other => return Err(RuntimeError::type_error("نص", other.type_name())),
    };

    // `fs::metadata` follows symlinks, and so do all four of the names this
    // folds, so a broken link reads as absent. An absent path, an unreadable one
    // and an empty name all land in the same `None`, deliberately.
    let metadata = path.and_then(|p| std::fs::metadata(p).ok());

    if field == STAT_FIELD_SIZE {
        return Ok(Value::Int(match metadata {
            // A byte length is a property of a regular file. A directory's
            // `st_size` is 4096 on ext4 and 64-96 on APFS, so answering it would
            // put a platform-dependent number in the contract.
            Some(meta) if meta.is_file() => meta.len() as i64,
            _ => STAT_NO_ANSWER,
        }));
    }

    Ok(Value::Int(match metadata {
        None => PATH_KIND_ABSENT,
        Some(meta) if meta.is_file() => PATH_KIND_FILE,
        Some(meta) if meta.is_dir() => PATH_KIND_DIR,
        Some(_) => PATH_KIND_OTHER,
    }))
}

/// `احذف_مسار`'s whole dispatch, shared with the debug interpreter the way
/// `call_path_status` above is.
///
/// The selector is **`symlink_metadata`, not `metadata`** — the one place this
/// deliberately disagrees with its sibling. `احذف_ملف` is `remove_file`, which
/// unlinks a symlink whatever it points at; `احذف_مجلد` is `remove_dir`, which
/// refuses one. Following the link would answer `خطأ` for a symlink-to-directory
/// where `احذف_ملف` answers `صحيح`, and could never delete a **broken** link at
/// all, since `حالة_مسار` reads one as absent.
///
/// Duplicated in `trq_path_delete` for the reason `call_path_status` records, so
/// every row is pinned cross-backend rather than only here.
pub(crate) fn call_path_delete(args: &[Value]) -> RuntimeResult<Value> {
    let path = args
        .first()
        .ok_or_else(|| RuntimeError::invalid_operation("احذف_مسار() تتطلب معاملاً: المسار"))?;

    let path = match path {
        Value::String(text) => text.as_str().to_string(),
        // The runtime answers `false` for a null path, so this is the designed
        // answer and not an artifact — the same arm `call_path_status` carries.
        Value::Null => return Ok(Value::Bool(false)),
        other => return Err(RuntimeError::type_error("نص", other.type_name())),
    };

    Ok(Value::Bool(match std::fs::symlink_metadata(&path) {
        // `rmdir`, not `rm -r`: a non-empty directory answers `خطأ`, which keeps
        // `احذف_مجلد`'s contract when it becomes a wrapper over this.
        Ok(meta) if meta.is_dir() => std::fs::remove_dir(&path).is_ok(),
        Ok(_) => std::fs::remove_file(&path).is_ok(),
        Err(_) => false,
    }))
}

/// `متغير_بيئة`'s whole dispatch, shared the way `call_substring_by_chars` above
/// is: the contract here lives almost entirely in the argument checks.
///
/// The name is read **raw**. `trq_env_get` deliberately does its own null/len/UTF-8
/// checks instead of going through `as_str`, which trims, so trimming here would
/// make `متغير_بيئة(" PATH ")` disagree between backends (#324).
pub(crate) fn call_env_var(args: &[Value]) -> RuntimeResult<Value> {
    let name = args.first().ok_or_else(|| {
        RuntimeError::invalid_operation("متغير_بيئة() تتطلب معاملاً واحداً: اسم المتغير")
    })?;

    match name {
        // One arm covers every failure the runtime folds into `""`: `env::var` is
        // the call `trq_env_get` makes too, and it errors alike on an empty name,
        // an absent one and a value that is not Unicode. Set-but-empty answers
        // `""` as well, so it is indistinguishable from unset by design.
        Value::String(s) => Ok(Value::string(std::env::var(s.as_str()).unwrap_or_default())),
        // A pointer parameter, so this mirrors the runtime's null guard the way
        // `قص_حروف`'s arm does, rather than encoding an integer-zero artifact.
        Value::Null => Ok(Value::string("")),
        _ => Err(RuntimeError::type_error("نص", name.type_name())),
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
                | "أنهِ_البرنامج"
                | "أنه_البرنامج"
                | "نم"
                | "وقت_الآن"
                | "وقت_أداء"
                // String functions
                | "قص_حروف"
                | "حرف_إلى_رمز"
                | "رمز_إلى_حرف"
                | "نص_إلى_ثنائي"
                | "ثنائي_إلى_نص"
                | "متغير_بيئة"
                | "اكتب_مجرى"
                | "اقرأ_مجرى"
                | "حالة_مسار"
                | "احذف_مسار"
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
                    // Bytes, not characters: `trq_string_len` is the byte-length
                    // symbol natively (`trq_string_len_chars` is the other one).
                    Value::String(s) => Ok(Value::Int(s.len() as i64)),
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

            "وقت_الآن" | "وقت_أداء" => Ok(Value::Int(epoch_millis())),

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

            "قص_حروف" => call_substring_by_chars(&args),

            "حرف_إلى_رمز" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation("حرف_إلى_رمز() تتطلب معامل واحد")
                })?;

                match val {
                    // `chars()`, so this is the first codepoint and not the first
                    // grapheme: the fatha in "مَ" is the second one.
                    Value::String(s) => Ok(Value::Int(s.chars().next().map_or(-1, |c| c as i64))),
                    // An un-narrowed `نص؟` is accepted into a `نص` parameter by
                    // `Type::compat`, and native lowers it to `ptr null`, where the
                    // runtime's guard answers -1. Erroring here instead would make
                    // the interpreter disagree with native on reachable source.
                    Value::Null => Ok(Value::Int(-1)),
                    _ => Err(RuntimeError::type_error("نص", val.type_name())),
                }
            }

            // No `Value::Null` arm, deliberately, and unlike `حرف_إلى_رمز`
            // above. There the runtime function guarded a null *pointer* and
            // answered -1, so the arm mirrored a designed contract. Here the
            // parameter is an integer: native turns `لا_شيء` into `0` as a
            // side effect of passing a null pointer in an i64 slot, and
            // encoding that artifact as "لا_شيء means U+0000" would make the
            // contract worse to close a gap this name does not own. `نم` and
            // `بتات_نفي` diverge identically on the same source (#327).
            "رمز_إلى_حرف" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation("رمز_إلى_حرف() تتطلب معامل واحد")
                })?;

                match val {
                    Value::Int(code) => Ok(Value::string(
                        u32::try_from(*code)
                            .ok()
                            .and_then(char::from_u32)
                            .map_or(String::new(), |c| c.to_string()),
                    )),
                    _ => Err(RuntimeError::type_error("عدد", val.type_name())),
                }
            }

            "نص_إلى_ثنائي" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation("نص_إلى_ثنائي() تتطلب معامل واحد")
                })?;

                match val {
                    // `bytes()`, not `chars()`: this is the one primitive whose
                    // unit is the octet, which is why `طول` of its result
                    // disagrees with `طول` of the string it came from.
                    Value::String(s) => Ok(Value::array_from(
                        s.bytes().map(|b| Value::Int(b as i64)).collect(),
                    )),
                    // Unlike `رمز_إلى_حرف`, the parameter here is a pointer: an
                    // un-narrowed `نص؟` lowers to `ptr null` and the runtime
                    // guard answers an empty array, so erroring instead would
                    // abort on source native runs fine.
                    Value::Null => Ok(Value::array()),
                    _ => Err(RuntimeError::type_error("نص", val.type_name())),
                }
            }

            // Its inverse, and the rejection is what keeps the backends
            // agreeing: a `Value::String` is a Rust `String` and cannot hold
            // invalid UTF-8 at all, so answering `""` is the only contract both
            // this and native can honour. See `trq_string_from_bytes`.
            "ثنائي_إلى_نص" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation("ثنائي_إلى_نص() تتطلب معامل واحد")
                })?;

                match val {
                    Value::Array(arr) => Ok(Value::string(
                        bytes_to_string(&arr.borrow()).unwrap_or_default(),
                    )),
                    // Load-bearing, but reached differently from its sibling:
                    // `مصفوفة<عدد>؟` does not parse (ب٠١٠١) and a bare `لا_شيء`
                    // is refused at the argument, so the route is an `أي` holder
                    // — where native's null guard answers `""` and erroring here
                    // instead would abort on source native runs fine.
                    Value::Null => Ok(Value::string("")),
                    _ => Err(RuntimeError::type_error("مصفوفة", val.type_name())),
                }
            }

            "متغير_بيئة" => call_env_var(&args),

            "اكتب_مجرى" => call_write_stream(&args),

            "اقرأ_مجرى" => call_read_stream(&args),

            "حالة_مسار" => call_path_status(&args),

            "احذف_مسار" => call_path_delete(&args),

            "أنهِ_البرنامج" | "أنه_البرنامج" => call_exit_program(&args),

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
                // `Value::to_display_string` is the single definition of how a
                // float reads; `f.to_string()` drops the fraction of a whole one
                // and made `"…" + 10000.0` disagree with `اطبع(10000.0)` (#185).
                match val {
                    Value::Float(f) => Ok(Value::string(Value::Float(*f).to_display_string())),
                    Value::Int(n) => Ok(Value::string(Value::Float(*n as f64).to_display_string())),
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
