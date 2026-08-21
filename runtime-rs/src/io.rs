//! I/O Operations for Tarqeem Runtime
//!
//! This module implements input/output functions for the Tarqeem language,
//! including console I/O, file operations, directory operations, and path utilities.

use crate::helpers::allocate_array;
use crate::string::{trq_string_new, trq_string_to_float, trq_string_to_int};
use crate::types::TrqString;
use crate::TrqArray;
use std::io::{self, BufRead, Write};

// ============================================================================
// Console Output Functions
// ============================================================================

/// Print a string to stdout.
#[no_mangle]
pub extern "C" fn trq_print(s: *const TrqString) {
    if s.is_null() {
        return;
    }

    unsafe {
        if (*s).data.is_null() {
            return;
        }
        let slice = std::slice::from_raw_parts((*s).data, (*s).len as usize);
        if let Ok(text) = std::str::from_utf8(slice) {
            print!("{}", text);
            io::stdout().flush().ok();
        }
    }
}

/// Print an integer to stdout.
#[no_mangle]
pub extern "C" fn trq_print_int(value: i64) {
    print!("{}", value);
    io::stdout().flush().ok();
}

/// Print a float to stdout.
///
/// Mirrors the interpreter's `Value::to_display_string` exactly (#185): a whole
/// float keeps one decimal place, so `اطبع(5.0)` reads `5.0` in every backend.
/// The previous `%g` convention printed `value as i64`, which agreed with no
/// other backend and made native output silently disagree with `tarqeem run`.
#[no_mangle]
pub extern "C" fn trq_print_float(value: f64) {
    if value.fract() == 0.0 {
        print!("{:.1}", value);
    } else {
        print!("{}", value);
    }
    io::stdout().flush().ok();
}

/// Print an optional scalar — a `عدد?`, `عدد_عشري?` or `منطقي?` — to stdout.
///
/// Optionals lower to a pointer, and a scalar one is a pointer to its boxed
/// value, so printing it means a null test and a load. Codegen used to hand the
/// pointer to `trq_print`, which reads it as a `TrqString*` and segfaults
/// (#185). `kind` selects the pointee: 0 = عدد, 1 = عدد_عشري, 2 = منطقي.
///
/// Rendering matches `Value::to_display_string`, including the whole-float
/// `.0` — a null prints `لا_شيء`, as the interpreter does.
#[no_mangle]
pub extern "C" fn trq_print_optional_scalar(value: *const u8, kind: i64) {
    if value.is_null() {
        print!("لا_شيء");
    } else {
        unsafe {
            match kind {
                1 => {
                    let f = *(value as *const f64);
                    if f.fract() == 0.0 {
                        print!("{:.1}", f);
                    } else {
                        print!("{}", f);
                    }
                }
                2 => print!("{}", if *value != 0 { "صحيح" } else { "خطأ" }),
                _ => print!("{}", *(value as *const i64)),
            }
        }
    }
    io::stdout().flush().ok();
}

/// Print a boolean to stdout in Arabic.
/// Outputs "صحيح" for true, "خطأ" for false — the language's boolean
/// literals (LANGUAGE_SPEC §4.3), matching the interpreter's rendering.
#[no_mangle]
pub extern "C" fn trq_print_bool(value: bool) {
    if value {
        print!("صحيح");
    } else {
        print!("خطأ");
    }
    io::stdout().flush().ok();
}

/// Print an array to stdout.
///
/// Prints the array in format: [elem1، elem2، elem3]
/// Uses Arabic comma (،) as separator.
///
/// # Safety
///
/// - `arr` must be a valid pointer to a `TrqArray` or null.
#[no_mangle]
pub extern "C" fn trq_print_array(arr: *const TrqArray) {
    if arr.is_null() {
        print!("[null]");
        io::stdout().flush().ok();
        return;
    }

    unsafe {
        let array = &*arr;
        print!("[");

        if !array.data.is_null() && array.len > 0 {
            // Assume i64 elements (most common case for Tarqeem arrays)
            let data = array.data as *const i64;
            for i in 0..array.len {
                if i > 0 {
                    print!("، "); // Arabic comma separator
                }
                print!("{}", *data.add(i as usize));
            }
        }

        print!("]");
        io::stdout().flush().ok();
    }
}

/// Print a newline to stdout.
#[no_mangle]
pub extern "C" fn trq_print_newline() {
    println!();
    io::stdout().flush().ok();
}

/// Print a string to stderr.
#[no_mangle]
pub extern "C" fn trq_print_error(s: *const TrqString) {
    if s.is_null() {
        return;
    }

    unsafe {
        if (*s).data.is_null() {
            return;
        }
        let slice = std::slice::from_raw_parts((*s).data, (*s).len as usize);
        if let Ok(text) = std::str::from_utf8(slice) {
            eprint!("{}", text);
            io::stderr().flush().ok();
        }
    }
}

// ============================================================================
// Console Input Functions
// ============================================================================

/// Read a line from stdin.
/// Returns a new TrqString with the input (newline stripped).
#[no_mangle]
pub extern "C" fn trq_input() -> *mut TrqString {
    let stdin = io::stdin();
    let mut line = String::new();

    if stdin.lock().read_line(&mut line).is_err() {
        return trq_string_new(std::ptr::null(), 0);
    }

    // Strip trailing newline characters
    let trimmed = line.trim_end_matches(&['\n', '\r'][..]);

    // Create TrqString from the input
    let bytes = trimmed.as_bytes();
    trq_string_new(bytes.as_ptr(), bytes.len() as i64)
}

/// Print a prompt and read a line from stdin.
#[no_mangle]
pub extern "C" fn trq_input_prompt(prompt: *const TrqString) -> *mut TrqString {
    // Print the prompt first
    trq_print(prompt);
    io::stdout().flush().ok();

    // Then read input
    trq_input()
}

/// Read a line from stdin and parse it as an integer.
#[no_mangle]
pub extern "C" fn trq_input_int() -> i64 {
    let input = trq_input();
    if input.is_null() {
        return 0;
    }
    let result = trq_string_to_int(input);
    crate::memory::trq_release(input as *mut u8);
    result
}

/// Read a line from stdin and parse it as a float.
#[no_mangle]
pub extern "C" fn trq_input_float() -> f64 {
    let input = trq_input();
    if input.is_null() {
        return 0.0;
    }
    let result = trq_string_to_float(input);
    crate::memory::trq_release(input as *mut u8);
    result
}

// ============================================================================
// File Operations
// ============================================================================

/// Helper to convert TrqString to a path string.
fn trq_string_to_path(s: *const TrqString) -> Option<String> {
    if s.is_null() {
        return None;
    }
    unsafe {
        if (*s).data.is_null() {
            return None;
        }
        let slice = std::slice::from_raw_parts((*s).data, (*s).len as usize);
        std::str::from_utf8(slice).ok().map(|s| s.to_string())
    }
}

/// Check if a file or directory exists.
#[no_mangle]
pub extern "C" fn trq_file_exists(path: *const TrqString) -> bool {
    match trq_string_to_path(path) {
        Some(p) => std::path::Path::new(&p).exists(),
        None => false,
    }
}

/// Check if the path is a file.
#[no_mangle]
pub extern "C" fn trq_file_is_file(path: *const TrqString) -> bool {
    match trq_string_to_path(path) {
        Some(p) => std::path::Path::new(&p).is_file(),
        None => false,
    }
}

/// Check if the path is a directory.
#[no_mangle]
pub extern "C" fn trq_file_is_dir(path: *const TrqString) -> bool {
    match trq_string_to_path(path) {
        Some(p) => std::path::Path::new(&p).is_dir(),
        None => false,
    }
}

/// Read entire file contents as a string.
#[no_mangle]
pub extern "C" fn trq_file_read(path: *const TrqString) -> *mut TrqString {
    let path_str = match trq_string_to_path(path) {
        Some(p) => p,
        None => return trq_string_new(std::ptr::null(), 0),
    };

    match std::fs::read_to_string(&path_str) {
        Ok(content) => {
            let bytes = content.as_bytes();
            trq_string_new(bytes.as_ptr(), bytes.len() as i64)
        }
        Err(_) => trq_string_new(std::ptr::null(), 0),
    }
}

/// Write string content to a file.
#[no_mangle]
pub extern "C" fn trq_file_write(path: *const TrqString, content: *const TrqString) -> bool {
    let path_str = match trq_string_to_path(path) {
        Some(p) => p,
        None => return false,
    };

    let content_slice = if content.is_null() {
        &[]
    } else {
        unsafe {
            if (*content).data.is_null() {
                &[]
            } else {
                std::slice::from_raw_parts((*content).data, (*content).len as usize)
            }
        }
    };

    std::fs::write(&path_str, content_slice).is_ok()
}

/// Append string content to a file.
#[no_mangle]
pub extern "C" fn trq_file_append(path: *const TrqString, content: *const TrqString) -> bool {
    use std::fs::OpenOptions;

    let path_str = match trq_string_to_path(path) {
        Some(p) => p,
        None => return false,
    };

    let content_slice = if content.is_null() {
        &[]
    } else {
        unsafe {
            if (*content).data.is_null() {
                &[]
            } else {
                std::slice::from_raw_parts((*content).data, (*content).len as usize)
            }
        }
    };

    match OpenOptions::new().append(true).create(true).open(&path_str) {
        Ok(mut file) => file.write_all(content_slice).is_ok(),
        Err(_) => false,
    }
}

/// Delete a file.
#[no_mangle]
pub extern "C" fn trq_file_delete(path: *const TrqString) -> bool {
    match trq_string_to_path(path) {
        Some(p) => std::fs::remove_file(&p).is_ok(),
        None => false,
    }
}

/// Copy a file.
#[no_mangle]
pub extern "C" fn trq_file_copy(src: *const TrqString, dst: *const TrqString) -> bool {
    let src_str = match trq_string_to_path(src) {
        Some(p) => p,
        None => return false,
    };
    let dst_str = match trq_string_to_path(dst) {
        Some(p) => p,
        None => return false,
    };

    std::fs::copy(&src_str, &dst_str).is_ok()
}

/// Move/rename a file.
#[no_mangle]
pub extern "C" fn trq_file_move(src: *const TrqString, dst: *const TrqString) -> bool {
    let src_str = match trq_string_to_path(src) {
        Some(p) => p,
        None => return false,
    };
    let dst_str = match trq_string_to_path(dst) {
        Some(p) => p,
        None => return false,
    };

    std::fs::rename(&src_str, &dst_str).is_ok()
}

/// Get file size in bytes.
#[no_mangle]
pub extern "C" fn trq_file_size(path: *const TrqString) -> i64 {
    match trq_string_to_path(path) {
        Some(p) => match std::fs::metadata(&p) {
            Ok(meta) => meta.len() as i64,
            Err(_) => -1,
        },
        None => -1,
    }
}

// ============================================================================
// Path Status
// ============================================================================

/// What `حالة_مسار`'s kind field answers: what is *at* a path.
///
/// `PATH_KIND_OTHER` is not decoration. [`trq_file_exists`] is `Path::exists()`,
/// which is true for a device, a socket or a fifo, while [`trq_file_is_file`] is
/// false for the same path — so a three-value kind could not reproduce both of
/// the names this folds. A row that promises to fold N names needs enough range
/// to answer for all N.
const PATH_KIND_ABSENT: i64 = 0;
const PATH_KIND_FILE: i64 = 1;
const PATH_KIND_DIR: i64 = 2;
const PATH_KIND_OTHER: i64 = 3;

/// The fields this function knows. One field per call, which is what keeps the
/// answer an `عدد` and keeps a struct off the FFI — the mistake the nine date
/// constructors made (#298).
const STAT_FIELD_KIND: i64 = 0;
const STAT_FIELD_SIZE: i64 = 1;

/// No answer: a field this function does not know, or a size asked of something
/// that has no byte length. Collision-free for the kind field, which never
/// answers negative.
const STAT_NO_ANSWER: i64 = -1;

