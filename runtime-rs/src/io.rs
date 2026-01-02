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
}
