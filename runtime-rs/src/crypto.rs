//! Cryptographic operations for Tarqeem runtime
//!
//! This module implements SHA-256 hashing and hex encoding/decoding
//! operations that are ABI-compatible with the C runtime.
//!
//! # Functions
//!
//! ## SHA-256 Hashing
//! - `trq_sha256_string` - Hash a string, return 64-char hex
//! - `trq_sha256_file` - Hash file contents
//! - `trq_sha256_bytes` - Hash a byte array
//! - `trq_sha256_compare` - Constant-time hash comparison
//!
//! ## Hex Encoding
//! - `trq_hex_encode` - Encode string to hex
//! - `trq_hex_decode` - Decode hex to string
//! - `trq_hex_encode_bytes` - Encode byte array to hex
//! - `trq_hex_decode_to_bytes` - Decode hex to byte array

use crate::memory::trq_alloc;
use crate::types::{TrqArray, TrqString};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{BufReader, Read};
use std::ptr;

// ============================================================================
// Helper Functions (internal)
// ============================================================================

/// Hex characters for encoding (lowercase)
const HEX_CHARS: &[u8; 16] = b"0123456789abcdef";

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

/// Allocate a TrqArray struct and its data buffer
///
/// # Safety
/// Returns null if allocation fails
unsafe fn allocate_array(len: i64, elem_size: i64) -> *mut TrqArray {
    if len < 0 || elem_size <= 0 {
        return ptr::null_mut();
    }

    // Allocate the TrqArray struct via trq_alloc (reference-counted)
    let arr_ptr = trq_alloc(std::mem::size_of::<TrqArray>() as i64) as *mut TrqArray;
    if arr_ptr.is_null() {
        return ptr::null_mut();
    }

    // Allocate data buffer via malloc (NOT reference-counted)
    let data_size = len * elem_size;
    let data_ptr = libc::malloc(data_size as usize) as *mut u8;
    if data_ptr.is_null() {
        // Free the struct since data allocation failed
        crate::memory::trq_free(arr_ptr as *mut u8);
        return ptr::null_mut();
    }

    // Zero-initialize the data buffer
    ptr::write_bytes(data_ptr, 0, data_size as usize);

    // Initialize the struct
    (*arr_ptr).len = len;
    (*arr_ptr).cap = len;
    (*arr_ptr).elem_size = elem_size;
    (*arr_ptr).data = data_ptr;

    arr_ptr
}

/// Convert a hash digest to a 64-character hex string
unsafe fn hash_to_hex_string(hash: &[u8; 32]) -> *mut TrqString {
    // 32 bytes -> 64 hex characters
    let str_ptr = allocate_string(64);
    if str_ptr.is_null() {
        return ptr::null_mut();
    }

    let data = (*str_ptr).data;
    for (i, byte) in hash.iter().enumerate() {
        *data.add(i * 2) = HEX_CHARS[(byte >> 4) as usize];
        *data.add(i * 2 + 1) = HEX_CHARS[(byte & 0x0F) as usize];
    }
    *data.add(64) = 0; // Null-terminate

    str_ptr
}

/// Convert a hex character to its value (0-15), or return -1 for invalid
#[inline]
fn hex_char_to_value(c: u8) -> i8 {
    match c {
        b'0'..=b'9' => (c - b'0') as i8,
        b'a'..=b'f' => (c - b'a' + 10) as i8,
        b'A'..=b'F' => (c - b'A' + 10) as i8,
        _ => -1,
    }
}

// ============================================================================
// SHA-256 Functions
// ============================================================================

/// Compute SHA-256 hash of a string
///
/// # Arguments
/// * `s` - Pointer to TrqString to hash
///
/// # Returns
/// * Pointer to new TrqString containing 64-character lowercase hex hash
/// * Returns empty string on error
///
/// # C Equivalent
/// ```c
/// TrqString* trq_sha256_string(const TrqString* s);
/// ```
#[no_mangle]
pub extern "C" fn trq_sha256_string(s: *const TrqString) -> *mut TrqString {
    unsafe {
        if s.is_null() || (*s).data.is_null() {
            return allocate_string(0);
        }

        let data = std::slice::from_raw_parts((*s).data, (*s).len as usize);
        let mut hasher = Sha256::new();
        hasher.update(data);
        let hash: [u8; 32] = hasher.finalize().into();

        hash_to_hex_string(&hash)
    }
}