/// Backs the core builtin `حالة_مسار`: `stat(2)`, one field per call.
///
/// `حقل ٠` answers what is at the path — `٠` absent, `١` a file, `٢` a
/// directory, `٣` something that exists and is neither. `حقل ١` answers the byte
/// length of a **regular file**, and `-١` for everything else. Any other field
/// answers `-١`.
///
/// **The field is checked before the path.** A question with no field has no
/// answer whatever the path holds, so an unknown field never touches the
/// filesystem. Same order as [`trq_write_stream`], which settles the descriptor
/// before reading the payload.
///
/// **Symlinks are followed**, because `fs::metadata` follows them and so do all
/// four of the names this folds. A broken symlink therefore reads as absent —
/// the link exists, but nothing is at the path it names.
///
/// **A directory has no size.** `trq_file_size` answers the OS `st_size` here,
/// which is 4096 on ext4 and 64–96 on APFS; no test and no golden file can
/// assert a number that changes with the filesystem. So the size is a property
/// of a regular file and `-١` otherwise, which makes the row assertable. This is
/// a deliberate delta from `trq_file_size`, recorded because the future
/// `حجم_ملف` wrapper inherits it.
///
/// **Absent, unreadable, empty and null are one answer.** A missing path, a
/// permission error, an empty name and a null pointer all answer `٠` / `-١`. A
/// caller that must tell them apart checks the path it passed — the same
/// conflation [`trq_env_get`](crate::runtime::trq_env_get) makes for an unset
/// versus an empty variable, and [`trq_file_read_line`] for EOF versus an
/// unknown handle.
///
/// The path is read as given, with no trimming: a filename with a leading or
/// trailing space is a legitimate filename.
///
/// # Returns
///
/// The requested field, or `-١` where there is no answer. Total — every
/// `(path, field)` pair is a valid call, and none can panic.
///
/// # Safety
///
/// - `path` must be a valid pointer to a `TrqString` or null.
///
/// # C Equivalent
/// ```c
/// int64_t trq_path_status(const TrqString* path, int64_t field);
/// ```
#[no_mangle]
pub extern "C" fn trq_path_status(path: *const TrqString, field: i64) -> i64 {
    if field != STAT_FIELD_KIND && field != STAT_FIELD_SIZE {
        return STAT_NO_ANSWER;
    }

    let metadata = trq_string_to_path(path).and_then(|p| std::fs::metadata(p).ok());

    if field == STAT_FIELD_SIZE {
        return match metadata {
            Some(meta) if meta.is_file() => meta.len() as i64,
            _ => STAT_NO_ANSWER,
        };
    }

    match metadata {
        None => PATH_KIND_ABSENT,
        Some(meta) if meta.is_file() => PATH_KIND_FILE,
        Some(meta) if meta.is_dir() => PATH_KIND_DIR,
        Some(_) => PATH_KIND_OTHER,
    }
}

// ============================================================================
// Path Deletion
// ============================================================================

/// Backs the core builtin `احذف_مسار`: `unlink(2)` for a file, `rmdir(2)` for an
/// empty directory, chosen by `lstat`.
///
/// **`lstat`, not `stat`** — [`symlink_metadata`](std::fs::symlink_metadata). The
/// registry row specified `stat`, and the two names this folds show why that is
/// wrong: [`trq_file_delete`] is `remove_file`, which unlinks a symlink whatever
/// it points at, while [`trq_dir_delete`] is `remove_dir`, which refuses one. And
/// [`trq_path_status`] reads a **broken** symlink as absent, so a `stat`-based
/// selector could never delete one at all.
///
/// So this acts on the **name** while [`trq_path_status`] answers about the
/// **target**. The two disagree about symlinks on purpose.
///
/// **Not recursive.** A non-empty directory answers `false`, keeping
/// [`trq_dir_delete`]'s `rmdir` contract.
///
/// **Absent, unreadable, empty and null are one answer**, as in
/// [`trq_path_status`]. The path is read as given, with no trimming.
///
/// # Returns
///
/// `true` only if something was removed. Total — every path is a valid call, and
/// none can panic.
///
/// # Safety
///
/// - `path` must be a valid pointer to a `TrqString` or null.
///
/// # C Equivalent
/// ```c
/// bool trq_path_delete(const TrqString* path);
/// ```
#[no_mangle]
pub extern "C" fn trq_path_delete(path: *const TrqString) -> bool {
    let path = match trq_string_to_path(path) {
        Some(p) => p,
        None => return false,
    };

    match std::fs::symlink_metadata(&path) {
        Ok(meta) if meta.is_dir() => std::fs::remove_dir(&path).is_ok(),
        // `lstat` reports a symlink as a non-directory whatever it targets, and on
        // Unix `unlink` removes it. On Windows a *directory* symlink or junction is
        // a directory reparse point: `DeleteFileW` refuses it and only
        // `RemoveDirectoryW` unlinks it. The `||` is that fallback, and it is a
        // provable no-op elsewhere — it runs only when `remove_file` already
        // failed, and `remove_dir` on anything `lstat` called a non-directory
        // fails too. Portable rather than a `cfg(windows)` branch so the one
        // documented behaviour has one implementation.
        Ok(_) => std::fs::remove_file(&path).is_ok() || std::fs::remove_dir(&path).is_ok(),
        Err(_) => false,
    }
}

// ============================================================================
// File Handle/Stream Operations
// ============================================================================

use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::sync::atomic::{AtomicI64, Ordering};

/// Global counter for generating unique file handles.
///
/// Starts at 3, not 1: `اكتب_مجرى` names a stream by descriptor, and `٠`, `١`
/// and `٢` are stdin, stdout and stderr there. Handed out from 1, the first file
/// a program opened *was* handle 1, so a write meant for it went to the terminal
/// instead — silently, since both succeed. Blocker B15 in
/// `docs/builtins-vs-stdlib.md` §7.
///
/// `0` was never a valid handle: every `trq_file_open_*` returns it on failure.
static NEXT_FILE_HANDLE: AtomicI64 = AtomicI64::new(3);

/// File handle types for streaming I/O.
enum FileHandle {
    Reader(BufReader<File>),
    Writer(BufWriter<File>),
}

// Thread-local storage for file handles
thread_local! {
    static FILE_HANDLES: RefCell<HashMap<i64, FileHandle>> = RefCell::new(HashMap::new());
}

/// Get the next unique file handle ID.
fn get_next_file_handle() -> i64 {
    NEXT_FILE_HANDLE.fetch_add(1, Ordering::SeqCst)
}

/// Store a file handle and return its ID.
fn store_file_handle(handle: FileHandle) -> i64 {
    let id = get_next_file_handle();
    FILE_HANDLES.with(|handles| {
        handles.borrow_mut().insert(id, handle);
    });
    id
}

/// Open a file for reading and return a handle.
/// Returns 0 on error.
#[no_mangle]
pub extern "C" fn trq_file_open_read(path: *const TrqString) -> i64 {
    let path_str = match trq_string_to_path(path) {
        Some(p) => p,
        None => return 0,
    };

    match File::open(&path_str) {
        Ok(file) => {
            let reader = BufReader::new(file);
            store_file_handle(FileHandle::Reader(reader))
        }
        Err(_) => 0,
    }
}

/// Open a file for writing (creates or truncates) and return a handle.
/// Returns 0 on error.
#[no_mangle]
pub extern "C" fn trq_file_open_write(path: *const TrqString) -> i64 {
    let path_str = match trq_string_to_path(path) {
        Some(p) => p,
        None => return 0,
    };

    match File::create(&path_str) {
        Ok(file) => {
            let writer = BufWriter::new(file);
            store_file_handle(FileHandle::Writer(writer))
        }
        Err(_) => 0,
    }
}

/// Open a file for appending and return a handle.
/// Returns 0 on error.
#[no_mangle]
pub extern "C" fn trq_file_open_append(path: *const TrqString) -> i64 {
    use std::fs::OpenOptions;

    let path_str = match trq_string_to_path(path) {
        Some(p) => p,
        None => return 0,
    };

    match OpenOptions::new().append(true).create(true).open(&path_str) {
        Ok(file) => {
            let writer = BufWriter::new(file);
            store_file_handle(FileHandle::Writer(writer))
        }
        Err(_) => 0,
    }
}

/// The three ways `افتح_ملف` can open a path.
///
/// Chosen to match the `ثابت`s `stdlib/ملفات/ملف.ترقيم` already declares. That
/// file also declares `وضع_قراءة_كتابة = 3`, which has no path here: a
/// read-write handle is neither a [`FileHandle::Reader`] nor a
/// [`FileHandle::Writer`], and a third variant would touch all eight functions
/// that read `FILE_HANDLES`. So `3` is refused rather than served silently by
/// one of its halves.
const OPEN_READ: i64 = 0;
const OPEN_WRITE: i64 = 1;
const OPEN_APPEND: i64 = 2;

/// `افتح_ملف`'s failure answer, and deliberately **not** the `0` the three
/// openers above return.
///
/// `0` names stdin in the stream pair, so a failed open answering `0` would send
/// `اقرأ_مجرى(٠، ن)` to the keyboard — succeeding, and reading the wrong thing.
/// `-1` is already refused by both stream primitives, and is what `اكتب_مجرى`
/// answers when it fails.
const OPEN_FAILED: i64 = -1;

/// Backs the core builtin `افتح_ملف`: opens `path` in `mode` and answers a
/// handle the stream pair can name.
///
/// The mode is settled before the path, the order [`trq_write_stream`] settles
/// its descriptor in and [`trq_path_status`] its field: an unknown mode is not a
/// request the filesystem should be troubled with, so `افتح_ملف(".", ٩)` creates
/// nothing.
///
/// Every handle is `>= 3`, since `NEXT_FILE_HANDLE` starts there, so the answer
/// goes straight to `اكتب_مجرى`/`اقرأ_مجرى` without colliding with a console
/// stream.
///
/// **Bytes written to a handle the program does not close are not guaranteed to
/// reach the file until it ends**, because [`trq_write_stream`]'s handle path does
/// not flush. [`trq_file_close`] is what makes them land sooner, and
/// [`flush_open_writers`] is what catches whatever is still open at the end.
///
/// **A directory is refused in every mode**, so the answer does not depend on the
/// platform — see the note in the body.
///
/// # Returns
/// * The handle, or `-1` for an unknown mode, a directory, an absent path *in read
///   mode*, a path that cannot be opened or created, an empty name, or a null
///   pointer. Write and append modes create an absent path rather than refusing
///   it.
///
/// # Safety
///
/// - `path` must be a valid pointer to a `TrqString` or null.
///
/// # C Equivalent
/// ```c
/// int64_t trq_file_open(const TrqString* path, int64_t mode);
/// ```
#[no_mangle]
pub extern "C" fn trq_file_open(path: *const TrqString, mode: i64) -> i64 {
    let handle = match mode {
        OPEN_READ => trq_file_open_read(path),
        OPEN_WRITE => trq_file_open_write(path),
        OPEN_APPEND => trq_file_open_append(path),
        _ => return OPEN_FAILED,
    };

    if handle == 0 {
        return OPEN_FAILED;
    }

    // A directory is refused, and this is what makes the answer the same on every
    // platform. `File::open` succeeds on one under POSIX and fails on Windows,
    // where opening a directory handle needs `FILE_FLAG_BACKUP_SEMANTICS` that
    // `std` does not pass — so honouring `open(2)` literally would make one
    // program answer a handle on Linux and `-1` on Windows, in a contract row.
    // The handle would be useless either way: `اقرأ_مجرى` reads nothing from it
    // and `قائمة_مجلد` is how a directory is listed.
    //
    // Checked through the open handle rather than the path, so there is no window
    // between the test and the open. Provably a no-op on Windows, where the open
    // already failed — the shape #355 chose over a `cfg(windows)` arm, because one
    // documented behaviour should have one implementation. Devices and FIFOs are
    // **not** refused: `/dev/null` opens usefully and portably enough.
    if handle_is_directory(handle) {
        trq_file_close(handle);
        return OPEN_FAILED;
    }

    handle
}

/// Whether an open handle names a directory, answering `false` if it cannot tell.
fn handle_is_directory(handle: i64) -> bool {
    FILE_HANDLES.with(|handles| {
        let handles = handles.borrow();
        match handles.get(&handle) {
            Some(FileHandle::Reader(reader)) => reader
                .get_ref()
                .metadata()
                .map(|data| data.is_dir())
                .unwrap_or(false),
            _ => false,
        }
    })
}

