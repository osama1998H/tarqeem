//! String operations for Tarqeem runtime
//!
//! This module implements UTF-8 string operations that are
//! ABI-compatible with the C runtime. All functions use the C calling
//! convention and can be linked with compiled Tarqeem programs.
//!
//! # Memory Model
//!
//! - TrqString struct is allocated via `trq_alloc` (reference-counted)
//! - The `data` buffer is allocated via `libc::malloc` (NOT reference-counted)
//! - String data is UTF-8 encoded and null-terminated

use crate::memory::trq_alloc;
use crate::types::TrqString;
use std::ffi::c_char;
use std::ptr;

// ============================================================================
// Helper Functions (internal)
// ============================================================================

/// Count UTF-8 characters (code points) in a byte slice
#[inline]
fn count_utf8_chars(bytes: &[u8]) -> i64 {
    bytes.iter().filter(|&&b| (b & 0xC0) != 0x80).count() as i64
}

/// Get the byte length of a UTF-8 character starting at the given byte
#[inline]
fn utf8_char_len(first_byte: u8) -> usize {
    if first_byte & 0x80 == 0 {
        1 // ASCII
    } else if first_byte & 0xE0 == 0xC0 {
        2 // 2-byte
    } else if first_byte & 0xF0 == 0xE0 {
        3 // 3-byte
    } else if first_byte & 0xF8 == 0xF0 {
        4 // 4-byte
    } else {
        1 // Invalid, treat as single byte
    }
}

/// Find the byte offset of the nth UTF-8 character
fn find_char_byte_offset(bytes: &[u8], char_index: i64) -> Option<usize> {
    if char_index < 0 {
        return None;
    }

    let mut byte_offset = 0;
    let mut char_count = 0;

    while byte_offset < bytes.len() && char_count < char_index {
        let char_len = utf8_char_len(bytes[byte_offset]);
        byte_offset += char_len;
        char_count += 1;
    }

    if char_count == char_index {
        Some(byte_offset)
    } else {
        None
    }
}

/// Check if a byte is whitespace
#[inline]
fn is_whitespace(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r')
}

/// Allocate a TrqString struct and its data buffer
///
/// # Safety
/// Returns null if allocation fails
unsafe fn allocate_string(len: i64) -> *mut TrqString {
    if len < 0 {
        return ptr::null_mut();
    }

    // Allocate the TrqString struct via trq_alloc (reference-counted)
    let str_ptr = trq_alloc(std::mem::size_of::<TrqString>() as i64) as *mut TrqString;
    if str_ptr.is_null() {
        return ptr::null_mut();
    }

    // Allocate data buffer via malloc (NOT reference-counted)
    let data_ptr = libc::malloc((len + 1) as usize) as *mut u8;
    if data_ptr.is_null() {
        // Free the struct since data allocation failed
        crate::memory::trq_free(str_ptr as *mut u8);
        return ptr::null_mut();
    }

    // Zero-initialize the data buffer
    ptr::write_bytes(data_ptr, 0, (len + 1) as usize);

    // Initialize the struct
    (*str_ptr).len = len;
    (*str_ptr).cap = len;
    (*str_ptr).data = data_ptr;

    str_ptr
}

// ============================================================================
// Core String Functions (High Priority)
// ============================================================================

/// Create a new string from raw bytes
///
/// # Arguments
/// * `data` - Pointer to UTF-8 data
/// * `len` - Length in bytes
///
/// # Returns
/// * Pointer to new TrqString, or NULL on failure
///
/// # C Equivalent
/// ```c
/// TrqString* trq_string_new(const char* data, int64_t len);
/// ```
#[no_mangle]
pub extern "C" fn trq_string_new(data: *const u8, len: i64) -> *mut TrqString {
    unsafe {
        let len = if len < 0 { 0 } else { len };

        let str_ptr = allocate_string(len);
        if str_ptr.is_null() {
            return ptr::null_mut();
        }

        // Copy data if provided
        if !data.is_null() && len > 0 {
            ptr::copy_nonoverlapping(data, (*str_ptr).data, len as usize);
        }

        // Null-terminate
        *(*str_ptr).data.add(len as usize) = 0;

        str_ptr
    }
}

/// Create a string from a null-terminated C string
///
/// # Arguments
/// * `cstr` - Pointer to null-terminated C string
///
/// # Returns
/// * Pointer to new TrqString, or empty string if input is NULL
///
/// # C Equivalent
/// ```c
/// TrqString* trq_string_from_cstr(const char* cstr);
/// ```
#[no_mangle]
pub extern "C" fn trq_string_from_cstr(cstr: *const c_char) -> *mut TrqString {
    unsafe {
        if cstr.is_null() {
            return trq_string_new(ptr::null(), 0);
        }

        let len = libc::strlen(cstr) as i64;
        trq_string_new(cstr as *const u8, len)
    }
}

/// Clone a string
///
/// # Arguments
/// * `s` - String to clone
///
/// # Returns
/// * Pointer to new TrqString, or NULL if input is NULL
///
/// # C Equivalent
/// ```c
/// TrqString* trq_string_clone(const TrqString* s);
/// ```
#[no_mangle]
pub extern "C" fn trq_string_clone(s: *const TrqString) -> *mut TrqString {
    unsafe {
        if s.is_null() || (*s).data.is_null() {
            return trq_string_new(ptr::null(), 0);
        }

        trq_string_new((*s).data, (*s).len)
    }
}

/// Free a string's data buffer (internal use)
///
/// Note: This only frees the data buffer, not the struct itself.
/// The struct is freed via trq_release when refcount reaches 0.
#[no_mangle]
pub extern "C" fn trq_string_free_data(s: *mut TrqString) {
    unsafe {
        if s.is_null() {
            return;
        }

        if !(*s).data.is_null() {
            libc::free((*s).data as *mut libc::c_void);
            (*s).data = ptr::null_mut();
        }

        (*s).len = 0;
        (*s).cap = 0;
    }
}

// ============================================================================
// Length & Comparison (High Priority)
// ============================================================================

/// Get the byte length of a string
///
/// # Arguments
/// * `s` - String to measure
///
/// # Returns
/// * Length in bytes, or 0 if NULL
///
/// # C Equivalent
/// ```c
/// int64_t trq_string_len(const TrqString* s);
/// ```
#[no_mangle]
pub extern "C" fn trq_string_len(s: *const TrqString) -> i64 {
    unsafe {
        if s.is_null() {
            return 0;
        }
        (*s).len
    }
}

/// Get the character count of a string (UTF-8 aware)
///
/// # Arguments
/// * `s` - String to measure
///
/// # Returns
/// * Character count (code points), or 0 if NULL
///
/// # C Equivalent
/// ```c
/// int64_t trq_string_len_chars(const TrqString* s);
/// ```
#[no_mangle]
pub extern "C" fn trq_string_len_chars(s: *const TrqString) -> i64 {
    unsafe {
        if s.is_null() || (*s).data.is_null() {
            return 0;
        }

        let bytes = std::slice::from_raw_parts((*s).data, (*s).len as usize);
        count_utf8_chars(bytes)
    }
}