/// Compute SHA-256 hash of a file's contents
///
/// # Arguments
/// * `path` - Pointer to TrqString containing the file path
///
/// # Returns
/// * Pointer to new TrqString containing 64-character lowercase hex hash
/// * Returns empty string on error (file not found, read error, etc.)
///
/// # C Equivalent
/// ```c
/// TrqString* trq_sha256_file(const TrqString* path);
/// ```
#[no_mangle]
pub extern "C" fn trq_sha256_file(path: *const TrqString) -> *mut TrqString {
    unsafe {
        if path.is_null() || (*path).data.is_null() {
            return allocate_string(0);
        }

        // Convert path to Rust string
        let path_bytes = std::slice::from_raw_parts((*path).data, (*path).len as usize);
        let path_str = match std::str::from_utf8(path_bytes) {
            Ok(s) => s,
            Err(_) => return allocate_string(0),
        };

        // Open file
        let file = match File::open(path_str) {
            Ok(f) => f,
            Err(_) => return allocate_string(0),
        };

        // Read and hash file in chunks (8KB buffer for efficiency)
        let mut reader = BufReader::with_capacity(8192, file);
        let mut hasher = Sha256::new();
        let mut buffer = [0u8; 8192];

        loop {
            match reader.read(&mut buffer) {
                Ok(0) => break, // EOF
                Ok(n) => hasher.update(&buffer[..n]),
                Err(_) => return allocate_string(0),
            }
        }

        let hash: [u8; 32] = hasher.finalize().into();
        hash_to_hex_string(&hash)
    }
}

/// Compute SHA-256 hash of a byte array
///
/// # Arguments
/// * `arr` - Pointer to TrqArray containing bytes (elem_size must be 8, uses low byte)
///
/// # Returns
/// * Pointer to new TrqString containing 64-character lowercase hex hash
/// * Returns empty string on error
///
/// # C Equivalent
/// ```c
/// TrqString* trq_sha256_bytes(const TrqArray* arr);
/// ```
#[no_mangle]
pub extern "C" fn trq_sha256_bytes(arr: *const TrqArray) -> *mut TrqString {
    unsafe {
        if arr.is_null() || (*arr).data.is_null() || (*arr).len <= 0 {
            return allocate_string(0);
        }

        // Validate elem_size - TrqArray stores int64_t values
        if (*arr).elem_size != 8 {
            return allocate_string(0);
        }

        // Extract bytes from array (low byte of each int64_t element)
        let data_ptr = (*arr).data as *const i64;
        let len = (*arr).len as usize;
        let mut bytes = Vec::with_capacity(len);

        for i in 0..len {
            let value = *data_ptr.add(i);
            bytes.push((value & 0xFF) as u8);
        }

        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        let hash: [u8; 32] = hasher.finalize().into();

        hash_to_hex_string(&hash)
    }
}

/// Compare two SHA-256 hashes in constant time (timing-attack resistant)
///
/// # Arguments
/// * `hash1` - First hash string (64-character hex)
/// * `hash2` - Second hash string (64-character hex)
///
/// # Returns
/// * `true` if hashes are equal, `false` otherwise
///
/// # Security
/// Uses XOR-based comparison to prevent timing attacks.
/// Always compares all characters regardless of early mismatches.
///
/// # C Equivalent
/// ```c
/// bool trq_sha256_compare(const TrqString* hash1, const TrqString* hash2);
/// ```
#[no_mangle]
pub extern "C" fn trq_sha256_compare(hash1: *const TrqString, hash2: *const TrqString) -> bool {
    unsafe {
        // Check for null pointers
        if hash1.is_null() || hash2.is_null() {
            return false;
        }
        if (*hash1).data.is_null() || (*hash2).data.is_null() {
            return false;
        }

        // Both must be 64 characters (SHA-256 hex)
        if (*hash1).len != 64 || (*hash2).len != 64 {
            return false;
        }

        // Constant-time comparison using XOR
        let data1 = std::slice::from_raw_parts((*hash1).data, 64);
        let data2 = std::slice::from_raw_parts((*hash2).data, 64);

        let mut result: u8 = 0;
        for i in 0..64 {
            result |= data1[i] ^ data2[i];
        }

        result == 0
    }
}

// ============================================================================
// Hex Encoding Functions
// ============================================================================

