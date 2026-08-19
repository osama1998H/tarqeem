//! Runtime initialization and utility functions for Tarqeem.
//!
//! This module provides runtime lifecycle management and debugging utilities.

use crate::types::TrqString;
use std::process;

// ============================================================================
// Runtime Initialization
// ============================================================================

/// Initialize the Tarqeem runtime.
///
/// This function should be called at the start of any Tarqeem program.
/// Currently performs minimal initialization, but provides a hook for
/// future runtime setup (thread pools, GC initialization, etc.).
#[no_mangle]
pub extern "C" fn trq_runtime_init() {
    // Currently a no-op, but provides a hook for future initialization:
    // - Thread pool initialization
    // - Memory allocator setup
    // - Signal handlers
    // - Locale/encoding setup for Arabic text

    // Ensure stdout is line-buffered for better interactivity
    // (This is the default on most systems, but we make it explicit)
}

/// Cleanup the Tarqeem runtime.
///
/// This function should be called at the end of any Tarqeem program.
/// Currently performs minimal cleanup, but provides a hook for
/// future runtime teardown.
#[no_mangle]
pub extern "C" fn trq_runtime_cleanup() {
    // Currently a no-op, but provides a hook for future cleanup:
    // - Flush all output streams
    // - Join thread pools
    // - Release global resources
    // - Report memory leaks in debug mode

    // Flush stdout and stderr to ensure all output is written
    use std::io::Write;
    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();
}

// ============================================================================
// Assertions and Panics
// ============================================================================

/// Assert a condition, panicking with a message if it fails.
///
/// # Safety
///
/// - `msg` must be a valid pointer to a `TrqString` or null.
#[no_mangle]
pub extern "C" fn trq_assert(condition: bool, msg: *const TrqString) {
    if !condition {
        let message = if msg.is_null() {
            "فشل التأكيد".to_string() // "Assertion failed" in Arabic
        } else {
            unsafe {
                let trq_str = &*msg;
                if trq_str.data.is_null() || trq_str.len <= 0 {
                    "فشل التأكيد".to_string()
                } else {
                    let slice =
                        std::slice::from_raw_parts(trq_str.data as *const u8, trq_str.len as usize);
                    String::from_utf8_lossy(slice).to_string()
                }
            }
        };

        eprintln!("خطأ تأكيد: {}", message);
        eprintln!("Assertion error: {}", message);
        process::exit(1);
    }
}

/// Panic with a message and terminate the program.
///
/// # Safety
///
/// - `msg` must be a valid pointer to a `TrqString` or null.
#[no_mangle]
pub extern "C" fn trq_panic(msg: *const TrqString) {
    let message = if msg.is_null() {
        "خطأ فادح".to_string() // "Fatal error" in Arabic
    } else {
        unsafe {
            let trq_str = &*msg;
            if trq_str.data.is_null() || trq_str.len <= 0 {
                "خطأ فادح".to_string()
            } else {
                let slice =
                    std::slice::from_raw_parts(trq_str.data as *const u8, trq_str.len as usize);
                String::from_utf8_lossy(slice).to_string()
            }
        }
    };

    eprintln!("خطأ فادح: {}", message);
    eprintln!("Panic: {}", message);
    process::exit(1);
}

/// Panic with a message and terminate the program (alias for trq_panic).
///
/// This is provided for semantic clarity when the intent is to abort
/// rather than panic.
///
/// # Safety
///
/// - `msg` must be a valid pointer to a `TrqString` or null.
#[no_mangle]
pub extern "C" fn trq_abort(msg: *const TrqString) {
    trq_panic(msg);
}

