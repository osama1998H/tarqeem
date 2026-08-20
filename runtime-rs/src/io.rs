//! I/O Operations for Tarqeem Runtime
//!
//! This module implements input/output functions for the Tarqeem language,
//! including console I/O, file operations, directory operations, and path utilities.

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

/// Close a file handle.
/// Returns true on success, false on error (invalid handle).
#[no_mangle]
pub extern "C" fn trq_file_close(handle: i64) -> bool {
    FILE_HANDLES.with(|handles| {
        let mut handles = handles.borrow_mut();
        if let Some(file_handle) = handles.remove(&handle) {
            // Flush writer before dropping
            if let FileHandle::Writer(mut writer) = file_handle {
                let _ = writer.flush();
            }
            true
        } else {
            false
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
            if out.write_all(&payload).is_err() {
                return WRITE_FAILED;
            }
            out.flush().ok();
            written
        }
        STREAM_STDERR => {
            let mut err = io::stderr();
            if err.write_all(&payload).is_err() {
                return WRITE_FAILED;
            }
            err.flush().ok();
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
    /// Driven from Rust because no Arabic name opens a handle yet — `افتح_ملف` is
    /// still ahead in Increment G — so this path is unreachable from Tarqeem
    /// source. It is implemented rather than stubbed, and covered here so the
    /// contract cannot shift under it when the opener lands.
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

        let bytes = byte_array(&[b'x']);

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
}