/// Compare two strings
///
/// # Arguments
/// * `a` - First string
/// * `b` - Second string
///
/// # Returns
/// * < 0 if a < b, 0 if a == b, > 0 if a > b
///
/// # C Equivalent
/// ```c
/// int64_t trq_string_compare(const TrqString* a, const TrqString* b);
/// ```
#[no_mangle]
pub extern "C" fn trq_string_compare(a: *const TrqString, b: *const TrqString) -> i64 {
    unsafe {
        // Handle null cases
        if a.is_null() && b.is_null() {
            return 0;
        }
        if a.is_null() {
            return -1;
        }
        if b.is_null() {
            return 1;
        }

        let a_data = (*a).data;
        let b_data = (*b).data;

        if a_data.is_null() && b_data.is_null() {
            return 0;
        }
        if a_data.is_null() {
            return -1;
        }
        if b_data.is_null() {
            return 1;
        }

        let min_len = std::cmp::min((*a).len, (*b).len) as usize;
        let cmp = libc::memcmp(
            a_data as *const libc::c_void,
            b_data as *const libc::c_void,
            min_len,
        );

        if cmp != 0 {
            return cmp as i64;
        }

        // Content is equal up to min_len, compare by length
        (*a).len - (*b).len
    }
}

/// Check if two strings are equal
///
/// # Arguments
/// * `a` - First string
/// * `b` - Second string
///
/// # Returns
/// * true if equal, false otherwise
///
/// # C Equivalent
/// ```c
/// bool trq_string_equals(const TrqString* a, const TrqString* b);
/// ```
#[no_mangle]
pub extern "C" fn trq_string_equals(a: *const TrqString, b: *const TrqString) -> bool {
    unsafe {
        // Handle null cases
        if a.is_null() && b.is_null() {
            return true;
        }
        if a.is_null() || b.is_null() {
            return false;
        }

        // Fast path: different lengths
        if (*a).len != (*b).len {
            return false;
        }

        // Handle empty strings
        if (*a).len == 0 {
            return true;
        }

        if (*a).data.is_null() || (*b).data.is_null() {
            return (*a).data.is_null() && (*b).data.is_null();
        }

        // Compare bytes
        libc::memcmp(
            (*a).data as *const libc::c_void,
            (*b).data as *const libc::c_void,
            (*a).len as usize,
        ) == 0
    }
}

// ============================================================================
// Concatenation & Substring (High Priority)
// ============================================================================

/// Concatenate two strings
///
/// # Arguments
/// * `left` - First string
/// * `right` - Second string
///
/// # Returns
/// * New concatenated string, or clone of non-null input if one is null
///
/// # C Equivalent
/// ```c
/// TrqString* trq_string_concat(const TrqString* left, const TrqString* right);
/// ```
#[no_mangle]
pub extern "C" fn trq_string_concat(
    left: *const TrqString,
    right: *const TrqString,
) -> *mut TrqString {
    unsafe {
        // Handle null cases
        if left.is_null() || (*left).data.is_null() {
            return if right.is_null() {
                trq_string_new(ptr::null(), 0)
            } else {
                trq_string_clone(right)
            };
        }
        if right.is_null() || (*right).data.is_null() {
            return trq_string_clone(left);
        }

        let total_len = (*left).len + (*right).len;
        let result = allocate_string(total_len);
        if result.is_null() {
            return ptr::null_mut();
        }

        // Copy left string
        ptr::copy_nonoverlapping((*left).data, (*result).data, (*left).len as usize);

        // Copy right string
        ptr::copy_nonoverlapping(
            (*right).data,
            (*result).data.add((*left).len as usize),
            (*right).len as usize,
        );

        // Null-terminate
        *(*result).data.add(total_len as usize) = 0;

        result
    }
}

/// Extract a substring by byte indices
///
/// # Arguments
/// * `s` - Source string
/// * `start` - Start byte index
/// * `len` - Number of bytes
///
/// # Returns
/// * New substring, or empty string on invalid input
///
/// # C Equivalent
/// ```c
/// TrqString* trq_string_substr(const TrqString* s, int64_t start, int64_t len);
/// ```
#[no_mangle]
pub extern "C" fn trq_string_substr(s: *const TrqString, start: i64, len: i64) -> *mut TrqString {
    unsafe {
        if s.is_null() || (*s).data.is_null() {
            return trq_string_new(ptr::null(), 0);
        }

        let str_len = (*s).len;

        // Bounds checking
        let start = if start < 0 { 0 } else { start };
        if start >= str_len {
            return trq_string_new(ptr::null(), 0);
        }

        let len = if len < 0 { 0 } else { len };
        let end = std::cmp::min(start + len, str_len);
        let actual_len = end - start;

        if actual_len <= 0 {
            return trq_string_new(ptr::null(), 0);
        }

        trq_string_new((*s).data.add(start as usize), actual_len)
    }
}

/// Extract a substring by character indices (UTF-8 aware)
///
/// # Arguments
/// * `s` - Source string
/// * `start` - Start character index
/// * `len` - Number of characters
///
/// # Returns
/// * New substring, or empty string on invalid input
///
/// # C Equivalent
/// ```c
/// TrqString* trq_string_substr_chars(const TrqString* s, int64_t start, int64_t len);
/// ```
#[no_mangle]
pub extern "C" fn trq_string_substr_chars(
    s: *const TrqString,
    start: i64,
    len: i64,
) -> *mut TrqString {
    unsafe {
        if s.is_null() || (*s).data.is_null() {
            return trq_string_new(ptr::null(), 0);
        }

        if start < 0 || len <= 0 {
            return trq_string_new(ptr::null(), 0);
        }

        let bytes = std::slice::from_raw_parts((*s).data, (*s).len as usize);

        // Find start byte offset
        let start_byte = match find_char_byte_offset(bytes, start) {
            Some(offset) => offset,
            None => return trq_string_new(ptr::null(), 0),
        };

        // Find end byte offset (start + len characters)
        let remaining_bytes = &bytes[start_byte..];
        let end_byte_relative = match find_char_byte_offset(remaining_bytes, len) {
            Some(offset) => offset,
            None => remaining_bytes.len(), // Take all remaining bytes
        };

        let byte_len = end_byte_relative as i64;
        trq_string_new((*s).data.add(start_byte), byte_len)
    }
}

// ============================================================================
// Type Conversions (High Priority)
// ============================================================================

/// Convert an integer to a string
///
/// # Arguments
/// * `value` - Integer value
///
/// # Returns
/// * String representation of the integer
///
/// # C Equivalent
/// ```c
/// TrqString* trq_int_to_string(int64_t value);
/// ```
#[no_mangle]
pub extern "C" fn trq_int_to_string(value: i64) -> *mut TrqString {
    let s = value.to_string();
    trq_string_new(s.as_ptr(), s.len() as i64)
}

/// Convert a float to a string
///
/// # Arguments
/// * `value` - Float value
///
/// # Returns
/// * String representation of the float
///
/// # C Equivalent
/// ```c
/// TrqString* trq_float_to_string(double value);
/// ```
#[no_mangle]
pub extern "C" fn trq_float_to_string(value: f64) -> *mut TrqString {
    let s = if value.fract() == 0.0 && value.abs() < 1e15 {
        format!("{}", value as i64)
    } else {
        format!("{}", value)
    };
    trq_string_new(s.as_ptr(), s.len() as i64)
}

/// Convert a boolean to a string (Arabic: "صحيح" / "خاطئ")
///
/// # Arguments
/// * `value` - Boolean value
///
/// # Returns
/// * Arabic string "صحيح" for true, "خاطئ" for false
///
/// # C Equivalent
/// ```c
/// TrqString* trq_bool_to_string(bool value);
/// ```
#[no_mangle]
pub extern "C" fn trq_bool_to_string(value: bool) -> *mut TrqString {
    if value {
        // "صحيح" in UTF-8
        let s = "صحيح";
        trq_string_new(s.as_ptr(), s.len() as i64)
    } else {
        // "خاطئ" in UTF-8
        let s = "خاطئ";
        trq_string_new(s.as_ptr(), s.len() as i64)
    }
}