/// Backs the core builtin `اغلق_ملف`: releases `handle` and, for a writer, sends
/// its buffer on its way.
///
/// The handle leaves the table whatever happens next, and the number is never
/// handed out again — `get_next_file_handle` only counts up. So a second close,
/// a console stream and a handle that was never opened all answer the same, which
/// is the conflation a `bool` return has no spare value to avoid.
///
/// The answer **folds the flush**, so it means "released, and a writer's bytes are
/// away" rather than merely "the table held it". `close(2)` reports `EIO` and
/// `اغلق_ملف` exists to make bytes land, so answering `true` over a failed flush
/// would be a name lying about the one thing it is for. That is a deliberate
/// deviation from the row in `docs/builtins-vs-stdlib.md` §1.3, which called this
/// implementation "reused unchanged".
///
/// # Returns
/// * `true` if the table held the handle and a writer's flush succeeded, `false`
///   for `٠`/`١`/`٢`, a negative handle, one already released, one never opened,
///   or a flush that failed.
///
/// # C Equivalent
/// ```c
/// bool trq_file_close(int64_t handle);
/// ```
#[no_mangle]
pub extern "C" fn trq_file_close(handle: i64) -> bool {
    FILE_HANDLES.with(|handles| {
        let mut handles = handles.borrow_mut();
        match handles.remove(&handle) {
            // Flushed before the `BufWriter` drops, which would flush anyway and
            // discard the result.
            Some(FileHandle::Writer(mut writer)) => writer.flush().is_ok(),
            Some(FileHandle::Reader(_)) => true,
            None => false,
        }
    })
}

/// Read a line from a file handle.
/// Returns an empty string at EOF or on error.
/// The newline character is stripped from the result.
#[no_mangle]
pub extern "C" fn trq_file_read_line(handle: i64) -> *mut TrqString {
    FILE_HANDLES.with(|handles| {
        let mut handles = handles.borrow_mut();
        if let Some(FileHandle::Reader(reader)) = handles.get_mut(&handle) {
            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(0) => trq_string_new(std::ptr::null(), 0), // EOF
                Ok(_) => {
                    // Remove trailing newline characters
                    if line.ends_with('\n') {
                        line.pop();
                        if line.ends_with('\r') {
                            line.pop();
                        }
                    }
                    let bytes = line.as_bytes();
                    trq_string_new(bytes.as_ptr(), bytes.len() as i64)
                }
                Err(_) => trq_string_new(std::ptr::null(), 0),
            }
        } else {
            trq_string_new(std::ptr::null(), 0)
        }
    })
}

/// Write a line to a file handle (appends newline).
/// Returns true on success, false on error.
#[no_mangle]
pub extern "C" fn trq_file_write_line(handle: i64, content: *const TrqString) -> bool {
    FILE_HANDLES.with(|handles| {
        let mut handles = handles.borrow_mut();
        if let Some(FileHandle::Writer(writer)) = handles.get_mut(&handle) {
            let content_slice = if content.is_null() {
                &[]
            } else {
                unsafe {
                    if (*content).data.is_null() {
                        &[]
                    } else {
                        std::slice::from_raw_parts((*content).data, (*content).len as usize)
                    }
                }
            };

            // Write content and newline
            if writer.write_all(content_slice).is_err() {
                return false;
            }
            writer.write_all(b"\n").is_ok()
        } else {
            false
        }
    })
}

/// Check if at end of file.
/// Returns true at EOF or for invalid handles.
#[no_mangle]
pub extern "C" fn trq_file_eof(handle: i64) -> bool {
    FILE_HANDLES.with(|handles| {
        let mut handles = handles.borrow_mut();
        if let Some(FileHandle::Reader(reader)) = handles.get_mut(&handle) {
            // Peek to check if at EOF without consuming
            reader.fill_buf().map(|buf| buf.is_empty()).unwrap_or(true)
        } else {
            true // Invalid handle = EOF
        }
    })
}

/// Flush the file buffer.
/// Returns true on success, false on error.
#[no_mangle]
pub extern "C" fn trq_file_flush(handle: i64) -> bool {
    FILE_HANDLES.with(|handles| {
        let mut handles = handles.borrow_mut();
        if let Some(FileHandle::Writer(writer)) = handles.get_mut(&handle) {
            writer.flush().is_ok()
        } else {
            false
        }
    })
}

/// Flush every open writer, closing nothing.
///
/// Called at program end, for the handles a program did **not** close itself:
/// `اغلق_ملف` flushes those, and [`trq_write_stream`]'s handle path never does —
/// so without this a
/// `BufWriter`'s contents are **lost** natively, where `main` is `extern "C"`
/// and no thread-local destructor runs, while the interpreter still wrote them.
/// That is the divergence [`crate::trq_exit`]'s stdout flush already exists to
/// prevent, one layer down.
pub(crate) fn flush_open_writers() {
    FILE_HANDLES.with(|handles| {
        for handle in handles.borrow_mut().values_mut() {
            if let FileHandle::Writer(writer) = handle {
                let _ = writer.flush();
            }
        }
    });
}

// ============================================================================
// Stream Writing
// ============================================================================

/// The three stream descriptors every program has without opening anything.
///
/// Reserved out of the handle space by `NEXT_FILE_HANDLE`, which starts past
/// them.
const STREAM_STDIN: i64 = 0;
const STREAM_STDOUT: i64 = 1;
const STREAM_STDERR: i64 = 2;

/// Failure. Collision-free as an answer because a byte count is never negative.
const WRITE_FAILED: i64 = -1;

/// Backs the core builtin `اكتب_مجرى`: writes `bytes` to the stream named by
/// `fd`, answering the number of bytes written.
///
/// `١` is stdout, `٢` is stderr, and `٣` upward is a handle from
/// [`trq_file_open_write`] or [`trq_file_open_append`]. `٠` is stdin, so writing
/// to it fails; so does any negative descriptor, and any handle the table does
/// not hold.
///
/// **All or nothing.** Every element is range-checked before the first byte
/// goes out, so a rejected array leaves the stream untouched. An element outside
/// `0..=255` is not a byte and answers `-1`; it is *not* truncated to its low
/// byte, because `[300]` would then be indistinguishable from `[44]` — the same
/// reason [`crate::string::trq_string_from_bytes`] rejects rather than truncates.
///
/// That range check is also what catches a type-confused call. A `TrqArray`
/// carries no element-kind tag, so a `مصفوفة<نص>` reaching this parameter
/// through an `أي` holder has the same `elem_size` as a `مصفوفة<عدد>`; its
/// elements are `TrqString` pointers, whose values are far outside a byte, so
/// the call is refused instead of writing addresses. `elem_size` is checked
/// first and separately, before `data` is read, for the reason spelled out in
/// `trq_string_from_bytes`: a `TrqString` is 24 bytes and its `data` field sits
/// at offset 24, one past the end.
///
/// An empty array answers `0`, and so does a null one. Both mean nothing was
/// written, so nothing is lost by giving them the same answer — and `0` is a
/// count here, not a sentinel.
///
/// Console writes go through Rust's `Stdout`/`Stderr` rather than a raw `write`,
/// and flush like every other print in this module. A raw descriptor write would
/// bypass the buffer `trq_print` shares, and the two would interleave in an
/// order that depends on buffering rather than on the program.
///
/// **A failed flush answers `-1`, unlike the prints here, which discard it.**
/// That convention was set by functions returning nothing: `trq_print` has no
/// answer to falsify. This one does. `Stdout` is line-buffered, so a payload
/// with no trailing newline sits in the buffer and a closed pipe fails at the
/// flush rather than at the `write_all` — reporting the count there would claim
/// bytes reached the descriptor when none did.
///
/// The handle path does **not** flush, matching `trq_file_write_line`: a
/// `BufWriter` exists to batch, and `trq_file_flush` is how a caller asks. So the
/// count means "accepted by the stream" for a handle and "left for the
/// descriptor" for a console stream — the difference the two APIs already had.
///
/// # Safety
///
/// - `bytes` must be a valid pointer to a `TrqArray` or null.
///
/// # C Equivalent
/// ```c
/// int64_t trq_write_stream(int64_t fd, const TrqArray* bytes);
/// ```
#[no_mangle]
pub extern "C" fn trq_write_stream(fd: i64, bytes: *const TrqArray) -> i64 {
    // The descriptor is checked before the array: a write to nowhere is refused
    // whatever it was going to carry.
    if fd == STREAM_STDIN || fd < 0 {
        return WRITE_FAILED;
    }

    let payload = match collect_stream_bytes(bytes) {
        Some(payload) => payload,
        None => return WRITE_FAILED,
    };

    // An unknown handle is still an error even with nothing to send, so the
    // descriptor is resolved below rather than short-circuited on an empty
    // payload.
    let written = payload.len() as i64;

    match fd {
        STREAM_STDOUT => {
            let mut out = io::stdout();
            if out.write_all(&payload).is_err() || out.flush().is_err() {
                return WRITE_FAILED;
            }
            written
        }
        STREAM_STDERR => {
            let mut err = io::stderr();
            if err.write_all(&payload).is_err() || err.flush().is_err() {
                return WRITE_FAILED;
            }
            written
        }
        _ => FILE_HANDLES.with(|handles| {
            let mut handles = handles.borrow_mut();
            match handles.get_mut(&fd) {
                Some(FileHandle::Writer(writer)) => {
                    if writer.write_all(&payload).is_err() {
                        return WRITE_FAILED;
                    }
                    written
                }
                // A reader is as wrong a destination as a handle that was never
                // opened, so both answer the same.
                _ => WRITE_FAILED,
            }
        }),
    }
}

/// Reads a `مصفوفة<عدد>` as bytes, or `None` if it is not one.
///
/// Separate from the write so the whole array is validated before any of it is
/// sent. A null or empty array is `Some(empty)` — a count of zero, not a
/// rejection.
fn collect_stream_bytes(bytes: *const TrqArray) -> Option<Vec<u8>> {
    if bytes.is_null() {
        return Some(Vec::new());
    }

    unsafe {
        // `elem_size` 8 for the reason `trq_string_to_bytes` writes 8: a
        // `مصفوفة<عدد>` element is a raw inline i64 slot. Checked before `data`,
        // which is why the order matters — see the note above.
        if (*bytes).elem_size != 8 {
            return None;
        }

        if (*bytes).data.is_null() || (*bytes).len <= 0 {
            return Some(Vec::new());
        }

        let slots = std::slice::from_raw_parts((*bytes).data as *const i64, (*bytes).len as usize);
        let mut payload = Vec::with_capacity(slots.len());
        for slot in slots {
            payload.push(u8::try_from(*slot).ok()?);
        }
        Some(payload)
    }
}

// ============================================================================
// Stream Reading
// ============================================================================

/// The most one read attempt asks for at a time.
///
/// `count` is an `i64`, so `Vec::with_capacity(count)` would let a typo'd
/// `١٠**١٢` reserve a terabyte before a single byte arrived. Reading in bounded
/// chunks and appending means a bounded stream costs only what it delivers, and
/// an unbounded one grows only as fast as it is actually read. The size itself
/// is not observable — it changes how the bytes are fetched, never how many
/// answer.
const READ_CHUNK: usize = 64 * 1024;

/// Backs the core builtin `اقرأ_مجرى`: reads up to `count` bytes from the stream
/// named by `fd`, answering them as a `مصفوفة<عدد>`, one byte per element.
///
/// `٠` is stdin and `٣` upward is a handle from [`trq_file_open_read`]. `١` and
/// `٢` are output streams, so reading them answers nothing; so does any negative
/// descriptor, any handle the table does not hold, a handle opened for writing,
/// a non-positive `count`, and a stream already at EOF.
///
/// **The read loops until `count` bytes or EOF**, which is the mirror of
/// [`trq_write_stream`]'s `write_all`. A single `read` answers whatever a pipe
/// happens to hold, so the length would depend on buffering and one program
/// would answer differently between runs and between backends — a flake rather
/// than a bug. Looping leaves a short answer one meaning: the stream ended.
///
/// **An empty array cannot be told apart from a refusal, and that is
/// deliberate.** A byte count could use `-١` because a count is never negative,
/// but every array is a legitimate answer, so EOF, an unreadable descriptor, an
/// absent handle and a zero `count` all answer the same empty array. This file
/// already conflates the first three: [`trq_file_read_line`] answers `""` for
/// EOF, for a read error *and* for an unknown handle, and [`trq_file_eof`]
/// answers `true` for a handle that was never opened. A caller that must
/// distinguish them checks the descriptor it passed.
///
/// **Never a raw descriptor read.** [`trq_write_stream`] avoids a raw `write(2)`
/// so it cannot reorder against the buffer `trq_print` shares; the read side has
/// that reason and a sharper one — on a terminal fd 1 is read-write, so a raw
/// `read(1, …)` would block on the keyboard instead of answering. Stdin goes
/// through `io::stdin()`, the process-wide buffered handle [`trq_input`] uses,
/// so bytes that call has already buffered are not stepped past and lost.
///
/// A read error after some bytes have arrived answers those bytes rather than
/// discarding them: they are already out of the stream and cannot be put back.
///
/// # Returns
/// * A new `TrqArray` of `elem_size` 8 holding one byte per element, empty when
///   there is nothing to read. NULL only if allocation failed.
///
/// # Safety
///
/// - The returned pointer is a fresh reference-counted `TrqArray`; the caller
///   owns it and releases it the way it releases any other array.
///
/// # C Equivalent
/// ```c
/// TrqArray* trq_read_stream(int64_t fd, int64_t count);
/// ```
#[no_mangle]
pub extern "C" fn trq_read_stream(fd: i64, count: i64) -> *mut TrqArray {
    // The descriptor is settled before anything is read, the order
    // `trq_write_stream` checks in: a read from nowhere is refused whatever it
    // was going to hold.
    if fd < 0 || fd == STREAM_STDOUT || fd == STREAM_STDERR || count <= 0 {
        return byte_array_from(&[]);
    }

    let payload = if fd == STREAM_STDIN {
        fill_from(&mut io::stdin().lock(), count)
    } else {
        FILE_HANDLES.with(|handles| {
            let mut handles = handles.borrow_mut();
            match handles.get_mut(&fd) {
                Some(FileHandle::Reader(reader)) => fill_from(reader, count),
                // A writer is as wrong a source as a handle that was never
                // opened, so both answer the same — the mirror of the reader arm
                // in `trq_write_stream`.
                _ => Vec::new(),
            }
        })
    };

    byte_array_from(&payload)
}

