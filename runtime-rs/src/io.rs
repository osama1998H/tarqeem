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
#[no_mangle]
pub extern "C" fn trq_print_float(value: f64) {
    // Use %g style formatting (remove trailing zeros)
    if value.fract() == 0.0 && value.abs() < 1e15 {
        print!("{}", value as i64);
    } else {
        print!("{}", value);
    }
    io::stdout().flush().ok();
}

/// Print a boolean to stdout in Arabic.
/// Outputs "صحيح" for true, "خاطئ" for false.
#[no_mangle]
pub extern "C" fn trq_print_bool(value: bool) {
    if value {
        print!("صحيح");
    } else {
        print!("خاطئ");
    }
    io::stdout().flush().ok();
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
static NEXT_FILE_HANDLE: AtomicI64 = AtomicI64::new(1);

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
}