/// Parse a string to an integer
///
/// # Arguments
/// * `s` - String to parse
///
/// # Returns
/// * Parsed integer, or 0 on failure
///
/// # C Equivalent
/// ```c
/// int64_t trq_string_to_int(const TrqString* s);
/// ```
#[no_mangle]
pub extern "C" fn trq_string_to_int(s: *const TrqString) -> i64 {
    unsafe {
        if s.is_null() || (*s).data.is_null() || (*s).len == 0 {
            return 0;
        }

        let bytes = std::slice::from_raw_parts((*s).data, (*s).len as usize);
        let text = match std::str::from_utf8(bytes) {
            Ok(t) => t.trim(),
            Err(_) => return 0,
        };

        text.parse::<i64>().unwrap_or(0)
    }
}

/// Parse a string to a float
///
/// # Arguments
/// * `s` - String to parse
///
/// # Returns
/// * Parsed float, or 0.0 on failure
///
/// # C Equivalent
/// ```c
/// double trq_string_to_float(const TrqString* s);
/// ```
#[no_mangle]
pub extern "C" fn trq_string_to_float(s: *const TrqString) -> f64 {
    unsafe {
        if s.is_null() || (*s).data.is_null() || (*s).len == 0 {
            return 0.0;
        }

        let bytes = std::slice::from_raw_parts((*s).data, (*s).len as usize);
        let text = match std::str::from_utf8(bytes) {
            Ok(t) => t.trim(),
            Err(_) => return 0.0,
        };

        text.parse::<f64>().unwrap_or(0.0)
    }
}

// ============================================================================
// Search Operations (Medium Priority)
// ============================================================================

/// Check if a string contains a substring
///
/// # Arguments
/// * `s` - String to search in
/// * `substr` - Substring to search for
///
/// # Returns
/// * true if found, false otherwise
///
/// # C Equivalent
/// ```c
/// bool trq_string_contains(const TrqString* s, const TrqString* substr);
/// ```
#[no_mangle]
pub extern "C" fn trq_string_contains(s: *const TrqString, substr: *const TrqString) -> bool {
    trq_string_index_of(s, substr) >= 0
}

/// Check if a string starts with a prefix
///
/// # Arguments
/// * `s` - String to check
/// * `prefix` - Prefix to look for
///
/// # Returns
/// * true if starts with prefix, false otherwise
///
/// # C Equivalent
/// ```c
/// bool trq_string_starts_with(const TrqString* s, const TrqString* prefix);
/// ```
#[no_mangle]
pub extern "C" fn trq_string_starts_with(s: *const TrqString, prefix: *const TrqString) -> bool {
    unsafe {
        if s.is_null() || prefix.is_null() {
            return prefix.is_null();
        }

        if (*s).data.is_null() || (*prefix).data.is_null() {
            return (*prefix).data.is_null() || (*prefix).len == 0;
        }

        // Empty prefix matches everything
        if (*prefix).len == 0 {
            return true;
        }

        // Prefix longer than string
        if (*prefix).len > (*s).len {
            return false;
        }

        libc::memcmp(
            (*s).data as *const libc::c_void,
            (*prefix).data as *const libc::c_void,
            (*prefix).len as usize,
        ) == 0
    }
}

/// Check if a string ends with a suffix
///
/// # Arguments
/// * `s` - String to check
/// * `suffix` - Suffix to look for
///
/// # Returns
/// * true if ends with suffix, false otherwise
///
/// # C Equivalent
/// ```c
/// bool trq_string_ends_with(const TrqString* s, const TrqString* suffix);
/// ```
#[no_mangle]
pub extern "C" fn trq_string_ends_with(s: *const TrqString, suffix: *const TrqString) -> bool {
    unsafe {
        if s.is_null() || suffix.is_null() {
            return suffix.is_null();
        }

        if (*s).data.is_null() || (*suffix).data.is_null() {
            return (*suffix).data.is_null() || (*suffix).len == 0;
        }

        // Empty suffix matches everything
        if (*suffix).len == 0 {
            return true;
        }

        // Suffix longer than string
        if (*suffix).len > (*s).len {
            return false;
        }

        let offset = ((*s).len - (*suffix).len) as usize;
        libc::memcmp(
            (*s).data.add(offset) as *const libc::c_void,
            (*suffix).data as *const libc::c_void,
            (*suffix).len as usize,
        ) == 0
    }
}

/// Find the first occurrence of a substring
///
/// # Arguments
/// * `s` - String to search in
/// * `substr` - Substring to search for
///
/// # Returns
/// * Byte index of first occurrence, or -1 if not found
///
/// # C Equivalent
/// ```c
/// int64_t trq_string_index_of(const TrqString* s, const TrqString* substr);
/// ```
#[no_mangle]
pub extern "C" fn trq_string_index_of(s: *const TrqString, substr: *const TrqString) -> i64 {
    unsafe {
        if s.is_null() || substr.is_null() {
            return -1;
        }

        if (*s).data.is_null() || (*substr).data.is_null() {
            return if (*substr).data.is_null() || (*substr).len == 0 {
                0
            } else {
                -1
            };
        }

        // Empty substring found at position 0
        if (*substr).len == 0 {
            return 0;
        }

        // Substring longer than string
        if (*substr).len > (*s).len {
            return -1;
        }

        let s_bytes = std::slice::from_raw_parts((*s).data, (*s).len as usize);
        let sub_bytes = std::slice::from_raw_parts((*substr).data, (*substr).len as usize);

        // Linear search
        for i in 0..=(s_bytes.len() - sub_bytes.len()) {
            if &s_bytes[i..i + sub_bytes.len()] == sub_bytes {
                return i as i64;
            }
        }

        -1
    }
}

/// Find the last occurrence of a substring
///
/// # Arguments
/// * `s` - String to search in
/// * `substr` - Substring to search for
///
/// # Returns
/// * Byte index of last occurrence, or -1 if not found
///
/// # C Equivalent
/// ```c
/// int64_t trq_string_last_index_of(const TrqString* s, const TrqString* substr);
/// ```
#[no_mangle]
pub extern "C" fn trq_string_last_index_of(s: *const TrqString, substr: *const TrqString) -> i64 {
    unsafe {
        if s.is_null() || substr.is_null() {
            return -1;
        }

        if (*s).data.is_null() || (*substr).data.is_null() {
            return if (*substr).data.is_null() || (*substr).len == 0 {
                (*s).len
            } else {
                -1
            };
        }

        // Empty substring found at end
        if (*substr).len == 0 {
            return (*s).len;
        }

        // Substring longer than string
        if (*substr).len > (*s).len {
            return -1;
        }

        let s_bytes = std::slice::from_raw_parts((*s).data, (*s).len as usize);
        let sub_bytes = std::slice::from_raw_parts((*substr).data, (*substr).len as usize);

        // Reverse linear search
        for i in (0..=(s_bytes.len() - sub_bytes.len())).rev() {
            if &s_bytes[i..i + sub_bytes.len()] == sub_bytes {
                return i as i64;
            }
        }

        -1
    }
}