/// Reads until `count` bytes have arrived or the source ends.
///
/// `Ok(0)` is EOF and stops the loop; `Interrupted` is retried, since it means
/// nothing about the stream; any other error stops the loop and keeps whatever
/// arrived before it.
fn fill_from(source: &mut impl io::Read, count: i64) -> Vec<u8> {
    // Saturating rather than `as usize`: `عدد` is 64-bit but `usize` is 32 on a
    // wasm32 target, where `as` would truncate `٢**٣٢ + ٤` to `٤` and answer four
    // bytes for a request of four billion. Saturating reads to EOF instead, which
    // is the honest answer to "more bytes than this machine can address".
    let wanted = usize::try_from(count).unwrap_or(usize::MAX);
    let mut payload = Vec::new();
    let mut chunk = vec![0u8; wanted.min(READ_CHUNK)];

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

/// Wraps bytes as a `مصفوفة<عدد>`.
///
/// `elem_size` 8 and one raw inline i64 slot per byte, for the reason
/// `trq_string_to_bytes` writes them that way: it is what codegen's `load i64`
/// after `trq_array_get` reads back. An empty slice answers the same empty array
/// `trq_string_to_bytes("")` does, so the two byte producers agree on it.
fn byte_array_from(payload: &[u8]) -> *mut TrqArray {
    unsafe {
        let arr = allocate_array(payload.len() as i64, 8);
        if arr.is_null() || payload.is_empty() {
            return arr;
        }

        let slots = (*arr).data as *mut i64;
        for (index, byte) in payload.iter().enumerate() {
            *slots.add(index) = *byte as i64;
        }

        arr
    }
}
// ============================================================================
// Directory Operations
// ============================================================================

/// Create a directory.
#[no_mangle]
pub extern "C" fn trq_dir_create(path: *const TrqString) -> bool {
    match trq_string_to_path(path) {
        Some(p) => std::fs::create_dir(&p).is_ok(),
        None => false,
    }
}

/// Create a directory and all parent directories.
#[no_mangle]
pub extern "C" fn trq_dir_create_all(path: *const TrqString) -> bool {
    match trq_string_to_path(path) {
        Some(p) => std::fs::create_dir_all(&p).is_ok(),
        None => false,
    }
}

/// Delete an empty directory.
#[no_mangle]
pub extern "C" fn trq_dir_delete(path: *const TrqString) -> bool {
    match trq_string_to_path(path) {
        Some(p) => std::fs::remove_dir(&p).is_ok(),
        None => false,
    }
}

/// List directory entries.
/// Returns an array of TrqString pointers.
#[no_mangle]
pub extern "C" fn trq_dir_list(path: *const TrqString) -> *mut TrqArray {
    use crate::array::trq_array_new;

    let result = trq_array_new(0, std::mem::size_of::<*mut TrqString>() as i64);
    if result.is_null() {
        return result;
    }

    let path_str = match trq_string_to_path(path) {
        Some(p) => p,
        None => return result,
    };

    if let Ok(entries) = std::fs::read_dir(&path_str) {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                let name_str = trq_string_new(name.as_ptr(), name.len() as i64);
                crate::array::trq_array_push(
                    result,
                    &name_str as *const _ as *const u8,
                    std::mem::size_of::<*mut TrqString>() as i64,
                );
            }
        }
    }

    result
}

/// Get the current working directory.
#[no_mangle]
pub extern "C" fn trq_dir_current() -> *mut TrqString {
    match std::env::current_dir() {
        Ok(path) => {
            let path_str = path.to_string_lossy();
            let bytes = path_str.as_bytes();
            trq_string_new(bytes.as_ptr(), bytes.len() as i64)
        }
        Err(_) => trq_string_new(std::ptr::null(), 0),
    }
}

/// Get the user's home directory.
#[no_mangle]
pub extern "C" fn trq_dir_home() -> *mut TrqString {
    match std::env::var("HOME") {
        Ok(home) => {
            let bytes = home.as_bytes();
            trq_string_new(bytes.as_ptr(), bytes.len() as i64)
        }
        Err(_) => trq_string_new(std::ptr::null(), 0),
    }
}

/// Get the system temp directory.
#[no_mangle]
pub extern "C" fn trq_dir_temp() -> *mut TrqString {
    let temp = std::env::temp_dir();
    let temp_str = temp.to_string_lossy();
    let bytes = temp_str.as_bytes();
    trq_string_new(bytes.as_ptr(), bytes.len() as i64)
}

// ============================================================================
// Path Operations
// ============================================================================

/// Join two path components.
#[no_mangle]
pub extern "C" fn trq_path_join(
    base: *const TrqString,
    component: *const TrqString,
) -> *mut TrqString {
    let base_str = match trq_string_to_path(base) {
        Some(p) => p,
        None => {
            return if component.is_null() {
                trq_string_new(std::ptr::null(), 0)
            } else {
                unsafe { trq_string_new((*component).data, (*component).len) }
            };
        }
    };

    let comp_str = match trq_string_to_path(component) {
        Some(p) => p,
        None => {
            let bytes = base_str.as_bytes();
            return trq_string_new(bytes.as_ptr(), bytes.len() as i64);
        }
    };

    let joined = std::path::Path::new(&base_str).join(&comp_str);
    let joined_str = joined.to_string_lossy();
    let bytes = joined_str.as_bytes();
    trq_string_new(bytes.as_ptr(), bytes.len() as i64)
}

/// Get the parent directory of a path.
#[no_mangle]
pub extern "C" fn trq_path_parent(path: *const TrqString) -> *mut TrqString {
    let path_str = match trq_string_to_path(path) {
        Some(p) => p,
        None => return trq_string_new(std::ptr::null(), 0),
    };

    let path = std::path::Path::new(&path_str);
    match path.parent() {
        Some(parent) => {
            let parent_str = parent.to_string_lossy();
            let bytes = parent_str.as_bytes();
            if bytes.is_empty() {
                // Return "." for current directory
                trq_string_new(".".as_ptr(), 1)
            } else {
                trq_string_new(bytes.as_ptr(), bytes.len() as i64)
            }
        }
        None => trq_string_new(".".as_ptr(), 1),
    }
}

/// Get the filename from a path.
#[no_mangle]
pub extern "C" fn trq_path_filename(path: *const TrqString) -> *mut TrqString {
    let path_str = match trq_string_to_path(path) {
        Some(p) => p,
        None => return trq_string_new(std::ptr::null(), 0),
    };

    let path = std::path::Path::new(&path_str);
    match path.file_name() {
        Some(name) => {
            let name_str = name.to_string_lossy();
            let bytes = name_str.as_bytes();
            trq_string_new(bytes.as_ptr(), bytes.len() as i64)
        }
        None => trq_string_new(std::ptr::null(), 0),
    }
}

/// Get the file extension from a path.
#[no_mangle]
pub extern "C" fn trq_path_extension(path: *const TrqString) -> *mut TrqString {
    let path_str = match trq_string_to_path(path) {
        Some(p) => p,
        None => return trq_string_new(std::ptr::null(), 0),
    };

    let path = std::path::Path::new(&path_str);
    match path.extension() {
        Some(ext) => {
            let ext_str = ext.to_string_lossy();
            let bytes = ext_str.as_bytes();
            trq_string_new(bytes.as_ptr(), bytes.len() as i64)
        }
        None => trq_string_new(std::ptr::null(), 0),
    }
}

/// Get the filename without extension (stem).
#[no_mangle]
pub extern "C" fn trq_path_stem(path: *const TrqString) -> *mut TrqString {
    let path_str = match trq_string_to_path(path) {
        Some(p) => p,
        None => return trq_string_new(std::ptr::null(), 0),
    };

    let path = std::path::Path::new(&path_str);
    match path.file_stem() {
        Some(stem) => {
            let stem_str = stem.to_string_lossy();
            let bytes = stem_str.as_bytes();
            trq_string_new(bytes.as_ptr(), bytes.len() as i64)
        }
        None => trq_string_new(std::ptr::null(), 0),
    }
}

/// Convert a path to absolute.
#[no_mangle]
pub extern "C" fn trq_path_absolute(path: *const TrqString) -> *mut TrqString {
    let path_str = match trq_string_to_path(path) {
        Some(p) => p,
        None => return trq_dir_current(),
    };

    match std::fs::canonicalize(&path_str) {
        Ok(abs_path) => {
            let abs_str = abs_path.to_string_lossy();
            let bytes = abs_str.as_bytes();
            trq_string_new(bytes.as_ptr(), bytes.len() as i64)
        }
        Err(_) => {
            // If canonicalize fails (file doesn't exist), try to construct absolute path
            let path = std::path::Path::new(&path_str);
            if path.is_absolute() {
                let bytes = path_str.as_bytes();
                trq_string_new(bytes.as_ptr(), bytes.len() as i64)
            } else {
                match std::env::current_dir() {
                    Ok(cwd) => {
                        let abs = cwd.join(&path_str);
                        let abs_str = abs.to_string_lossy();
                        let bytes = abs_str.as_bytes();
                        trq_string_new(bytes.as_ptr(), bytes.len() as i64)
                    }
                    Err(_) => {
                        let bytes = path_str.as_bytes();
                        trq_string_new(bytes.as_ptr(), bytes.len() as i64)
                    }
                }
            }
        }
    }
}

/// Check if a path is absolute.
#[no_mangle]
pub extern "C" fn trq_path_is_absolute(path: *const TrqString) -> bool {
    match trq_string_to_path(path) {
        Some(p) => std::path::Path::new(&p).is_absolute(),
        None => false,
    }
}