/// The exit status an `عدد` reduces to.
///
/// A POSIX status is eight bits wide while `عدد` is a signed 64-bit integer, so
/// the language has to say what the other 56 mean. Masking here rather than
/// handing the value to `exit(2)` is what keeps the answer the same everywhere:
/// POSIX truncates to the low byte anyway, but Windows keeps all 32 bits, so a
/// pass-through would make one program report two statuses depending on the
/// platform. `أنهِ_البرنامج(٣٠٠)` is 44 on both.
///
/// Shared with the interpreter's arm by contract, not by code — the two
/// implementations are one line each and both are pinned across all three
/// backends, so they cannot drift silently.
pub(crate) fn exit_status(status: i64) -> i32 {
    (status & 0xFF) as i32
}

/// Terminate the program with an explicit exit status and no message.
///
/// Backs the core builtin `أنهِ_البرنامج`. Distinct from `trq_panic` above,
/// which is always status 1 *and* writes to stderr: nothing in the language
/// could report a status of its own choosing before this, since all three
/// `process::exit` calls in the runtime are hardcoded to 1.
///
/// The flushes are load-bearing. `process::exit` runs no destructors, so a
/// buffered `print!` with no trailing newline would be dropped here while the
/// interpreter's own path still printed it — a cross-backend divergence in the
/// one direction stdout buffering can produce.
#[no_mangle]
pub extern "C" fn trq_exit(status: i64) -> ! {
    use std::io::Write;

    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();
    process::exit(exit_status(status));
}

/// Suspend the calling thread for `milliseconds`.
///
/// Backs the core builtin `نم`, which codegen has always emitted a call to
/// while nothing defined the symbol, so any program using it failed to link.
#[no_mangle]
pub extern "C" fn trq_sleep(milliseconds: i64) {
    if milliseconds > 0 {
        std::thread::sleep(std::time::Duration::from_millis(milliseconds as u64));
    }
}

/// Milliseconds since the UNIX epoch, or 0 if the clock predates it.
///
/// The single source of truth for both time builtins, so the value native code
/// sees cannot drift from `src/interpreter/executor/builtins.rs`.
fn epoch_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Backs the stdlib builtin `وقت_الآن` (`استورد { وقت_الآن } من "وقت"`).
///
/// Declared by codegen since before any definition existed, so every program
/// importing it failed to link (#241).
#[no_mangle]
pub extern "C" fn trq_time_now() -> i64 {
    epoch_millis()
}

/// Backs the stdlib builtin `وقت_أداء`. Returns the same epoch-millisecond
/// clock as `trq_time_now`, matching the interpreter, which treats the two
/// names identically.
#[no_mangle]
pub extern "C" fn trq_performance_now() -> i64 {
    epoch_millis()
}

// ============================================================================
// Debug Utilities
// ============================================================================

/// Print a debug message to stderr.
///
/// # Safety
///
/// - `msg` must be a valid pointer to a `TrqString` or null.
#[no_mangle]
pub extern "C" fn trq_debug(msg: *const TrqString) {
    if msg.is_null() {
        eprintln!("[تنقيح]");
        return;
    }

    unsafe {
        let trq_str = &*msg;
        if trq_str.data.is_null() || trq_str.len <= 0 {
            eprintln!("[تنقيح]");
        } else {
            let slice = std::slice::from_raw_parts(trq_str.data as *const u8, trq_str.len as usize);
            let s = String::from_utf8_lossy(slice);
            eprintln!("[تنقيح] {}", s);
        }
    }
}

/// Get the Tarqeem runtime version as a string.
///
/// Returns a pointer to a newly allocated TrqString containing the version.
/// The caller is responsible for freeing this string.
#[no_mangle]
pub extern "C" fn trq_version() -> *mut TrqString {
    use crate::string::trq_string_new;

    const VERSION: &str = "1.0.0";
    let bytes = VERSION.as_bytes();
    trq_string_new(bytes.as_ptr(), bytes.len() as i64)
}

// ============================================================================
// Environment
// ============================================================================