/// Encode a string to hexadecimal
///
/// # Arguments
/// * `s` - Pointer to TrqString to encode
///
/// # Returns
/// * Pointer to new TrqString containing lowercase hex encoding
/// * Returns empty string on error
///
/// # C Equivalent
/// ```c
/// TrqString* trq_hex_encode(const TrqString* s);
/// ```
#[no_mangle]
pub extern "C" fn trq_hex_encode(s: *const TrqString) -> *mut TrqString {
    unsafe {
        if s.is_null() || (*s).data.is_null() {
            return allocate_string(0);
        }

        let input_len = (*s).len as usize;
        if input_len == 0 {
            return allocate_string(0);
        }

        // Each byte becomes 2 hex characters
        let output_len = input_len * 2;
        let str_ptr = allocate_string(output_len as i64);
        if str_ptr.is_null() {
            return ptr::null_mut();
        }

        let input = std::slice::from_raw_parts((*s).data, input_len);
        let output = (*str_ptr).data;

        for (i, byte) in input.iter().enumerate() {
            *output.add(i * 2) = HEX_CHARS[(byte >> 4) as usize];
            *output.add(i * 2 + 1) = HEX_CHARS[(byte & 0x0F) as usize];
        }
        *output.add(output_len) = 0; // Null-terminate

        str_ptr
    }
}

/// Decode a hexadecimal string to original bytes as string
///
/// # Arguments
/// * `hex` - Pointer to TrqString containing hex characters
///
/// # Returns
/// * Pointer to new TrqString containing decoded bytes
/// * Returns null on error (invalid hex, odd length, etc.)
///
/// # C Equivalent
/// ```c
/// TrqString* trq_hex_decode(const TrqString* hex);
/// ```
#[no_mangle]
pub extern "C" fn trq_hex_decode(hex: *const TrqString) -> *mut TrqString {
    unsafe {
        if hex.is_null() || (*hex).data.is_null() {
            return ptr::null_mut();
        }

        let input_len = (*hex).len as usize;
        if input_len == 0 {
            return allocate_string(0);
        }

        // Must be even length
        if input_len % 2 != 0 {
            return ptr::null_mut();
        }

        let output_len = input_len / 2;
        let str_ptr = allocate_string(output_len as i64);
        if str_ptr.is_null() {
            return ptr::null_mut();
        }

        let input = std::slice::from_raw_parts((*hex).data, input_len);
        let output = (*str_ptr).data;

        for i in 0..output_len {
            let high = hex_char_to_value(input[i * 2]);
            let low = hex_char_to_value(input[i * 2 + 1]);

            if high < 0 || low < 0 {
                // Invalid hex character - free allocated string and return null
                libc::free(output as *mut libc::c_void);
                crate::memory::trq_free(str_ptr as *mut u8);
                return ptr::null_mut();
            }

            *output.add(i) = ((high << 4) | low) as u8;
        }
        *output.add(output_len) = 0; // Null-terminate

        str_ptr
    }
}

/// Encode a byte array to hexadecimal string
///
/// # Arguments
/// * `arr` - Pointer to TrqArray containing bytes (elem_size must be 8, uses low byte)
///
/// # Returns
/// * Pointer to new TrqString containing lowercase hex encoding
/// * Returns empty string on error
///
/// # C Equivalent
/// ```c
/// TrqString* trq_hex_encode_bytes(const TrqArray* arr);
/// ```
#[no_mangle]
pub extern "C" fn trq_hex_encode_bytes(arr: *const TrqArray) -> *mut TrqString {
    unsafe {
        if arr.is_null() || (*arr).data.is_null() || (*arr).len <= 0 {
            return allocate_string(0);
        }

        // Validate elem_size - TrqArray stores int64_t values
        if (*arr).elem_size != 8 {
            return allocate_string(0);
        }

        let input_len = (*arr).len as usize;
        let output_len = input_len * 2;

        let str_ptr = allocate_string(output_len as i64);
        if str_ptr.is_null() {
            return ptr::null_mut();
        }

        let data_ptr = (*arr).data as *const i64;
        let output = (*str_ptr).data;

        for i in 0..input_len {
            let byte = (*data_ptr.add(i) & 0xFF) as u8;
            *output.add(i * 2) = HEX_CHARS[(byte >> 4) as usize];
            *output.add(i * 2 + 1) = HEX_CHARS[(byte & 0x0F) as usize];
        }
        *output.add(output_len) = 0; // Null-terminate

        str_ptr
    }
}