/// Get the path separator for the current platform.
#[no_mangle]
pub extern "C" fn trq_path_separator() -> *mut TrqString {
    let sep = std::path::MAIN_SEPARATOR.to_string();
    let bytes = sep.as_bytes();
    trq_string_new(bytes.as_ptr(), bytes.len() as i64)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trq_print_bool() {
        // This test is more of a manual verification
        // The function prints to stdout
    }

    #[test]
    fn test_file_exists() {
        // Test with a file that should exist
        let path = trq_string_new("Cargo.toml".as_ptr(), "Cargo.toml".len() as i64);
        assert!(trq_file_exists(path));
        unsafe {
            crate::memory::trq_release(path as *mut u8);
        }

        // Test with a file that doesn't exist
        let path2 = trq_string_new(
            "nonexistent_file_xyz.txt".as_ptr(),
            "nonexistent_file_xyz.txt".len() as i64,
        );
        assert!(!trq_file_exists(path2));
        unsafe {
            crate::memory::trq_release(path2 as *mut u8);
        }
    }

    #[test]
    fn test_path_operations() {
        let path = trq_string_new(
            "/home/user/file.txt".as_ptr(),
            "/home/user/file.txt".len() as i64,
        );

        // Test parent
        let parent = trq_path_parent(path);
        assert!(!parent.is_null());
        unsafe {
            let parent_slice = std::slice::from_raw_parts((*parent).data, (*parent).len as usize);
            let parent_str = std::str::from_utf8(parent_slice).unwrap();
            assert_eq!(parent_str, "/home/user");
            crate::memory::trq_release(parent as *mut u8);
        }

        // Test filename
        let filename = trq_path_filename(path);
        assert!(!filename.is_null());
        unsafe {
            let filename_slice =
                std::slice::from_raw_parts((*filename).data, (*filename).len as usize);
            let filename_str = std::str::from_utf8(filename_slice).unwrap();
            assert_eq!(filename_str, "file.txt");
            crate::memory::trq_release(filename as *mut u8);
        }

        // Test extension
        let ext = trq_path_extension(path);
        assert!(!ext.is_null());
        unsafe {
            let ext_slice = std::slice::from_raw_parts((*ext).data, (*ext).len as usize);
            let ext_str = std::str::from_utf8(ext_slice).unwrap();
            assert_eq!(ext_str, "txt");
            crate::memory::trq_release(ext as *mut u8);
        }

        // Test stem
        let stem = trq_path_stem(path);
        assert!(!stem.is_null());
        unsafe {
            let stem_slice = std::slice::from_raw_parts((*stem).data, (*stem).len as usize);
            let stem_str = std::str::from_utf8(stem_slice).unwrap();
            assert_eq!(stem_str, "file");
            crate::memory::trq_release(stem as *mut u8);
        }

        // Test is_absolute
        assert!(trq_path_is_absolute(path));

        unsafe {
            crate::memory::trq_release(path as *mut u8);
        }
    }

    #[test]
    fn test_path_join() {
        let base = trq_string_new("/home/user".as_ptr(), "/home/user".len() as i64);
        let comp = trq_string_new("file.txt".as_ptr(), "file.txt".len() as i64);

        let joined = trq_path_join(base, comp);
        assert!(!joined.is_null());

        unsafe {
            let joined_slice = std::slice::from_raw_parts((*joined).data, (*joined).len as usize);
            let joined_str = std::str::from_utf8(joined_slice).unwrap();
            assert_eq!(joined_str, "/home/user/file.txt");

            crate::memory::trq_release(joined as *mut u8);
            crate::memory::trq_release(base as *mut u8);
            crate::memory::trq_release(comp as *mut u8);
        }
    }

    #[test]
    fn test_dir_current() {
        let cwd = trq_dir_current();
        assert!(!cwd.is_null());
        unsafe {
            assert!((*cwd).len > 0);
            crate::memory::trq_release(cwd as *mut u8);
        }
    }

    #[test]
    fn test_dir_temp() {
        let temp = trq_dir_temp();
        assert!(!temp.is_null());
        unsafe {
            assert!((*temp).len > 0);
            crate::memory::trq_release(temp as *mut u8);
        }
    }

    #[test]
    fn test_file_handle_read() {
        // Create a test file
        let test_content = "سطر أول\nسطر ثاني\nسطر ثالث";
        let test_path = "/tmp/tarqeem_test_file_handle.txt";
        std::fs::write(test_path, test_content).unwrap();

        // Open for reading
        let path = trq_string_new(test_path.as_ptr(), test_path.len() as i64);
        let handle = trq_file_open_read(path);
        assert!(handle > 0);

        // Read first line
        let line1 = trq_file_read_line(handle);
        assert!(!line1.is_null());
        unsafe {
            let slice = std::slice::from_raw_parts((*line1).data, (*line1).len as usize);
            let line_str = std::str::from_utf8(slice).unwrap();
            assert_eq!(line_str, "سطر أول");
            crate::memory::trq_release(line1 as *mut u8);
        }

        // Read second line
        let line2 = trq_file_read_line(handle);
        assert!(!line2.is_null());
        unsafe {
            let slice = std::slice::from_raw_parts((*line2).data, (*line2).len as usize);
            let line_str = std::str::from_utf8(slice).unwrap();
            assert_eq!(line_str, "سطر ثاني");
            crate::memory::trq_release(line2 as *mut u8);
        }

        // Read third line
        let line3 = trq_file_read_line(handle);
        assert!(!line3.is_null());
        unsafe {
            let slice = std::slice::from_raw_parts((*line3).data, (*line3).len as usize);
            let line_str = std::str::from_utf8(slice).unwrap();
            assert_eq!(line_str, "سطر ثالث");
            crate::memory::trq_release(line3 as *mut u8);
        }

        // Check EOF
        assert!(trq_file_eof(handle));

        // Close handle
        assert!(trq_file_close(handle));

        // Cleanup
        std::fs::remove_file(test_path).ok();
        unsafe {
            crate::memory::trq_release(path as *mut u8);
        }
    }

    #[test]
    fn test_file_handle_write() {
        let test_path = "/tmp/tarqeem_test_file_handle_write.txt";

        // Open for writing
        let path = trq_string_new(test_path.as_ptr(), test_path.len() as i64);
        let handle = trq_file_open_write(path);
        assert!(handle > 0);

        // Write lines
        let line1 = trq_string_new("مرحبا".as_ptr(), "مرحبا".len() as i64);
        let line2 = trq_string_new("بالعالم".as_ptr(), "بالعالم".len() as i64);

        assert!(trq_file_write_line(handle, line1));
        assert!(trq_file_write_line(handle, line2));

        // Flush and close
        assert!(trq_file_flush(handle));
        assert!(trq_file_close(handle));

        // Verify content
        let content = std::fs::read_to_string(test_path).unwrap();
        assert_eq!(content, "مرحبا\nبالعالم\n");

        // Cleanup
        std::fs::remove_file(test_path).ok();
        unsafe {
            crate::memory::trq_release(path as *mut u8);
            crate::memory::trq_release(line1 as *mut u8);
            crate::memory::trq_release(line2 as *mut u8);
        }
    }

    #[test]
    fn test_file_handle_append() {
        let test_path = "/tmp/tarqeem_test_file_handle_append.txt";

        // Create initial file
        std::fs::write(test_path, "السطر الأول\n").unwrap();

        // Open for appending
        let path = trq_string_new(test_path.as_ptr(), test_path.len() as i64);
        let handle = trq_file_open_append(path);
        assert!(handle > 0);

        // Append a line
        let line = trq_string_new("السطر الثاني".as_ptr(), "السطر الثاني".len() as i64);
        assert!(trq_file_write_line(handle, line));

        // Close
        assert!(trq_file_close(handle));

        // Verify content
        let content = std::fs::read_to_string(test_path).unwrap();
        assert_eq!(content, "السطر الأول\nالسطر الثاني\n");

        // Cleanup
        std::fs::remove_file(test_path).ok();
        unsafe {
            crate::memory::trq_release(path as *mut u8);
            crate::memory::trq_release(line as *mut u8);
        }
    }

    #[test]
    fn test_file_handle_invalid() {
        // Test operations on invalid handle
        assert!(!trq_file_close(99999));
        assert!(trq_file_eof(99999)); // Invalid handle returns true for EOF
        assert!(!trq_file_flush(99999));

        let line = trq_file_read_line(99999);
        assert!(!line.is_null());
        unsafe {
            assert_eq!((*line).len, 0); // Empty string
            crate::memory::trq_release(line as *mut u8);
        }
    }

    // ===== Phase 8: Additional I/O Error Condition Tests =====

    #[test]
    fn test_file_not_found() {
        // Test reading a non-existent file
        let nonexistent = "/tmp/tarqeem_nonexistent_file_xyz_12345.txt";
        let path = trq_string_new(nonexistent.as_ptr(), nonexistent.len() as i64);

        // trq_file_read should return empty string for non-existent file
        let content = trq_file_read(path);
        assert!(!content.is_null());
        unsafe {
            assert_eq!((*content).len, 0);
            crate::memory::trq_release(content as *mut u8);
        }

        // trq_file_open_read should return 0 for non-existent file
        let handle = trq_file_open_read(path);
        assert_eq!(handle, 0);

        // trq_file_exists should return false
        assert!(!trq_file_exists(path));

        // trq_file_is_file should return false
        assert!(!trq_file_is_file(path));

        // trq_file_size should return -1 for non-existent file
        assert_eq!(trq_file_size(path), -1);

        unsafe {
            crate::memory::trq_release(path as *mut u8);
        }
    }

    #[test]
    fn test_file_write_readonly() {
        // Test writing to a read-only location (root directory on Unix)
        // This should fail gracefully
        let readonly_path = "/readonly_test_file.txt";
        let path = trq_string_new(readonly_path.as_ptr(), readonly_path.len() as i64);

        let content = trq_string_new("test".as_ptr(), 4);

        // trq_file_write should return false for permission denied
        let result = trq_file_write(path, content);
        assert!(!result);

        // trq_file_open_write should return 0 for permission denied
        let handle = trq_file_open_write(path);
        assert_eq!(handle, 0);

        // trq_file_append should return false for permission denied
        let append_result = trq_file_append(path, content);
        assert!(!append_result);

        // trq_file_open_append should return 0 for permission denied
        let append_handle = trq_file_open_append(path);
        assert_eq!(append_handle, 0);

        unsafe {
            crate::memory::trq_release(path as *mut u8);
            crate::memory::trq_release(content as *mut u8);
        }
    }

    #[test]
    fn test_path_operations_edge_cases() {
        // Test with null path
        let result = trq_path_parent(std::ptr::null());
        assert!(!result.is_null());
        unsafe {
            assert_eq!((*result).len, 0);
            crate::memory::trq_release(result as *mut u8);
        }

        // Test with empty path
        let empty = trq_string_new(std::ptr::null(), 0);
        assert!(!trq_file_exists(empty));
        assert!(!trq_path_is_absolute(empty));
        unsafe {
            crate::memory::trq_release(empty as *mut u8);
        }

        // Test path join with null components
        let base = trq_string_new("/home".as_ptr(), 5);
        let joined_null = trq_path_join(base, std::ptr::null());
        assert!(!joined_null.is_null());
        unsafe {
            let slice =
                std::slice::from_raw_parts((*joined_null).data, (*joined_null).len as usize);
            let str_val = std::str::from_utf8(slice).unwrap();
            assert_eq!(str_val, "/home");
            crate::memory::trq_release(joined_null as *mut u8);
        }

        // Test path with trailing slash
        let trailing = trq_string_new("/home/user/".as_ptr(), "/home/user/".len() as i64);
        let parent = trq_path_parent(trailing);
        assert!(!parent.is_null());
        unsafe {
            let slice = std::slice::from_raw_parts((*parent).data, (*parent).len as usize);
            let str_val = std::str::from_utf8(slice).unwrap();
            // Parent of "/home/user/" should be "/home/user" or "/home" depending on impl
            assert!(str_val.starts_with("/home"));
            crate::memory::trq_release(parent as *mut u8);
            crate::memory::trq_release(trailing as *mut u8);
        }

        // Test path with no extension
        let no_ext = trq_string_new("/home/file".as_ptr(), "/home/file".len() as i64);
        let ext = trq_path_extension(no_ext);
        assert!(!ext.is_null());
        unsafe {
            assert_eq!((*ext).len, 0); // No extension
            crate::memory::trq_release(ext as *mut u8);
            crate::memory::trq_release(no_ext as *mut u8);
        }

        // Test path with multiple extensions
        let multi_ext = trq_string_new("file.tar.gz".as_ptr(), "file.tar.gz".len() as i64);
        let ext2 = trq_path_extension(multi_ext);
        assert!(!ext2.is_null());
        unsafe {
            let slice = std::slice::from_raw_parts((*ext2).data, (*ext2).len as usize);
            let str_val = std::str::from_utf8(slice).unwrap();
            assert_eq!(str_val, "gz"); // Only last extension
            crate::memory::trq_release(ext2 as *mut u8);
        }

        let stem = trq_path_stem(multi_ext);
        assert!(!stem.is_null());
        unsafe {
            let slice = std::slice::from_raw_parts((*stem).data, (*stem).len as usize);
            let str_val = std::str::from_utf8(slice).unwrap();
            assert_eq!(str_val, "file.tar"); // Stem without last extension
            crate::memory::trq_release(stem as *mut u8);
            crate::memory::trq_release(multi_ext as *mut u8);
        }

        // Test relative path
        let relative = trq_string_new("relative/path".as_ptr(), "relative/path".len() as i64);
        assert!(!trq_path_is_absolute(relative));
        unsafe {
            crate::memory::trq_release(relative as *mut u8);
            crate::memory::trq_release(base as *mut u8);
        }
    }

    /// The `٣`-and-up half of `اكتب_مجرى`'s descriptor map: bytes reach a handle
    /// opened by `trq_file_open_write`.
    ///
    /// Driven from Rust because it predates `افتح_ملف` (#362), which is what made
    /// this path reachable from Tarqeem source. Kept here as the unit-level pin:
    /// the cross-backend tests exercise the same path through the opener.
    #[test]
    fn test_write_stream_writes_to_a_file_handle() {
        let test_path = "/tmp/tarqeem_test_write_stream_handle.txt";

        let path = trq_string_new(test_path.as_ptr(), test_path.len() as i64);
        let handle = trq_file_open_write(path);
        assert!(handle > 0);
        // Descriptors 0-2 are the streams, so a handle can never collide with
        // them. This is blocker B15, and the assertion is what keeps it fixed.
        assert!(
            handle >= 3,
            "المعرِّف {handle} يزاحم مجرى قياسياً / handle collides with a standard stream"
        );

        let bytes = byte_array(&[b'A', 0xD9, 0x85, b'\n']);
        assert_eq!(trq_write_stream(handle, bytes), 4);

        assert!(trq_file_flush(handle));
        assert!(trq_file_close(handle));

        assert_eq!(std::fs::read_to_string(test_path).unwrap(), "Aم\n");

        std::fs::remove_file(test_path).ok();
        unsafe {
            crate::memory::trq_release(path as *mut u8);
            crate::memory::trq_release(bytes as *mut u8);
        }
    }

    /// A closed handle, one never opened, and a reader are all "nowhere to
    /// write", so all three answer `-1`.
    #[test]
    fn test_write_stream_refuses_a_handle_it_cannot_write_to() {
        let test_path = "/tmp/tarqeem_test_write_stream_refuses.txt";
        std::fs::write(test_path, "سطر\n").unwrap();

        let bytes = byte_array(b"x");

        // Never opened.
        assert_eq!(trq_write_stream(9_999, bytes), -1);

        // A reader is as wrong a destination as no handle at all.
        let path = trq_string_new(test_path.as_ptr(), test_path.len() as i64);
        let reader = trq_file_open_read(path);
        assert!(reader >= 3);
        assert_eq!(trq_write_stream(reader, bytes), -1);
        assert!(trq_file_close(reader));

        // Closed: the handle is gone from the table, so it is the first case
        // again under a plausible-looking number.
        let writer = trq_file_open_write(path);
        assert!(writer >= 3);
        assert!(trq_file_close(writer));
        assert_eq!(trq_write_stream(writer, bytes), -1);

        std::fs::remove_file(test_path).ok();
        unsafe {
            crate::memory::trq_release(path as *mut u8);
            crate::memory::trq_release(bytes as *mut u8);
        }
    }

    /// Descriptor `٠` is stdin and a negative one names nothing, and both are
    /// refused before the array is looked at — which is why this can pass a null
    /// one and still expect `-1`.
    #[test]
    fn test_write_stream_resolves_the_descriptor_before_the_bytes() {
        assert_eq!(trq_write_stream(0, std::ptr::null()), -1);
        assert_eq!(trq_write_stream(-1, std::ptr::null()), -1);
    }

    /// Nothing to write is a count of zero: an empty array and a null pointer
    /// both answer `0` rather than failing.
    #[test]
    fn test_write_stream_of_nothing_answers_zero() {
        let empty = byte_array(&[]);
        assert_eq!(trq_write_stream(1, empty), 0);
        assert_eq!(trq_write_stream(1, std::ptr::null()), 0);
        unsafe {
            crate::memory::trq_release(empty as *mut u8);
        }
    }

    /// The type-confusion guard, and the reason `elem_size` is checked before
    /// `data` is read.
    ///
    /// A `TrqArray` carries no element-kind tag, so an `أي` holder can land a
    /// `TrqString` on this parameter. A `TrqString` is 24 bytes — `len`, `cap`,
    /// `data` — and `elem_size` sits at offset 16, *inside* it, while `data` sits
    /// at offset 24, one past the end. Reading `data` first would be a heap
    /// over-read; rejecting on `elem_size` first cannot be, because a real `data`
    /// pointer is never 8. Same guard, same order, as
    /// `crate::string::trq_string_from_bytes`.
    #[test]
    fn test_write_stream_rejects_an_array_whose_elements_are_not_bytes() {
        let text = "مرحبا";
        let masquerading = trq_string_new(text.as_ptr(), text.len() as i64);
        assert_eq!(
            trq_write_stream(1, masquerading as *const TrqArray),
            -1,
            "قُرئ نصٌّ كأنه مصفوفة بايتات / a string was read as a byte array"
        );

        // And the ordinary case the guard exists for: an element that is not a
        // byte refuses the whole call, so the good byte beside it is not written.
        let out_of_range = byte_array_from_slots(&[65, 300]);
        assert_eq!(trq_write_stream(1, out_of_range), -1);

        unsafe {
            crate::memory::trq_release(masquerading as *mut u8);
            crate::memory::trq_release(out_of_range as *mut u8);
        }
    }

    /// `اقرأ_مجرى` over a handle from `trq_file_open_read`.
    ///
    /// Driven from Rust for the reason the write test is: it predates `افتح_ملف`
    /// (#362). Kept as the unit-level pin for a path the cross-backend tests now
    /// reach through the opener.
    #[test]
    fn test_read_stream_reads_a_file_handle() {
        let test_path = "/tmp/tarqeem_test_read_stream_handle.txt";
        std::fs::write(test_path, "Aم\n").unwrap();

        let path = trq_string_new(test_path.as_ptr(), test_path.len() as i64);
        let handle = trq_file_open_read(path);
        assert!(handle > 0);
        // B15 again, from the reading side: a handle must never name a standard
        // stream, or descriptor 0 would sometimes mean a file.
        assert!(
            handle >= 3,
            "المعرِّف {handle} يزاحم مجرى قياسياً / handle collides with a standard stream"
        );

        // Bytes, not characters: «م» is two of them.
        let answer = trq_read_stream(handle, 4);
        assert_eq!(slots_of(answer), vec![65, 0xD9, 0x85, 10]);

        assert!(trq_file_close(handle));
        std::fs::remove_file(test_path).ok();
        crate::memory::trq_release(path as *mut u8);
        crate::memory::trq_release(answer as *mut u8);
    }

    /// The loop stops at EOF rather than at the count, and the next read answers
    /// nothing. This is what makes a short answer mean exactly one thing.
    #[test]
    fn test_read_stream_stops_at_end_of_file() {
        let test_path = "/tmp/tarqeem_test_read_stream_eof.txt";
        std::fs::write(test_path, "abc").unwrap();

        let path = trq_string_new(test_path.as_ptr(), test_path.len() as i64);
        let handle = trq_file_open_read(path);
        assert!(handle >= 3);

        // Ten asked for, three there.
        let first = trq_read_stream(handle, 10);
        assert_eq!(slots_of(first), vec![97, 98, 99]);

        // And nothing left, which is the same empty array a refusal answers.
        let second = trq_read_stream(handle, 10);
        assert_eq!(slots_of(second), Vec::<i64>::new());

        assert!(trq_file_close(handle));
        std::fs::remove_file(test_path).ok();
        crate::memory::trq_release(path as *mut u8);
        crate::memory::trq_release(first as *mut u8);
        crate::memory::trq_release(second as *mut u8);
    }

    /// More than `READ_CHUNK`, so the loop runs more than once.
    ///
    /// The chunk size is not observable in the answer, and this is the test that
    /// says so: a loop that returned after one chunk would answer 65536 here.
    #[test]
    fn test_read_stream_reads_past_one_chunk() {
        let test_path = "/tmp/tarqeem_test_read_stream_chunks.txt";
        let payload: Vec<u8> = (0..70_000u32).map(|i| (i % 251) as u8).collect();
        std::fs::write(test_path, &payload).unwrap();

        let path = trq_string_new(test_path.as_ptr(), test_path.len() as i64);
        let handle = trq_file_open_read(path);
        assert!(handle >= 3);

        let answer = trq_read_stream(handle, 70_000);
        let slots = slots_of(answer);
        assert_eq!(slots.len(), 70_000);
        // Spot-check either side of the boundary the chunk would have cut at.
        assert_eq!(slots[65_535], payload[65_535] as i64);
        assert_eq!(slots[65_536], payload[65_536] as i64);
        assert_eq!(slots[69_999], payload[69_999] as i64);

        assert!(trq_file_close(handle));
        std::fs::remove_file(test_path).ok();
        crate::memory::trq_release(path as *mut u8);
        crate::memory::trq_release(answer as *mut u8);
    }

    /// A writer, a handle never opened, a closed one, and the two output streams
    /// are all "nowhere to read from", so all of them answer the empty array —
    /// the mirror of `trq_write_stream` refusing a reader with `-1`.
    #[test]
    fn test_read_stream_refuses_a_stream_it_cannot_read() {
        let test_path = "/tmp/tarqeem_test_read_stream_refuses.txt";

        // A handle opened for writing is the wrong direction.
        let path = trq_string_new(test_path.as_ptr(), test_path.len() as i64);
        let writer = trq_file_open_write(path);
        assert!(writer >= 3);
        let from_writer = trq_read_stream(writer, 4);
        assert_eq!(slots_of(from_writer), Vec::<i64>::new());
        assert!(trq_file_close(writer));

        // And a closed one, which is now as absent as one never opened.
        let after_close = trq_read_stream(writer, 4);
        let never_opened = trq_read_stream(99999, 4);
        // stdout and stderr carry bytes the other way.
        let from_stdout = trq_read_stream(1, 4);
        let from_stderr = trq_read_stream(2, 4);
        let negative = trq_read_stream(-1, 4);

        for answer in [
            after_close,
            never_opened,
            from_stdout,
            from_stderr,
            negative,
        ] {
            assert_eq!(slots_of(answer), Vec::<i64>::new());
        }

        std::fs::remove_file(test_path).ok();
        crate::memory::trq_release(path as *mut u8);
        crate::memory::trq_release(from_writer as *mut u8);
        for answer in [
            after_close,
            never_opened,
            from_stdout,
            from_stderr,
            negative,
        ] {
            crate::memory::trq_release(answer as *mut u8);
        }
    }

    /// A non-positive count answers nothing **and reads nothing**, which the
    /// stream position proves: the bytes are still there for the next call.
    #[test]
    fn test_read_stream_of_nothing_consumes_nothing() {
        let test_path = "/tmp/tarqeem_test_read_stream_zero.txt";
        std::fs::write(test_path, "abc").unwrap();

        let path = trq_string_new(test_path.as_ptr(), test_path.len() as i64);
        let handle = trq_file_open_read(path);
        assert!(handle >= 3);

        let none = trq_read_stream(handle, 0);
        assert_eq!(slots_of(none), Vec::<i64>::new());
        let negative = trq_read_stream(handle, -5);
        assert_eq!(slots_of(negative), Vec::<i64>::new());

        // Nothing was taken, so everything is still readable.
        let all = trq_read_stream(handle, 3);
        assert_eq!(slots_of(all), vec![97, 98, 99]);

        assert!(trq_file_close(handle));
        std::fs::remove_file(test_path).ok();
        crate::memory::trq_release(path as *mut u8);
        crate::memory::trq_release(none as *mut u8);
        crate::memory::trq_release(negative as *mut u8);
        crate::memory::trq_release(all as *mut u8);
    }

    /// Reads back what `trq_read_stream` answered: one `i64` slot per byte.
    fn slots_of(arr: *mut TrqArray) -> Vec<i64> {
        assert!(!arr.is_null());
        unsafe {
            assert_eq!((*arr).elem_size, 8);
            if (*arr).len == 0 {
                return Vec::new();
            }
            std::slice::from_raw_parts((*arr).data as *const i64, (*arr).len as usize).to_vec()
        }
    }

    // ────────────────────────────────────────────────────────────────────
    // trq_path_status — حالة_مسار
    // ────────────────────────────────────────────────────────────────────

    /// Builds the `TrqString` the compiler would hand a path parameter.
    fn path_string(text: &str) -> *mut TrqString {
        trq_string_new(text.as_ptr(), text.len() as i64)
    }

    /// The two fields over a regular file.
    ///
    /// The content is Arabic on purpose: «مرحبا» is five characters and ten
    /// bytes, so an implementation that counted characters would pass this test
    /// with an ASCII fixture and fail it here.
    #[test]
    fn test_path_status_reads_a_file_kind_and_its_byte_size() {
        let test_path = "/tmp/tarqeem_test_path_status_file.txt";
        std::fs::write(test_path, "مرحبا").expect("تعذّر إنشاء الملف / could not create the file");

        let path = path_string(test_path);
        assert_eq!(trq_path_status(path, STAT_FIELD_KIND), PATH_KIND_FILE);
        assert_eq!(trq_path_status(path, STAT_FIELD_SIZE), 10);

        std::fs::remove_file(test_path).ok();
        unsafe {
            crate::memory::trq_release(path as *mut u8);
        }
    }

    /// A directory answers its kind and **no** size.
    ///
    /// `trq_file_size` answers the OS `st_size` for the same path — 4096 on
    /// ext4, 64–96 on APFS — which is why this function does not: a number that
    /// changes with the filesystem cannot be asserted here or in a golden file.
    #[test]
    fn test_path_status_answers_a_directory_without_a_size() {
        let path = path_string("/tmp");
        assert_eq!(trq_path_status(path, STAT_FIELD_KIND), PATH_KIND_DIR);
        assert_eq!(trq_path_status(path, STAT_FIELD_SIZE), STAT_NO_ANSWER);
        unsafe {
            crate::memory::trq_release(path as *mut u8);
        }
    }

    /// An absent path, an empty name and a null pointer are one answer.
    #[test]
    fn test_path_status_reads_nothing_as_absent() {
        for name in ["/tmp/tarqeem_test_path_status_absent_xyz", ""] {
            let path = path_string(name);
            assert_eq!(
                trq_path_status(path, STAT_FIELD_KIND),
                PATH_KIND_ABSENT,
                "المسار «{name}» ليس معدوماً / path is not read as absent"
            );
            assert_eq!(trq_path_status(path, STAT_FIELD_SIZE), STAT_NO_ANSWER);
            unsafe {
                crate::memory::trq_release(path as *mut u8);
            }
        }

        assert_eq!(
            trq_path_status(std::ptr::null(), STAT_FIELD_KIND),
            PATH_KIND_ABSENT
        );
        assert_eq!(
            trq_path_status(std::ptr::null(), STAT_FIELD_SIZE),
            STAT_NO_ANSWER
        );
    }

    /// A field this function does not know has no answer, whatever the path
    /// holds — `/tmp` exists and is readable, and every one of these is `-1`.
    #[test]
    fn test_path_status_has_no_answer_for_an_unknown_field() {
        let path = path_string("/tmp");
        for field in [2, 9, -1, i64::MIN, i64::MAX] {
            assert_eq!(
                trq_path_status(path, field),
                STAT_NO_ANSWER,
                "الحقل {field} أجاب بغير -١ / unknown field answered something"
            );
        }
        unsafe {
            crate::memory::trq_release(path as *mut u8);
        }
    }

    /// The fourth kind, and the reason it exists: `/dev/null` **exists** and is
    /// **not** a file, so `trq_file_exists` and `trq_file_is_file` disagree
    /// about it. A three-value kind could not fold both names.
    #[test]
    #[cfg(unix)]
    fn test_path_status_marks_a_device_as_neither_file_nor_directory() {
        let path = path_string("/dev/null");
        assert!(trq_file_exists(path), "الجهاز غير موجود / device is absent");
        assert!(!trq_file_is_file(path));
        assert!(!trq_file_is_dir(path));

        assert_eq!(trq_path_status(path, STAT_FIELD_KIND), PATH_KIND_OTHER);
        assert_eq!(trq_path_status(path, STAT_FIELD_SIZE), STAT_NO_ANSWER);
        unsafe {
            crate::memory::trq_release(path as *mut u8);
        }
    }

    /// Symlinks are followed, so a link to a file reads as a file — and a link
    /// whose target is gone reads as **absent**: the link is there, nothing is
    /// at the path it names.
    #[test]
    #[cfg(unix)]
    fn test_path_status_follows_a_symlink_and_reads_a_broken_one_as_absent() {
        let target = "/tmp/tarqeem_test_path_status_symlink_target.txt";
        let link = "/tmp/tarqeem_test_path_status_symlink";
        std::fs::remove_file(link).ok();
        std::fs::write(target, "ab").expect("تعذّر إنشاء الملف / could not create the file");
        std::os::unix::fs::symlink(target, link).expect("تعذّر إنشاء الوصلة / could not link");

        let path = path_string(link);
        assert_eq!(trq_path_status(path, STAT_FIELD_KIND), PATH_KIND_FILE);
        assert_eq!(trq_path_status(path, STAT_FIELD_SIZE), 2);

        std::fs::remove_file(target).ok();
        assert_eq!(trq_path_status(path, STAT_FIELD_KIND), PATH_KIND_ABSENT);
        assert_eq!(trq_path_status(path, STAT_FIELD_SIZE), STAT_NO_ANSWER);

        std::fs::remove_file(link).ok();
        unsafe {
            crate::memory::trq_release(path as *mut u8);
        }
    }

    // ────────────────────────────────────────────────────────────────────
    // trq_path_delete — احذف_مسار
    // ────────────────────────────────────────────────────────────────────

    /// Frees a path built by [`path_string`].
    ///
    /// Deliberately without the `unsafe` block that the neighbouring tests wrap
    /// this call in: `trq_release` is a safe function, so the block is a lint
    /// (#310 tracks the 55 others) and new code should not add to the sweep.
    fn release_path(path: *mut TrqString) {
        crate::memory::trq_release(path as *mut u8);
    }

    #[test]
    fn test_path_delete_removes_a_regular_file() {
        let target = "/tmp/tarqeem_test_path_delete_file.txt";
        std::fs::write(target, "مرحبا").expect("تعذّر إنشاء الملف / could not create the file");

        let path = path_string(target);
        assert!(trq_path_delete(path));
        assert!(!std::path::Path::new(target).exists());

        release_path(path);
    }

    #[test]
    fn test_path_delete_removes_an_empty_directory() {
        let target = "/tmp/tarqeem_test_path_delete_empty_dir";
        std::fs::remove_dir_all(target).ok();
        std::fs::create_dir(target).expect("تعذّر إنشاء المجلد / could not create the directory");

        let path = path_string(target);
        assert!(trq_path_delete(path));
        assert!(!std::path::Path::new(target).exists());

        release_path(path);
    }

    /// `rmdir`, not `rm -r`: a directory with anything in it survives, which is
    /// what keeps `احذف_مجلد`'s contract when it becomes a wrapper.
    #[test]
    fn test_path_delete_refuses_a_non_empty_directory() {
        let dir = "/tmp/tarqeem_test_path_delete_full_dir";
        std::fs::remove_dir_all(dir).ok();
        std::fs::create_dir(dir).expect("تعذّر إنشاء المجلد / could not create the directory");
        std::fs::write(format!("{dir}/ساكن.نص"), "x").expect("تعذّر إنشاء الملف");

        let path = path_string(dir);
        assert!(!trq_path_delete(path));
        assert!(std::path::Path::new(dir).exists());

        release_path(path);
        std::fs::remove_dir_all(dir).ok();
    }

    /// An absent path, an empty name and a null pointer are one answer.
    #[test]
    fn test_path_delete_removes_nothing_that_is_not_there() {
        for name in ["/tmp/tarqeem_test_path_delete_absent_xyz", ""] {
            let path = path_string(name);
            assert!(!trq_path_delete(path), "حذف ما ليس موجوداً: {name:?}");
            release_path(path);
        }

        assert!(!trq_path_delete(std::ptr::null()));
    }

    /// The lstat decision, and the row that makes it load-bearing. Following the
    /// link would call `remove_dir` on it and fail; the link is unlinked instead,
    /// and the directory it named survives.
    #[test]
    #[cfg(unix)]
    fn test_path_delete_unlinks_a_symlink_to_a_directory_and_spares_its_target() {
        let target = "/tmp/tarqeem_test_path_delete_link_target_dir";
        let link = "/tmp/tarqeem_test_path_delete_link_to_dir";
        std::fs::remove_file(link).ok();
        std::fs::remove_dir_all(target).ok();
        std::fs::create_dir(target).expect("تعذّر إنشاء المجلد / could not create the directory");
        std::os::unix::fs::symlink(target, link).expect("تعذّر إنشاء الوصلة / could not link");

        // What a `stat`-based selector would have seen, and why it is wrong here.
        assert!(std::fs::metadata(link).expect("الوصلة تُتبَع").is_dir());

        let path = path_string(link);
        assert!(trq_path_delete(path));
        assert!(std::fs::symlink_metadata(link).is_err(), "الوصلة باقية");
        assert!(std::path::Path::new(target).is_dir(), "الهدف حُذف");

        release_path(path);
        std::fs::remove_dir_all(target).ok();
    }

    /// A broken link is removable — the row a `stat`-based selector could never
    /// reach, since [`trq_path_status`] reads it as absent.
    #[test]
    #[cfg(unix)]
    fn test_path_delete_unlinks_a_broken_symlink() {
        let link = "/tmp/tarqeem_test_path_delete_broken_link";
        std::fs::remove_file(link).ok();
        std::os::unix::fs::symlink("/tmp/tarqeem_test_path_delete_never_existed", link)
            .expect("تعذّر إنشاء الوصلة / could not link");

        let path = path_string(link);
        assert_eq!(trq_path_status(path, STAT_FIELD_KIND), PATH_KIND_ABSENT);
        assert!(trq_path_delete(path));
        assert!(std::fs::symlink_metadata(link).is_err(), "الوصلة باقية");

        release_path(path);
    }

    /// Builds the `مصفوفة<عدد>` the compiler would hand this primitive: one raw
    /// `i64` slot per element, `elem_size` 8.
    fn byte_array(bytes: &[u8]) -> *const TrqArray {
        let slots: Vec<i64> = bytes.iter().map(|b| *b as i64).collect();
        byte_array_from_slots(&slots)
    }

    /// The same, for slots that are deliberately not bytes.
    fn byte_array_from_slots(slots: &[i64]) -> *const TrqArray {
        let arr = crate::array::trq_array_new(slots.len() as i64, 8);
        assert!(!arr.is_null());
        unsafe {
            let data = (*arr).data as *mut i64;
            for (i, slot) in slots.iter().enumerate() {
                *data.add(i) = *slot;
            }
        }
        arr
    }

    // ────────────────────────────────────────────────────────────────────
    // trq_file_open — افتح_ملف
    // ────────────────────────────────────────────────────────────────────

    /// Every handle is past the console streams, so a program may hand the
    /// answer straight to `اكتب_مجرى`/`اقرأ_مجرى`. Asserted as `>= 3` and never
    /// `== 3`: `NEXT_FILE_HANDLE` is process-global, so the value depends on how
    /// many handles other tests in this binary have taken.
    #[test]
    fn test_file_open_answers_a_handle_past_the_console_streams() {
        let target = "/tmp/tarqeem_test_file_open_read.txt";
        std::fs::write(target, "مرحبا").expect("تعذّر إنشاء الملف / could not create the file");

        let path = path_string(target);
        let handle = trq_file_open(path, OPEN_READ);
        assert!(
            handle >= 3,
            "المعرِّف {handle} يزاحم مجرى قياسياً / handle collides with a standard stream"
        );

        assert!(trq_file_close(handle));
        std::fs::remove_file(target).ok();
        release_path(path);
    }

    /// Write mode creates the file, and that is observable with no flush and no
    /// close — `File::create` reaches the filesystem before any byte does. It is
    /// what lets `حالة_مسار` see a file the program just opened.
    #[test]
    fn test_file_open_in_write_mode_creates_the_file() {
        let target = "/tmp/tarqeem_test_file_open_creates.txt";
        std::fs::remove_file(target).ok();

        let path = path_string(target);
        let handle = trq_file_open(path, OPEN_WRITE);
        assert!(handle >= 3);
        assert_eq!(trq_path_status(path, STAT_FIELD_KIND), PATH_KIND_FILE);

        assert!(trq_file_close(handle));
        std::fs::remove_file(target).ok();
        release_path(path);
    }

    /// Append keeps what was there; write truncates it. The two modes differ in
    /// exactly this, so one test covers both.
    #[test]
    fn test_file_open_appends_where_write_truncates() {
        let target = "/tmp/tarqeem_test_file_open_append.txt";
        std::fs::write(target, "أ").expect("تعذّر إنشاء الملف / could not create the file");

        let path = path_string(target);
        let appender = trq_file_open(path, OPEN_APPEND);
        assert!(appender >= 3);
        assert_eq!(trq_write_stream(appender, byte_array(b"B")), 1);
        assert!(trq_file_close(appender));
        assert_eq!(std::fs::read(target).unwrap(), "أB".as_bytes());

        let truncater = trq_file_open(path, OPEN_WRITE);
        assert!(truncater >= 3);
        assert!(trq_file_close(truncater));
        assert!(std::fs::read(target).unwrap().is_empty());

        std::fs::remove_file(target).ok();
        release_path(path);
    }

    /// A mode this function does not know is refused **before** the path, so a
    /// bad mode creates nothing. `3` is `وضع_قراءة_كتابة` in
    /// `stdlib/ملفات/ملف.ترقيم`, which has no handle kind here — the row that
    /// makes this test more than a range check.
    #[test]
    fn test_file_open_refuses_an_unknown_mode_without_touching_the_path() {
        let target = "/tmp/tarqeem_test_file_open_bad_mode.txt";
        std::fs::remove_file(target).ok();

        let path = path_string(target);
        for mode in [3, 9, -1, i64::MIN, i64::MAX] {
            assert_eq!(trq_file_open(path, mode), OPEN_FAILED, "الوضع {mode}");
            assert!(
                !std::path::Path::new(target).exists(),
                "الوضع {mode} أنشأ ملفاً / mode created a file"
            );
        }

        release_path(path);
    }

    /// `-1`, never `0`: `0` names stdin, so a failed open answering it would send
    /// a later `اقرأ_مجرى` to the keyboard. An absent path, an empty name and a
    /// null pointer are one answer.
    #[test]
    fn test_file_open_answers_minus_one_for_nothing_to_open() {
        let absent = path_string("/tmp/tarqeem_test_file_open_absent_dir/nothing.txt");
        assert_eq!(trq_file_open(absent, OPEN_READ), OPEN_FAILED);
        release_path(absent);

        let empty = path_string("");
        assert_eq!(trq_file_open(empty, OPEN_READ), OPEN_FAILED);
        release_path(empty);

        assert_eq!(trq_file_open(std::ptr::null(), OPEN_READ), OPEN_FAILED);
    }

    /// A directory is refused in every mode, which is a deliberate deviation from
    /// `open(2)`: POSIX opens a directory read-only and Windows does not, so a
    /// literal reading would answer a handle on one platform and `-1` on the other
    /// in a contract row. The handle is useless either way.
    ///
    /// Also asserts the handle is not leaked — the refusal closes what it opened,
    /// so a following open answers the *next* id rather than skipping two.
    #[test]
    fn test_file_open_refuses_a_directory_in_every_mode() {
        let target = "/tmp/tarqeem_test_file_open_dir";
        std::fs::remove_dir_all(target).ok();
        std::fs::create_dir(target).expect("تعذّر إنشاء المجلد / could not create the directory");

        let path = path_string(target);
        for mode in [OPEN_READ, OPEN_WRITE, OPEN_APPEND] {
            assert_eq!(trq_file_open(path, mode), OPEN_FAILED, "الوضع {mode}");
        }
        assert!(
            std::path::Path::new(target).is_dir(),
            "المجلد تغيّر / the directory was disturbed"
        );

        std::fs::remove_dir_all(target).ok();
        release_path(path);
    }

    /// Two opens are two handles. They must differ, or a program holding both
    /// would write through the one it meant to read.
    #[test]
    fn test_file_open_hands_out_distinct_handles() {
        let target = "/tmp/tarqeem_test_file_open_distinct.txt";
        std::fs::write(target, "مرحبا").expect("تعذّر إنشاء الملف / could not create the file");

        let path = path_string(target);
        let first = trq_file_open(path, OPEN_READ);
        let second = trq_file_open(path, OPEN_READ);
        assert!(first >= 3 && second >= 3);
        assert_ne!(first, second);

        assert!(trq_file_close(first));
        assert!(trq_file_close(second));
        std::fs::remove_file(target).ok();
        release_path(path);
    }

    /// A handle carries a direction, and the stream pair honours it: writing to a
    /// reader fails and reading a writer answers nothing. Both refusals already
    /// existed for a handle that was never opened; this is the first time a
    /// *live* handle can be the wrong kind.
    #[test]
    fn test_file_open_handles_carry_their_direction() {
        let target = "/tmp/tarqeem_test_file_open_direction.txt";
        std::fs::write(target, "مرحبا").expect("تعذّر إنشاء الملف / could not create the file");

        let path = path_string(target);
        let reader = trq_file_open(path, OPEN_READ);
        assert_eq!(trq_write_stream(reader, byte_array(b"x")), WRITE_FAILED);

        let writer = trq_file_open(path, OPEN_APPEND);
        let nothing = trq_read_stream(writer, 4);
        assert!(slots_of(nothing).is_empty());

        assert!(trq_file_close(reader));
        assert!(trq_file_close(writer));
        std::fs::remove_file(target).ok();
        release_path(path);
        crate::memory::trq_release(nothing as *mut u8);
    }

    /// The round trip the opener exists for: open, write bytes through
    /// `اكتب_مجرى`, open again, read them back through `اقرأ_مجرى`. Arabic
    /// content on purpose — two bytes per character, so a byte count and a
    /// character count cannot be confused for one another.
    #[test]
    fn test_file_open_round_trips_bytes_through_the_stream_pair() {
        let target = "/tmp/tarqeem_test_file_open_round_trip.txt";
        std::fs::remove_file(target).ok();

        let path = path_string(target);
        let writer = trq_file_open(path, OPEN_WRITE);
        assert_eq!(trq_write_stream(writer, byte_array("مرحبا".as_bytes())), 10);
        assert!(trq_file_close(writer));

        let reader = trq_file_open(path, OPEN_READ);
        let answer = trq_read_stream(reader, 64);
        let expected: Vec<i64> = "مرحبا".as_bytes().iter().map(|b| *b as i64).collect();
        assert_eq!(slots_of(answer), expected);

        assert!(trq_file_close(reader));
        std::fs::remove_file(target).ok();
        release_path(path);
        crate::memory::trq_release(answer as *mut u8);
    }

    /// The durability row, and the reason [`flush_open_writers`] exists.
    ///
    /// `trq_write_stream`'s handle path does not flush and this program closes
    /// nothing, so without this call the bytes sit in a `BufWriter` that no
    /// destructor runs — `main` is `extern "C"`. The empty read before it is what
    /// proves the flush is doing the work rather than the write having already
    /// landed. `اغلق_ملف` reaches the same bytes earlier; this is the path for
    /// handles a program never closes.
    #[test]
    fn test_flush_open_writers_lands_bytes_with_no_close() {
        let target = "/tmp/tarqeem_test_file_open_flush.txt";
        std::fs::remove_file(target).ok();

        let path = path_string(target);
        let writer = trq_file_open(path, OPEN_WRITE);
        assert_eq!(trq_write_stream(writer, byte_array(b"hi")), 2);
        assert!(
            std::fs::read(target).unwrap().is_empty(),
            "الكتابة بلغت الملف قبل الإفراغ"
        );

        flush_open_writers();
        assert_eq!(std::fs::read(target).unwrap(), b"hi");

        assert!(trq_file_close(writer));
        std::fs::remove_file(target).ok();
        release_path(path);
    }
    // -----------------------------------------------------------------------
    // trq_file_close — اغلق_ملف
    // -----------------------------------------------------------------------

    /// The row the name exists for: the bytes land on close, not at program end.
    ///
    /// The empty read before it is what makes the assertion about `اغلق_ملف`
    /// rather than about the write, exactly as
    /// `test_flush_open_writers_lands_bytes_with_no_close` does for the other
    /// flusher. Arabic content, so a byte count and a character count differ and
    /// a wrong unit cannot pass.
    #[test]
    fn test_file_close_lands_written_bytes_before_the_program_ends() {
        let target = "/tmp/tarqeem_test_file_close_lands.txt";
        std::fs::remove_file(target).ok();

        let path = path_string(target);
        let writer = trq_file_open(path, OPEN_WRITE);
        assert_eq!(trq_write_stream(writer, byte_array("مرحبا".as_bytes())), 10);
        assert!(
            std::fs::read(target).unwrap().is_empty(),
            "الكتابة بلغت الملف قبل الإغلاق"
        );

        assert!(trq_file_close(writer));
        assert_eq!(std::fs::read(target).unwrap(), "مرحبا".as_bytes());

        std::fs::remove_file(target).ok();
        release_path(path);
    }

    /// A reader has nothing to flush, so the answer is about the release alone.
    #[test]
    fn test_file_close_releases_a_reader() {
        let target = "/tmp/tarqeem_test_file_close_reader.txt";
        std::fs::write(target, "مرحبا").unwrap();

        let path = path_string(target);
        let reader = trq_file_open(path, OPEN_READ);
        assert!(trq_file_close(reader));

        std::fs::remove_file(target).ok();
        release_path(path);
    }

    /// The handle leaves the table, so a second close is a miss like any other.
    #[test]
    fn test_file_close_refuses_a_handle_it_already_released() {
        let target = "/tmp/tarqeem_test_file_close_twice.txt";
        std::fs::remove_file(target).ok();

        let path = path_string(target);
        let writer = trq_file_open(path, OPEN_WRITE);
        assert!(trq_file_close(writer));
        assert!(!trq_file_close(writer), "الإغلاق الثاني نجح");

        std::fs::remove_file(target).ok();
        release_path(path);
    }

    /// The console streams are not closable, and they need no arm of their own:
    /// `NEXT_FILE_HANDLE` starts at 3, so `٠`/`١`/`٢` were never in the table.
    /// This deviates from `close(2)`, which does close descriptor 1 — one
    /// documented behaviour rather than a platform-shaped one, the shape #362
    /// chose for the directory refusal.
    #[test]
    fn test_file_close_refuses_the_console_streams() {
        assert!(!trq_file_close(STREAM_STDIN));
        assert!(!trq_file_close(STREAM_STDOUT));
        assert!(!trq_file_close(STREAM_STDERR));
    }

    /// A negative handle names nothing. `99999` is pinned above, with the rest of
    /// the pre-`افتح_ملف` handle tests.
    ///
    /// Neither value is one any test could have opened, and that is deliberate
    /// rather than a bug being fixed: `FILE_HANDLES` is a `thread_local!` that
    /// every test in this binary shares under `--test-threads=1`, so a hardcoded
    /// plausible number like `٣` would answer `true` for any *future* test that
    /// leaves a handle stored. No test leaks one today — measured, every open that
    /// goes unclosed here is one that failed — so `٣` passes; it is simply not
    /// worth depending on test order for. Same reasoning as the debug
    /// interpreter's dispatch test, and the `٣` row is pinned per-process anyway,
    /// in the cross-backend suite.
    #[test]
    fn test_file_close_refuses_a_handle_never_opened() {
        assert!(!trq_file_close(-1));
        assert!(!trq_file_close(i64::MAX));
    }

    /// Closing frees the entry, never the number: the counter only ever counts
    /// up, so a program that prints a handle sees the same sequence on every
    /// backend.
    #[test]
    fn test_file_close_does_not_recycle_the_number() {
        let target = "/tmp/tarqeem_test_file_close_no_reuse.txt";
        std::fs::write(target, "م").unwrap();

        let path = path_string(target);
        let first = trq_file_open(path, OPEN_READ);
        assert!(trq_file_close(first));
        let second = trq_file_open(path, OPEN_READ);
        assert!(
            second > first,
            "أُعيد المعرِّف بعد انصرافه: {first} ثم {second}"
        );
        assert!(trq_file_close(second));

        std::fs::remove_file(target).ok();
        release_path(path);
    }

    /// The stream pair inherits the release with no arm of its own: a closed
    /// number is absent from the table, which both halves already refuse.
    #[test]
    fn test_the_stream_pair_refuses_a_closed_handle() {
        let target = "/tmp/tarqeem_test_file_close_then_stream.txt";
        std::fs::write(target, "مرحبا").unwrap();

        let path = path_string(target);
        let reader = trq_file_open(path, OPEN_READ);
        assert!(trq_file_close(reader));

        let answer = trq_read_stream(reader, 8);
        assert_eq!(unsafe { (*answer).len }, 0, "قُرئ من معرِّف مُغلق");
        crate::memory::trq_release(answer as *mut u8);

        let writer = trq_file_open(path, OPEN_APPEND);
        assert!(trq_file_close(writer));
        assert_eq!(trq_write_stream(writer, byte_array(b"x")), -1);

        std::fs::remove_file(target).ok();
        release_path(path);
    }
}