/// Count non-overlapping occurrences of a substring
///
/// # Arguments
/// * `s` - String to search in
/// * `substr` - Substring to count
///
/// # Returns
/// * Number of non-overlapping occurrences
///
/// # C Equivalent
/// ```c
/// int64_t trq_string_count(const TrqString* s, const TrqString* substr);
/// ```
#[no_mangle]
pub extern "C" fn trq_string_count(s: *const TrqString, substr: *const TrqString) -> i64 {
    unsafe {
        if s.is_null()
            || substr.is_null()
            || (*s).data.is_null()
            || (*substr).data.is_null()
            || (*substr).len == 0
        {
            return 0;
        }

        if (*substr).len > (*s).len {
            return 0;
        }

        let s_bytes = std::slice::from_raw_parts((*s).data, (*s).len as usize);
        let sub_bytes = std::slice::from_raw_parts((*substr).data, (*substr).len as usize);

        let mut count = 0;
        let mut pos = 0;

        while pos <= s_bytes.len() - sub_bytes.len() {
            if &s_bytes[pos..pos + sub_bytes.len()] == sub_bytes {
                count += 1;
                pos += sub_bytes.len(); // Non-overlapping
            } else {
                pos += 1;
            }
        }

        count
    }
}

// ============================================================================
// Transformation Operations (Medium Priority)
// ============================================================================

/// Convert string to uppercase (ASCII only)
///
/// # Arguments
/// * `s` - String to convert
///
/// # Returns
/// * New uppercase string
///
/// # C Equivalent
/// ```c
/// TrqString* trq_string_to_upper(const TrqString* s);
/// ```
#[no_mangle]
pub extern "C" fn trq_string_to_upper(s: *const TrqString) -> *mut TrqString {
    unsafe {
        if s.is_null() || (*s).data.is_null() {
            return trq_string_new(ptr::null(), 0);
        }

        let result = allocate_string((*s).len);
        if result.is_null() {
            return ptr::null_mut();
        }

        for i in 0..(*s).len as usize {
            let b = *(*s).data.add(i);
            *(*result).data.add(i) = if b >= b'a' && b <= b'z' {
                b - 32 // Convert to uppercase
            } else {
                b
            };
        }
        *(*result).data.add((*s).len as usize) = 0;

        result
    }
}

/// Convert string to lowercase (ASCII only)
///
/// # Arguments
/// * `s` - String to convert
///
/// # Returns
/// * New lowercase string
///
/// # C Equivalent
/// ```c
/// TrqString* trq_string_to_lower(const TrqString* s);
/// ```
#[no_mangle]
pub extern "C" fn trq_string_to_lower(s: *const TrqString) -> *mut TrqString {
    unsafe {
        if s.is_null() || (*s).data.is_null() {
            return trq_string_new(ptr::null(), 0);
        }

        let result = allocate_string((*s).len);
        if result.is_null() {
            return ptr::null_mut();
        }

        for i in 0..(*s).len as usize {
            let b = *(*s).data.add(i);
            *(*result).data.add(i) = if b >= b'A' && b <= b'Z' {
                b + 32 // Convert to lowercase
            } else {
                b
            };
        }
        *(*result).data.add((*s).len as usize) = 0;

        result
    }
}

/// Trim whitespace from both ends of a string
///
/// # Arguments
/// * `s` - String to trim
///
/// # Returns
/// * New trimmed string
///
/// # C Equivalent
/// ```c
/// TrqString* trq_string_trim(const TrqString* s);
/// ```
#[no_mangle]
pub extern "C" fn trq_string_trim(s: *const TrqString) -> *mut TrqString {
    unsafe {
        if s.is_null() || (*s).data.is_null() || (*s).len == 0 {
            return trq_string_new(ptr::null(), 0);
        }

        let bytes = std::slice::from_raw_parts((*s).data, (*s).len as usize);

        // Find start (first non-whitespace)
        let start = bytes
            .iter()
            .position(|&b| !is_whitespace(b))
            .unwrap_or(bytes.len());

        // Find end (last non-whitespace)
        let end = bytes
            .iter()
            .rposition(|&b| !is_whitespace(b))
            .map(|p| p + 1)
            .unwrap_or(0);

        if start >= end {
            return trq_string_new(ptr::null(), 0);
        }

        trq_string_new((*s).data.add(start), (end - start) as i64)
    }
}

/// Trim whitespace from the left side of a string
///
/// # Arguments
/// * `s` - String to trim
///
/// # Returns
/// * New left-trimmed string
///
/// # C Equivalent
/// ```c
/// TrqString* trq_string_trim_left(const TrqString* s);
/// ```
#[no_mangle]
pub extern "C" fn trq_string_trim_left(s: *const TrqString) -> *mut TrqString {
    unsafe {
        if s.is_null() || (*s).data.is_null() || (*s).len == 0 {
            return trq_string_new(ptr::null(), 0);
        }

        let bytes = std::slice::from_raw_parts((*s).data, (*s).len as usize);

        // Find start (first non-whitespace)
        let start = bytes
            .iter()
            .position(|&b| !is_whitespace(b))
            .unwrap_or(bytes.len());

        if start >= bytes.len() {
            return trq_string_new(ptr::null(), 0);
        }

        trq_string_new((*s).data.add(start), (bytes.len() - start) as i64)
    }
}

/// Trim whitespace from the right side of a string
///
/// # Arguments
/// * `s` - String to trim
///
/// # Returns
/// * New right-trimmed string
///
/// # C Equivalent
/// ```c
/// TrqString* trq_string_trim_right(const TrqString* s);
/// ```
#[no_mangle]
pub extern "C" fn trq_string_trim_right(s: *const TrqString) -> *mut TrqString {
    unsafe {
        if s.is_null() || (*s).data.is_null() || (*s).len == 0 {
            return trq_string_new(ptr::null(), 0);
        }

        let bytes = std::slice::from_raw_parts((*s).data, (*s).len as usize);

        // Find end (last non-whitespace)
        let end = bytes
            .iter()
            .rposition(|&b| !is_whitespace(b))
            .map(|p| p + 1)
            .unwrap_or(0);

        if end == 0 {
            return trq_string_new(ptr::null(), 0);
        }

        trq_string_new((*s).data, end as i64)
    }
}

/// Reverse a string (UTF-8 aware)
///
/// # Arguments
/// * `s` - String to reverse
///
/// # Returns
/// * New reversed string
///
/// # C Equivalent
/// ```c
/// TrqString* trq_string_reverse(const TrqString* s);
/// ```
#[no_mangle]
pub extern "C" fn trq_string_reverse(s: *const TrqString) -> *mut TrqString {
    unsafe {
        if s.is_null() || (*s).data.is_null() || (*s).len == 0 {
            return trq_string_new(ptr::null(), 0);
        }

        let bytes = std::slice::from_raw_parts((*s).data, (*s).len as usize);

        // Collect character boundaries
        let mut char_offsets: Vec<(usize, usize)> = Vec::new();
        let mut pos = 0;
        while pos < bytes.len() {
            let char_len = utf8_char_len(bytes[pos]);
            let end = std::cmp::min(pos + char_len, bytes.len());
            char_offsets.push((pos, end - pos));
            pos = end;
        }

        // Allocate result
        let result = allocate_string((*s).len);
        if result.is_null() {
            return ptr::null_mut();
        }

        // Copy characters in reverse order
        let mut dest_pos = 0;
        for &(offset, len) in char_offsets.iter().rev() {
            ptr::copy_nonoverlapping(
                bytes.as_ptr().add(offset),
                (*result).data.add(dest_pos),
                len,
            );
            dest_pos += len;
        }
        *(*result).data.add(dest_pos) = 0;

        result
    }
}