/// Decode a hexadecimal string to byte array
///
/// # Arguments
/// * `hex` - Pointer to TrqString containing hex characters
///
/// # Returns
/// * Pointer to new TrqArray containing decoded bytes (elem_size=8, values in low byte)
/// * Returns null on error (invalid hex, odd length, etc.)
///
/// # C Equivalent
/// ```c
/// TrqArray* trq_hex_decode_to_bytes(const TrqString* hex);
/// ```
#[no_mangle]
pub extern "C" fn trq_hex_decode_to_bytes(hex: *const TrqString) -> *mut TrqArray {
    unsafe {
        if hex.is_null() || (*hex).data.is_null() {
            return ptr::null_mut();
        }

        let input_len = (*hex).len as usize;
        if input_len == 0 {
            // Return empty array
            return allocate_array(0, 8);
        }

        // Must be even length
        if input_len % 2 != 0 {
            return ptr::null_mut();
        }

        let output_len = input_len / 2;
        let arr_ptr = allocate_array(output_len as i64, 8);
        if arr_ptr.is_null() {
            return ptr::null_mut();
        }

        let input = std::slice::from_raw_parts((*hex).data, input_len);
        let output = (*arr_ptr).data as *mut i64;

        for i in 0..output_len {
            let high = hex_char_to_value(input[i * 2]);
            let low = hex_char_to_value(input[i * 2 + 1]);

            if high < 0 || low < 0 {
                // Invalid hex character - free allocated array and return null
                libc::free(output as *mut libc::c_void);
                crate::memory::trq_free(arr_ptr as *mut u8);
                return ptr::null_mut();
            }

            *output.add(i) = ((high << 4) | low) as i64;
        }

        arr_ptr
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
    fn test_sha256_string_empty() {
        unsafe {
            let s = trq_string_new(b"".as_ptr(), 0);
            let hash = trq_sha256_string(s);
            assert!(!hash.is_null());
            assert_eq!((*hash).len, 64);

            // SHA-256 of empty string
            let expected = b"e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
            let result = std::slice::from_raw_parts((*hash).data, 64);
            assert_eq!(result, expected);
        }
    }

    #[test]
    fn test_sha256_string_hello() {
        unsafe {
            let s = trq_string_new(b"hello".as_ptr(), 5);
            let hash = trq_sha256_string(s);
            assert!(!hash.is_null());
            assert_eq!((*hash).len, 64);

            // SHA-256 of "hello"
            let expected = b"2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";
            let result = std::slice::from_raw_parts((*hash).data, 64);
            assert_eq!(result, expected);
        }
    }

    #[test]
    fn test_sha256_compare_equal() {
        unsafe {
            let s1 = trq_string_new(b"hello".as_ptr(), 5);
            let hash1 = trq_sha256_string(s1);
            let s2 = trq_string_new(b"hello".as_ptr(), 5);
            let hash2 = trq_sha256_string(s2);

            assert!(trq_sha256_compare(hash1, hash2));
        }
    }

    #[test]
    fn test_sha256_compare_different() {
        unsafe {
            let s1 = trq_string_new(b"hello".as_ptr(), 5);
            let hash1 = trq_sha256_string(s1);
            let s2 = trq_string_new(b"world".as_ptr(), 5);
            let hash2 = trq_sha256_string(s2);

            assert!(!trq_sha256_compare(hash1, hash2));
        }
    }

    #[test]
    fn test_hex_encode() {
        unsafe {
            let s = trq_string_new(b"Hello".as_ptr(), 5);
            let hex = trq_hex_encode(s);
            assert!(!hex.is_null());
            assert_eq!((*hex).len, 10);

            let result = std::slice::from_raw_parts((*hex).data, 10);
            assert_eq!(result, b"48656c6c6f");
        }
    }

    #[test]
    fn test_hex_decode() {
        unsafe {
            let hex = trq_string_new(b"48656c6c6f".as_ptr(), 10);
            let decoded = trq_hex_decode(hex);
            assert!(!decoded.is_null());
            assert_eq!((*decoded).len, 5);

            let result = std::slice::from_raw_parts((*decoded).data, 5);
            assert_eq!(result, b"Hello");
        }
    }

    #[test]
    fn test_hex_roundtrip() {
        unsafe {
            let original = trq_string_new(b"Test123!".as_ptr(), 8);
            let encoded = trq_hex_encode(original);
            let decoded = trq_hex_decode(encoded);

            assert!(!decoded.is_null());
            assert_eq!((*decoded).len, 8);

            let result = std::slice::from_raw_parts((*decoded).data, 8);
            assert_eq!(result, b"Test123!");
        }
    }

    #[test]
    fn test_hex_decode_invalid() {
        unsafe {
            // Invalid hex character 'g'
            let hex = trq_string_new(b"48656g6c6f".as_ptr(), 10);
            let decoded = trq_hex_decode(hex);
            assert!(decoded.is_null());
        }
    }

    #[test]
    fn test_hex_decode_odd_length() {
        unsafe {
            // Odd length
            let hex = trq_string_new(b"48656".as_ptr(), 5);
            let decoded = trq_hex_decode(hex);
            assert!(decoded.is_null());
        }
    }
}