/// Get an environment variable by name.
///
/// Returns a pointer to a newly allocated TrqString containing the value,
/// or an empty string if the variable is not set.
///
/// # Safety
///
/// - `name` must be a valid pointer to a `TrqString` or null.
#[no_mangle]
pub extern "C" fn trq_env_get(name: *const TrqString) -> *mut TrqString {
    use crate::string::trq_string_new;

    if name.is_null() {
        return trq_string_new(std::ptr::null(), 0);
    }

    unsafe {
        let trq_str = &*name;
        if trq_str.data.is_null() || trq_str.len <= 0 {
            return trq_string_new(std::ptr::null(), 0);
        }

        let slice = std::slice::from_raw_parts(trq_str.data as *const u8, trq_str.len as usize);
        let key = match std::str::from_utf8(slice) {
            Ok(s) => s,
            Err(_) => return trq_string_new(std::ptr::null(), 0),
        };

        match std::env::var(key) {
            Ok(value) => {
                let bytes = value.as_bytes();
                trq_string_new(bytes.as_ptr(), bytes.len() as i64)
            }
            Err(_) => trq_string_new(std::ptr::null(), 0),
        }
    }
}

/// Set an environment variable.
///
/// # Safety
///
/// - `name` and `value` must be valid pointers to `TrqString` or null.
#[no_mangle]
pub extern "C" fn trq_env_set(name: *const TrqString, value: *const TrqString) -> bool {
    if name.is_null() {
        return false;
    }

    unsafe {
        let name_str = &*name;
        if name_str.data.is_null() || name_str.len <= 0 {
            return false;
        }

        let name_slice =
            std::slice::from_raw_parts(name_str.data as *const u8, name_str.len as usize);
        let key = match std::str::from_utf8(name_slice) {
            Ok(s) => s,
            Err(_) => return false,
        };

        let val = if value.is_null() {
            ""
        } else {
            let value_str = &*value;
            if value_str.data.is_null() || value_str.len <= 0 {
                ""
            } else {
                let value_slice =
                    std::slice::from_raw_parts(value_str.data as *const u8, value_str.len as usize);
                match std::str::from_utf8(value_slice) {
                    Ok(s) => s,
                    Err(_) => return false,
                }
            }
        };

        std::env::set_var(key, val);
        true
    }
}

/// Remove an environment variable.
///
/// # Safety
///
/// - `name` must be a valid pointer to a `TrqString` or null.
#[no_mangle]
pub extern "C" fn trq_env_remove(name: *const TrqString) -> bool {
    if name.is_null() {
        return false;
    }

    unsafe {
        let name_str = &*name;
        if name_str.data.is_null() || name_str.len <= 0 {
            return false;
        }

        let name_slice =
            std::slice::from_raw_parts(name_str.data as *const u8, name_str.len as usize);
        let key = match std::str::from_utf8(name_slice) {
            Ok(s) => s,
            Err(_) => return false,
        };

        std::env::remove_var(key);
        true
    }
}

// ============================================================================
// Program Entry Point
// ============================================================================

// The entry point is only included in non-test builds to avoid linking errors
// when running tests. The `__main__` symbol is provided by the compiled
// Tarqeem user code at final link time.
#[cfg(not(test))]
mod entry_point {
    use super::{trq_runtime_cleanup, trq_runtime_init};

    // The user's program entry point, generated by the Tarqeem compiler.
    // This is declared as an external C function that will be provided by
    // the compiled user code.
    extern "C" {
        fn __main__();
    }