/// Convert string to title case (ASCII only)
///
/// # Arguments
/// * `s` - String to convert
///
/// # Returns
/// * New title-cased string
///
/// # C Equivalent
/// ```c
/// TrqString* trq_string_to_title(const TrqString* s);
/// ```
#[no_mangle]
pub extern "C" fn trq_string_to_title(s: *const TrqString) -> *mut TrqString {
    unsafe {
        if s.is_null() || (*s).data.is_null() {
            return trq_string_new(ptr::null(), 0);
        }

        let result = allocate_string((*s).len);
        if result.is_null() {
            return ptr::null_mut();
        }

        let mut capitalize_next = true;
        for i in 0..(*s).len as usize {
            let b = *(*s).data.add(i);

            if is_whitespace(b) {
                *(*result).data.add(i) = b;
                capitalize_next = true;
            } else if capitalize_next {
                *(*result).data.add(i) = if b >= b'a' && b <= b'z' {
                    b - 32 // Uppercase
                } else {
                    b
                };
                capitalize_next = false;
            } else {
                *(*result).data.add(i) = if b >= b'A' && b <= b'Z' {
                    b + 32 // Lowercase
                } else {
                    b
                };
            }
        }
        *(*result).data.add((*s).len as usize) = 0;

        result
    }
}

/// Repeat a string N times
///
/// # Arguments
/// * `s` - String to repeat
/// * `n` - Number of repetitions
///
/// # Returns
/// * New repeated string
///
/// # C Equivalent
/// ```c
/// TrqString* trq_string_repeat(const TrqString* s, int64_t n);
/// ```
#[no_mangle]
pub extern "C" fn trq_string_repeat(s: *const TrqString, n: i64) -> *mut TrqString {
    unsafe {
        if s.is_null() || (*s).data.is_null() || n <= 0 || (*s).len == 0 {
            return trq_string_new(ptr::null(), 0);
        }

        let total_len = (*s).len * n;
        let result = allocate_string(total_len);
        if result.is_null() {
            return ptr::null_mut();
        }

        for i in 0..n {
            ptr::copy_nonoverlapping(
                (*s).data,
                (*result).data.add((i * (*s).len) as usize),
                (*s).len as usize,
            );
        }
        *(*result).data.add(total_len as usize) = 0;

        result
    }
}

// ============================================================================
// Replace Operations (Medium Priority)
// ============================================================================

/// Replace the first occurrence of a substring
///
/// # Arguments
/// * `s` - String to modify
/// * `old_str` - Substring to replace
/// * `new_str` - Replacement string
///
/// # Returns
/// * New string with first occurrence replaced
///
/// # C Equivalent
/// ```c
/// TrqString* trq_string_replace(const TrqString* s, const TrqString* old_str, const TrqString* new_str);
/// ```
#[no_mangle]
pub extern "C" fn trq_string_replace(
    s: *const TrqString,
    old_str: *const TrqString,
    new_str: *const TrqString,
) -> *mut TrqString {
    unsafe {
        if s.is_null() || (*s).data.is_null() {
            return trq_string_new(ptr::null(), 0);
        }

        if old_str.is_null() || (*old_str).data.is_null() || (*old_str).len == 0 {
            return trq_string_clone(s);
        }

        let pos = trq_string_index_of(s, old_str);
        if pos < 0 {
            return trq_string_clone(s);
        }

        let new_len = if new_str.is_null() || (*new_str).data.is_null() {
            0
        } else {
            (*new_str).len
        };

        let result_len = (*s).len - (*old_str).len + new_len;
        let result = allocate_string(result_len);
        if result.is_null() {
            return ptr::null_mut();
        }

        // Copy before old_str
        if pos > 0 {
            ptr::copy_nonoverlapping((*s).data, (*result).data, pos as usize);
        }

        // Copy new_str
        if new_len > 0 {
            ptr::copy_nonoverlapping(
                (*new_str).data,
                (*result).data.add(pos as usize),
                new_len as usize,
            );
        }

        // Copy after old_str
        let after_pos = pos + (*old_str).len;
        let remaining = (*s).len - after_pos;
        if remaining > 0 {
            ptr::copy_nonoverlapping(
                (*s).data.add(after_pos as usize),
                (*result).data.add((pos + new_len) as usize),
                remaining as usize,
            );
        }

        *(*result).data.add(result_len as usize) = 0;
        result
    }
}

/// Replace all occurrences of a substring
///
/// # Arguments
/// * `s` - String to modify
/// * `old_str` - Substring to replace
/// * `new_str` - Replacement string
///
/// # Returns
/// * New string with all occurrences replaced
///
/// # C Equivalent
/// ```c
/// TrqString* trq_string_replace_all(const TrqString* s, const TrqString* old_str, const TrqString* new_str);
/// ```
#[no_mangle]
pub extern "C" fn trq_string_replace_all(
    s: *const TrqString,
    old_str: *const TrqString,
    new_str: *const TrqString,
) -> *mut TrqString {
    unsafe {
        if s.is_null() || (*s).data.is_null() {
            return trq_string_new(ptr::null(), 0);
        }

        if old_str.is_null() || (*old_str).data.is_null() || (*old_str).len == 0 {
            return trq_string_clone(s);
        }

        // Count occurrences
        let count = trq_string_count(s, old_str);
        if count == 0 {
            return trq_string_clone(s);
        }

        let new_len = if new_str.is_null() || (*new_str).data.is_null() {
            0
        } else {
            (*new_str).len
        };

        let result_len = (*s).len + count * (new_len - (*old_str).len);
        let result = allocate_string(result_len);
        if result.is_null() {
            return ptr::null_mut();
        }

        let s_bytes = std::slice::from_raw_parts((*s).data, (*s).len as usize);
        let old_bytes = std::slice::from_raw_parts((*old_str).data, (*old_str).len as usize);

        let mut src_pos = 0;
        let mut dest_pos = 0;

        while src_pos <= s_bytes.len() - old_bytes.len() {
            if &s_bytes[src_pos..src_pos + old_bytes.len()] == old_bytes {
                // Copy new_str
                if new_len > 0 {
                    ptr::copy_nonoverlapping(
                        (*new_str).data,
                        (*result).data.add(dest_pos),
                        new_len as usize,
                    );
                    dest_pos += new_len as usize;
                }
                src_pos += old_bytes.len();
            } else {
                *(*result).data.add(dest_pos) = s_bytes[src_pos];
                dest_pos += 1;
                src_pos += 1;
            }
        }

        // Copy remaining bytes
        while src_pos < s_bytes.len() {
            *(*result).data.add(dest_pos) = s_bytes[src_pos];
            dest_pos += 1;
            src_pos += 1;
        }

        *(*result).data.add(dest_pos) = 0;
        (*result).len = dest_pos as i64;

        result
    }
}

// ============================================================================
// Split & Join (Medium Priority)
// ============================================================================

/// Split a string by delimiter
///
/// # Arguments
/// * `s` - String to split
/// * `delim` - Delimiter string
///
/// # Returns
/// * Array of TrqString pointers
///
/// # C Equivalent
/// ```c
/// TrqArray* trq_string_split(const TrqString* s, const TrqString* delim);
/// ```
#[no_mangle]
pub extern "C" fn trq_string_split(
    s: *const TrqString,
    delim: *const TrqString,
) -> *mut crate::types::TrqArray {
    unsafe {
        // Create empty array
        let elem_size = std::mem::size_of::<*mut TrqString>() as i64;
        let arr = crate::array::trq_array_new(0, elem_size);
        if arr.is_null() {
            return ptr::null_mut();
        }

        if s.is_null() || (*s).data.is_null() || (*s).len == 0 {
            return arr;
        }

        // Empty delimiter: return whole string as one element
        if delim.is_null() || (*delim).data.is_null() || (*delim).len == 0 {
            let cloned = trq_string_clone(s);
            crate::array::trq_array_push(arr, &cloned as *const _ as *const u8, elem_size);
            return arr;
        }

        let s_bytes = std::slice::from_raw_parts((*s).data, (*s).len as usize);
        let delim_bytes = std::slice::from_raw_parts((*delim).data, (*delim).len as usize);

        let mut start = 0;
        let mut pos = 0;

        while pos <= s_bytes.len() - delim_bytes.len() {
            if &s_bytes[pos..pos + delim_bytes.len()] == delim_bytes {
                // Found delimiter, add substring
                let part = trq_string_new((*s).data.add(start), (pos - start) as i64);
                crate::array::trq_array_push(arr, &part as *const _ as *const u8, elem_size);
                pos += delim_bytes.len();
                start = pos;
            } else {
                pos += 1;
            }
        }

        // Add remaining part
        let part = trq_string_new((*s).data.add(start), (s_bytes.len() - start) as i64);
        crate::array::trq_array_push(arr, &part as *const _ as *const u8, elem_size);

        arr
    }
}

/// Join an array of strings with a delimiter
///
/// # Arguments
/// * `arr` - Array of TrqString pointers
/// * `delim` - Delimiter string
///
/// # Returns
/// * Joined string
///
/// # C Equivalent
/// ```c
/// TrqString* trq_string_join(const TrqArray* arr, const TrqString* delim);
/// ```
#[no_mangle]
pub extern "C" fn trq_string_join(
    arr: *const crate::types::TrqArray,
    delim: *const TrqString,
) -> *mut TrqString {
    unsafe {
        if arr.is_null() || (*arr).data.is_null() || (*arr).len == 0 {
            return trq_string_new(ptr::null(), 0);
        }

        let delim_len = if delim.is_null() || (*delim).data.is_null() {
            0
        } else {
            (*delim).len
        };

        // Calculate total length
        let elem_size = std::mem::size_of::<*mut TrqString>();
        let mut total_len: i64 = 0;

        for i in 0..(*arr).len {
            let str_ptr_ptr = (*arr).data.add((i as usize) * elem_size) as *const *mut TrqString;
            let str_ptr = *str_ptr_ptr;
            if !str_ptr.is_null() && !(*str_ptr).data.is_null() {
                total_len += (*str_ptr).len;
            }
            if i > 0 {
                total_len += delim_len;
            }
        }

        let result = allocate_string(total_len);
        if result.is_null() {
            return ptr::null_mut();
        }

        let mut dest_pos: usize = 0;
        for i in 0..(*arr).len {
            // Add delimiter before elements (except first)
            if i > 0 && delim_len > 0 {
                ptr::copy_nonoverlapping(
                    (*delim).data,
                    (*result).data.add(dest_pos),
                    delim_len as usize,
                );
                dest_pos += delim_len as usize;
            }

            let str_ptr_ptr = (*arr).data.add((i as usize) * elem_size) as *const *mut TrqString;
            let str_ptr = *str_ptr_ptr;
            if !str_ptr.is_null() && !(*str_ptr).data.is_null() && (*str_ptr).len > 0 {
                ptr::copy_nonoverlapping(
                    (*str_ptr).data,
                    (*result).data.add(dest_pos),
                    (*str_ptr).len as usize,
                );
                dest_pos += (*str_ptr).len as usize;
            }
        }

        *(*result).data.add(dest_pos) = 0;
        result
    }
}

// ============================================================================
// Padding (Lower Priority)
// ============================================================================

/// Pad string on the left to reach target character length
///
/// # Arguments
/// * `s` - String to pad
/// * `target_len` - Target character length
/// * `pad` - Padding string (uses first character, default: space)
///
/// # Returns
/// * New padded string
///
/// # C Equivalent
/// ```c
/// TrqString* trq_string_pad_left(const TrqString* s, int64_t target_len, const TrqString* pad);
/// ```
#[no_mangle]
pub extern "C" fn trq_string_pad_left(
    s: *const TrqString,
    target_len: i64,
    pad: *const TrqString,
) -> *mut TrqString {
    unsafe {
        if s.is_null() || (*s).data.is_null() {
            return trq_string_new(ptr::null(), 0);
        }

        let current_chars = trq_string_len_chars(s);
        if current_chars >= target_len {
            return trq_string_clone(s);
        }

        let pad_char = if pad.is_null() || (*pad).data.is_null() || (*pad).len == 0 {
            b' '
        } else {
            *(*pad).data
        };

        let pad_count = (target_len - current_chars) as usize;
        let new_len = (*s).len + pad_count as i64;
        let result = allocate_string(new_len);
        if result.is_null() {
            return ptr::null_mut();
        }

        // Fill padding
        ptr::write_bytes((*result).data, pad_char, pad_count);

        // Copy original string
        ptr::copy_nonoverlapping((*s).data, (*result).data.add(pad_count), (*s).len as usize);

        *(*result).data.add(new_len as usize) = 0;
        result
    }
}

/// Pad string on the right to reach target character length
///
/// # Arguments
/// * `s` - String to pad
/// * `target_len` - Target character length
/// * `pad` - Padding string (uses first character, default: space)
///
/// # Returns
/// * New padded string
///
/// # C Equivalent
/// ```c
/// TrqString* trq_string_pad_right(const TrqString* s, int64_t target_len, const TrqString* pad);
/// ```
#[no_mangle]
pub extern "C" fn trq_string_pad_right(
    s: *const TrqString,
    target_len: i64,
    pad: *const TrqString,
) -> *mut TrqString {
    unsafe {
        if s.is_null() || (*s).data.is_null() {
            return trq_string_new(ptr::null(), 0);
        }

        let current_chars = trq_string_len_chars(s);
        if current_chars >= target_len {
            return trq_string_clone(s);
        }

        let pad_char = if pad.is_null() || (*pad).data.is_null() || (*pad).len == 0 {
            b' '
        } else {
            *(*pad).data
        };

        let pad_count = (target_len - current_chars) as usize;
        let new_len = (*s).len + pad_count as i64;
        let result = allocate_string(new_len);
        if result.is_null() {
            return ptr::null_mut();
        }

        // Copy original string
        ptr::copy_nonoverlapping((*s).data, (*result).data, (*s).len as usize);

        // Fill padding
        ptr::write_bytes((*result).data.add((*s).len as usize), pad_char, pad_count);

        *(*result).data.add(new_len as usize) = 0;
        result
    }
}

// ============================================================================
// Classification (Lower Priority)
// ============================================================================

/// Check if string contains only ASCII digits
///
/// # Arguments
/// * `s` - String to check
///
/// # Returns
/// * true if all characters are '0'-'9'
///
/// # C Equivalent
/// ```c
/// bool trq_string_is_numeric(const TrqString* s);
/// ```
#[no_mangle]
pub extern "C" fn trq_string_is_numeric(s: *const TrqString) -> bool {
    unsafe {
        if s.is_null() || (*s).data.is_null() || (*s).len == 0 {
            return false;
        }

        let bytes = std::slice::from_raw_parts((*s).data, (*s).len as usize);
        bytes.iter().all(|&b| b >= b'0' && b <= b'9')
    }
}