    /// Main entry point wrapper.
    ///
    /// This is the actual `main` function that the linker requires for creating
    /// executables. It initializes the runtime, calls the user's `__main__`
    /// function (generated by the Tarqeem compiler), and performs cleanup.
    ///
    /// # Safety
    ///
    /// This function calls the external `__main__` function which must be
    /// provided by the compiled Tarqeem program.
    #[no_mangle]
    pub extern "C" fn main(_argc: i32, _argv: *const *const i8) -> i32 {
        trq_runtime_init();

        unsafe {
            __main__();
        }

        trq_runtime_cleanup();

        0
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::string::trq_string_new;

    #[test]
    fn test_runtime_init_cleanup() {
        // These should not panic
        trq_runtime_init();
        trq_runtime_cleanup();
    }

    #[test]
    fn test_assert_passing() {
        // Should not panic
        trq_assert(true, std::ptr::null());
    }

    #[test]
    fn test_version() {
        let version = trq_version();
        assert!(!version.is_null());

        unsafe {
            let v = &*version;
            assert!(v.len > 0);

            let slice = std::slice::from_raw_parts(v.data as *const u8, v.len as usize);
            let s = std::str::from_utf8(slice).unwrap();
            assert!(s.contains('.')); // Version should contain dots
        }
    }

    #[test]
    fn test_debug_null() {
        // Should not panic
        trq_debug(std::ptr::null());
    }

    #[test]
    fn test_debug_message() {
        let msg = "test message";
        let trq_msg = trq_string_new(msg.as_ptr(), msg.len() as i64);
        // Should not panic
        trq_debug(trq_msg);
    }

    #[test]
    fn test_env_get_nonexistent() {
        let name = "TARQEEM_NONEXISTENT_VAR_12345";
        let trq_name = trq_string_new(name.as_ptr(), name.len() as i64);
        let result = trq_env_get(trq_name);

        assert!(!result.is_null());
        unsafe {
            let r = &*result;
            assert_eq!(r.len, 0); // Empty string for non-existent var
        }
    }

    #[test]
    fn test_env_set_get() {
        let name = "TARQEEM_TEST_VAR";
        let value = "test_value";

        let trq_name = trq_string_new(name.as_ptr(), name.len() as i64);
        let trq_value = trq_string_new(value.as_ptr(), value.len() as i64);

        // Set the variable
        assert!(trq_env_set(trq_name, trq_value));

        // Get it back
        let result = trq_env_get(trq_name);
        assert!(!result.is_null());

        unsafe {
            let r = &*result;
            let slice = std::slice::from_raw_parts(r.data as *const u8, r.len as usize);
            let s = std::str::from_utf8(slice).unwrap();
            assert_eq!(s, value);
        }

        // Clean up
        trq_env_remove(trq_name);
    }

    #[test]
    fn test_env_remove() {
        let name = "TARQEEM_TEST_VAR_REMOVE";
        let value = "to_remove";

        let trq_name = trq_string_new(name.as_ptr(), name.len() as i64);
        let trq_value = trq_string_new(value.as_ptr(), value.len() as i64);

        // Set and then remove
        trq_env_set(trq_name, trq_value);
        assert!(trq_env_remove(trq_name));

        // Should be empty now
        let result = trq_env_get(trq_name);
        unsafe {
            let r = &*result;
            assert_eq!(r.len, 0);
        }
    }
    /// The masking half of `أنهِ_البرنامج`. `trq_exit` itself cannot be unit
    /// tested — it ends the process, which under `cargo test` is the whole test
    /// binary — so the arithmetic is factored out and the termination is covered
    /// end-to-end in `tests/builtins_execution_tests.rs` across all three
    /// backends.
    #[test]
    fn test_exit_status_masks_to_the_low_byte() {
        assert_eq!(exit_status(0), 0);
        assert_eq!(exit_status(3), 3);
        assert_eq!(exit_status(255), 255);
        // A complete byte's worth past the end wraps to zero, and one short of
        // zero is the top of the range — the same "describe what the platform
        // does" rule the shift range contract follows.
        assert_eq!(exit_status(256), 0);
        assert_eq!(exit_status(-1), 255);
        assert_eq!(exit_status(300), 44);
        // The extremes stay in range rather than overflowing the cast: masking
        // happens in i64 and only then narrows.
        assert_eq!(exit_status(i64::MIN), 0);
        assert_eq!(exit_status(i64::MAX), 255);
    }
}