/// Check if string contains only alphabetic characters
///
/// # Arguments
/// * `s` - String to check
///
/// # Returns
/// * true if all characters are ASCII A-Z or a-z (skips multi-byte)
///
/// # C Equivalent
/// ```c
/// bool trq_string_is_alpha(const TrqString* s);
/// ```
#[no_mangle]
pub extern "C" fn trq_string_is_alpha(s: *const TrqString) -> bool {
    unsafe {
        if s.is_null() || (*s).data.is_null() || (*s).len == 0 {
            return false;
        }

        let bytes = std::slice::from_raw_parts((*s).data, (*s).len as usize);

        // Check for ASCII alphabetic (A-Z, a-z) or allow multi-byte UTF-8
        bytes
            .iter()
            .all(|&b| (b >= b'A' && b <= b'Z') || (b >= b'a' && b <= b'z') || (b & 0x80) != 0)
    }
}

/// Check if string contains Arabic characters
///
/// # Arguments
/// * `s` - String to check
///
/// # Returns
/// * true if string contains Arabic characters
///
/// # C Equivalent
/// ```c
/// bool trq_string_is_arabic(const TrqString* s);
/// ```
#[no_mangle]
pub extern "C" fn trq_string_is_arabic(s: *const TrqString) -> bool {
    unsafe {
        if s.is_null() || (*s).data.is_null() || (*s).len == 0 {
            return false;
        }

        let bytes = std::slice::from_raw_parts((*s).data, (*s).len as usize);

        // Arabic in UTF-8 starts with bytes 0xD8-0xDB (U+0600-U+06FF range)
        // Also check for Arabic Extended (0xD9-0xDA)
        let mut i = 0;
        while i < bytes.len() {
            let b = bytes[i];
            if b >= 0xD8 && b <= 0xDB {
                return true;
            }
            // Skip UTF-8 continuation bytes
            if (b & 0xC0) == 0x80 {
                i += 1;
                continue;
            }
            // Skip multi-byte sequences
            i += utf8_char_len(b);
        }

        false
    }
}

// ============================================================================
// Character Access (Lower Priority)
// ============================================================================

/// Get character at index (UTF-8 aware)
///
/// # Arguments
/// * `s` - String
/// * `index` - Character index (0-based)
///
/// # Returns
/// * New string containing the character, or empty if out of bounds
///
/// # C Equivalent
/// ```c
/// TrqString* trq_string_char_at(const TrqString* s, int64_t index);
/// ```
#[no_mangle]
pub extern "C" fn trq_string_char_at(s: *const TrqString, index: i64) -> *mut TrqString {
    trq_string_substr_chars(s, index, 1)
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_new() {
        let data = b"hello";
        let s = trq_string_new(data.as_ptr(), 5);
        assert!(!s.is_null());
        unsafe {
            assert_eq!((*s).len, 5);
            crate::memory::trq_release(s as *mut u8);
        }
    }

    #[test]
    fn test_string_from_cstr() {
        let s = trq_string_from_cstr(b"hello\0".as_ptr() as *const c_char);
        assert!(!s.is_null());
        unsafe {
            assert_eq!((*s).len, 5);
            crate::memory::trq_release(s as *mut u8);
        }
    }

    #[test]
    fn test_string_len_chars_ascii() {
        let s = trq_string_from_cstr(b"hello\0".as_ptr() as *const c_char);
        assert_eq!(trq_string_len_chars(s), 5);
        unsafe {
            crate::memory::trq_release(s as *mut u8);
        }
    }

    #[test]
    fn test_string_len_chars_utf8() {
        // "مرحبا" is 5 Arabic characters, 10 bytes in UTF-8
        let arabic = "مرحبا";
        let s = trq_string_new(arabic.as_ptr(), arabic.len() as i64);
        assert_eq!(trq_string_len(s), 10);
        assert_eq!(trq_string_len_chars(s), 5);
        unsafe {
            crate::memory::trq_release(s as *mut u8);
        }
    }

    #[test]
    fn test_string_concat() {
        let a = trq_string_from_cstr(b"hello\0".as_ptr() as *const c_char);
        let b = trq_string_from_cstr(b" world\0".as_ptr() as *const c_char);
        let result = trq_string_concat(a, b);
        unsafe {
            assert_eq!((*result).len, 11);
            crate::memory::trq_release(a as *mut u8);
            crate::memory::trq_release(b as *mut u8);
            crate::memory::trq_release(result as *mut u8);
        }
    }

    #[test]
    fn test_string_compare() {
        let a = trq_string_from_cstr(b"abc\0".as_ptr() as *const c_char);
        let b = trq_string_from_cstr(b"abc\0".as_ptr() as *const c_char);
        let c = trq_string_from_cstr(b"abd\0".as_ptr() as *const c_char);

        assert_eq!(trq_string_compare(a, b), 0);
        assert!(trq_string_compare(a, c) < 0);
        assert!(trq_string_compare(c, a) > 0);

        unsafe {
            crate::memory::trq_release(a as *mut u8);
            crate::memory::trq_release(b as *mut u8);
            crate::memory::trq_release(c as *mut u8);
        }
    }

    #[test]
    fn test_string_equals() {
        let a = trq_string_from_cstr(b"hello\0".as_ptr() as *const c_char);
        let b = trq_string_from_cstr(b"hello\0".as_ptr() as *const c_char);
        let c = trq_string_from_cstr(b"world\0".as_ptr() as *const c_char);

        assert!(trq_string_equals(a, b));
        assert!(!trq_string_equals(a, c));

        unsafe {
            crate::memory::trq_release(a as *mut u8);
            crate::memory::trq_release(b as *mut u8);
            crate::memory::trq_release(c as *mut u8);
        }
    }

    #[test]
    fn test_string_contains() {
        let s = trq_string_from_cstr(b"hello world\0".as_ptr() as *const c_char);
        let sub = trq_string_from_cstr(b"world\0".as_ptr() as *const c_char);
        let not_sub = trq_string_from_cstr(b"foo\0".as_ptr() as *const c_char);

        assert!(trq_string_contains(s, sub));
        assert!(!trq_string_contains(s, not_sub));

        unsafe {
            crate::memory::trq_release(s as *mut u8);
            crate::memory::trq_release(sub as *mut u8);
            crate::memory::trq_release(not_sub as *mut u8);
        }
    }

    #[test]
    fn test_string_substr() {
        let s = trq_string_from_cstr(b"hello world\0".as_ptr() as *const c_char);
        let sub = trq_string_substr(s, 6, 5);

        unsafe {
            assert_eq!((*sub).len, 5);
            let bytes = std::slice::from_raw_parts((*sub).data, 5);
            assert_eq!(bytes, b"world");

            crate::memory::trq_release(s as *mut u8);
            crate::memory::trq_release(sub as *mut u8);
        }
    }

    #[test]
    fn test_string_int_conversion() {
        let s = trq_int_to_string(42);
        unsafe {
            let bytes = std::slice::from_raw_parts((*s).data, (*s).len as usize);
            assert_eq!(bytes, b"42");
            crate::memory::trq_release(s as *mut u8);
        }

        let s2 = trq_string_from_cstr(b"123\0".as_ptr() as *const c_char);
        assert_eq!(trq_string_to_int(s2), 123);
        unsafe {
            crate::memory::trq_release(s2 as *mut u8);
        }
    }

    #[test]
    fn test_string_bool_conversion() {
        let t = trq_bool_to_string(true);
        let f = trq_bool_to_string(false);

        unsafe {
            let t_bytes = std::slice::from_raw_parts((*t).data, (*t).len as usize);
            let f_bytes = std::slice::from_raw_parts((*f).data, (*f).len as usize);

            assert_eq!(t_bytes, "صحيح".as_bytes());
            assert_eq!(f_bytes, "خاطئ".as_bytes());

            crate::memory::trq_release(t as *mut u8);
            crate::memory::trq_release(f as *mut u8);
        }
    }

    #[test]
    fn test_string_trim() {
        let s = trq_string_from_cstr(b"  hello  \0".as_ptr() as *const c_char);
        let trimmed = trq_string_trim(s);

        unsafe {
            assert_eq!((*trimmed).len, 5);
            let bytes = std::slice::from_raw_parts((*trimmed).data, 5);
            assert_eq!(bytes, b"hello");

            crate::memory::trq_release(s as *mut u8);
            crate::memory::trq_release(trimmed as *mut u8);
        }
    }

    #[test]
    fn test_string_reverse() {
        let s = trq_string_from_cstr(b"abc\0".as_ptr() as *const c_char);
        let rev = trq_string_reverse(s);

        unsafe {
            let bytes = std::slice::from_raw_parts((*rev).data, (*rev).len as usize);
            assert_eq!(bytes, b"cba");

            crate::memory::trq_release(s as *mut u8);
            crate::memory::trq_release(rev as *mut u8);
        }
    }

    #[test]
    fn test_string_reverse_utf8() {
        // "أب" -> "بأ"
        let arabic = "أب";
        let s = trq_string_new(arabic.as_ptr(), arabic.len() as i64);
        let rev = trq_string_reverse(s);

        unsafe {
            let bytes = std::slice::from_raw_parts((*rev).data, (*rev).len as usize);
            assert_eq!(std::str::from_utf8(bytes).unwrap(), "بأ");

            crate::memory::trq_release(s as *mut u8);
            crate::memory::trq_release(rev as *mut u8);
        }
    }

    #[test]
    fn test_string_is_arabic() {
        let arabic = "مرحبا";
        let s = trq_string_new(arabic.as_ptr(), arabic.len() as i64);
        assert!(trq_string_is_arabic(s));
        unsafe {
            crate::memory::trq_release(s as *mut u8);
        }

        let english = trq_string_from_cstr(b"hello\0".as_ptr() as *const c_char);
        assert!(!trq_string_is_arabic(english));
        unsafe {
            crate::memory::trq_release(english as *mut u8);
        }
    }

    #[test]
    fn test_null_safety() {
        assert_eq!(trq_string_len(ptr::null()), 0);
        assert_eq!(trq_string_len_chars(ptr::null()), 0);
        assert_eq!(trq_string_compare(ptr::null(), ptr::null()), 0);
        assert!(trq_string_equals(ptr::null(), ptr::null()));
        assert!(!trq_string_contains(ptr::null(), ptr::null()));
    }

    // ===== Phase 8: Additional UTF-8/Arabic Tests =====

    #[test]
    fn test_arabic_with_diacritics() {
        // مَرْحَبًا with tashkeel (diacritical marks)
        let arabic_with_tashkeel = "مَرْحَبًا";
        let s = trq_string_new(
            arabic_with_tashkeel.as_ptr(),
            arabic_with_tashkeel.len() as i64,
        );

        // Byte length should match UTF-8 encoding
        assert_eq!(trq_string_len(s), arabic_with_tashkeel.len() as i64);

        // Character count should account for combining marks
        // Note: Each base letter + diacritical mark may count differently
        let char_count = trq_string_len_chars(s);
        assert!(char_count > 0);
        assert!(char_count <= arabic_with_tashkeel.chars().count() as i64);

        // Should be recognized as Arabic
        assert!(trq_string_is_arabic(s));

        unsafe {
            crate::memory::trq_release(s as *mut u8);
        }
    }

    #[test]
    fn test_arabic_numbers() {
        // Arabic-Indic numerals ٠١٢٣٤٥٦٧٨٩
        let arabic_nums = "٠١٢٣٤٥٦٧٨٩";
        let s = trq_string_new(arabic_nums.as_ptr(), arabic_nums.len() as i64);

        // Should have 10 characters
        assert_eq!(trq_string_len_chars(s), 10);

        // Test contains with Arabic zero
        let zero = "٠";
        let zero_s = trq_string_new(zero.as_ptr(), zero.len() as i64);
        assert!(trq_string_contains(s, zero_s));

        unsafe {
            crate::memory::trq_release(s as *mut u8);
            crate::memory::trq_release(zero_s as *mut u8);
        }
    }

    #[test]
    fn test_mixed_bidi_text() {
        // Bidirectional text: "Hello مرحبا World"
        let mixed = "Hello مرحبا World";
        let s = trq_string_new(mixed.as_ptr(), mixed.len() as i64);

        // trq_string_is_arabic returns true if string CONTAINS Arabic chars
        // Mixed content with Arabic should return true
        assert!(trq_string_is_arabic(s));

        // Should correctly handle length
        assert_eq!(trq_string_len(s), mixed.len() as i64);
        assert_eq!(trq_string_len_chars(s), mixed.chars().count() as i64);

        // Test contains with Arabic part
        let arabic_part = "مرحبا";
        let arabic_s = trq_string_new(arabic_part.as_ptr(), arabic_part.len() as i64);
        assert!(trq_string_contains(s, arabic_s));

        // Test contains with English part
        let english_part = trq_string_from_cstr(b"Hello\0".as_ptr() as *const c_char);
        assert!(trq_string_contains(s, english_part));

        unsafe {
            crate::memory::trq_release(s as *mut u8);
            crate::memory::trq_release(arabic_s as *mut u8);
            crate::memory::trq_release(english_part as *mut u8);
        }
    }

    #[test]
    fn test_empty_string_all_ops() {
        let empty = trq_string_from_cstr(b"\0".as_ptr() as *const c_char);

        // Length operations
        assert_eq!(trq_string_len(empty), 0);
        assert_eq!(trq_string_len_chars(empty), 0);

        // Contains on empty
        assert!(trq_string_contains(empty, empty)); // Empty contains empty
        let non_empty = trq_string_from_cstr(b"x\0".as_ptr() as *const c_char);
        assert!(!trq_string_contains(empty, non_empty)); // Empty doesn't contain non-empty
        assert!(trq_string_contains(non_empty, empty)); // Non-empty contains empty

        // Trim empty
        let trimmed = trq_string_trim(empty);
        assert_eq!(trq_string_len(trimmed), 0);

        // Reverse empty
        let reversed = trq_string_reverse(empty);
        assert_eq!(trq_string_len(reversed), 0);

        // Clone empty
        let cloned = trq_string_clone(empty);
        assert_eq!(trq_string_len(cloned), 0);
        assert!(trq_string_equals(empty, cloned));

        unsafe {
            crate::memory::trq_release(empty as *mut u8);
            crate::memory::trq_release(non_empty as *mut u8);
            crate::memory::trq_release(trimmed as *mut u8);
            crate::memory::trq_release(reversed as *mut u8);
            crate::memory::trq_release(cloned as *mut u8);
        }
    }

    #[test]
    fn test_string_concat_arabic() {
        let part1 = "مرحبا ";
        let part2 = "بالعالم";
        let s1 = trq_string_new(part1.as_ptr(), part1.len() as i64);
        let s2 = trq_string_new(part2.as_ptr(), part2.len() as i64);

        let concat = trq_string_concat(s1, s2);

        unsafe {
            let bytes = std::slice::from_raw_parts((*concat).data, (*concat).len as usize);
            let result = std::str::from_utf8(bytes).unwrap();
            assert_eq!(result, "مرحبا بالعالم");
        }

        unsafe {
            crate::memory::trq_release(s1 as *mut u8);
            crate::memory::trq_release(s2 as *mut u8);
            crate::memory::trq_release(concat as *mut u8);
        }
    }
}
