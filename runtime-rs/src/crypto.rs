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

use crate::helpers::{allocate_array, allocate_string};
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
        if !input_len.is_multiple_of(2) {
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
        if !input_len.is_multiple_of(2) {
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

/// Standard base64 alphabet (RFC 4648 §4), padded with `=`.
const BASE64_CHARS: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Position of a byte in [`BASE64_CHARS`], or `None` if it is not a base64
/// character.
fn base64_value(c: u8) -> Option<u32> {
    let index = match c {
        b'A'..=b'Z' => c - b'A',
        b'a'..=b'z' => c - b'a' + 26,
        b'0'..=b'9' => c - b'0' + 52,
        b'+' => 62,
        b'/' => 63,
        _ => return None,
    };
    Some(index as u32)
}

/// Encode a string as base64. Backs the builtin `ترميز_أساس64`, declared by
/// codegen since before any definition existed (#241).
///
/// Hand-rolled rather than pulling in a crate, matching the hex encoding above.
/// The caller owns the returned string.
///
/// # Safety
///
/// - `s` must be a valid pointer to a `TrqString` or null.
#[no_mangle]
pub extern "C" fn trq_base64_encode(s: *const TrqString) -> *mut TrqString {
    unsafe {
        if s.is_null() || (*s).data.is_null() || (*s).len <= 0 {
            return allocate_string(0);
        }

        let input = std::slice::from_raw_parts((*s).data, (*s).len as usize);
        let output_len = input.len().div_ceil(3) * 4;
        let str_ptr = allocate_string(output_len as i64);
        if str_ptr.is_null() {
            return ptr::null_mut();
        }
        let output = (*str_ptr).data;

        for (block, chunk) in input.chunks(3).enumerate() {
            let mut bits = 0u32;
            for (i, byte) in chunk.iter().enumerate() {
                bits |= (*byte as u32) << (16 - 8 * i);
            }

            let at = block * 4;
            for slot in 0..4 {
                // A 3-byte block yields 4 characters; a short tail pads the
                // slots it cannot fill.
                *output.add(at + slot) = if slot <= chunk.len() {
                    BASE64_CHARS[((bits >> (18 - 6 * slot)) & 0x3F) as usize]
                } else {
                    b'='
                };
            }
        }
        *output.add(output_len) = 0;

        str_ptr
    }
}

/// Decode a base64 string. Backs the builtin `فك_أساس64`.
///
/// Returns an empty string when the input is not valid base64, matching how
/// `trq_sha256_file` reports failure. The caller owns the returned string.
///
/// # Safety
///
/// - `s` must be a valid pointer to a `TrqString` or null.
#[no_mangle]
pub extern "C" fn trq_base64_decode(s: *const TrqString) -> *mut TrqString {
    unsafe {
        if s.is_null() || (*s).data.is_null() || (*s).len <= 0 {
            return allocate_string(0);
        }

        let input = std::slice::from_raw_parts((*s).data, (*s).len as usize);
        let body = match input.iter().position(|&c| c == b'=') {
            Some(pad) => {
                // Padding is only legal as the final one or two characters.
                if pad + 2 < input.len() || input[pad..].iter().any(|&c| c != b'=') {
                    return allocate_string(0);
                }
                &input[..pad]
            }
            None => input,
        };

        if input.len() % 4 != 0 || body.len() % 4 == 1 {
            return allocate_string(0);
        }

        let mut decoded = Vec::with_capacity(body.len() / 4 * 3);
        for chunk in body.chunks(4) {
            let mut bits = 0u32;
            for (i, byte) in chunk.iter().enumerate() {
                match base64_value(*byte) {
                    Some(value) => bits |= value << (18 - 6 * i),
                    None => return allocate_string(0),
                }
            }
            for slot in 0..chunk.len() - 1 {
                decoded.push((bits >> (16 - 8 * slot)) as u8);
            }
        }

        let str_ptr = allocate_string(decoded.len() as i64);
        if str_ptr.is_null() {
            return ptr::null_mut();
        }
        ptr::copy_nonoverlapping(decoded.as_ptr(), (*str_ptr).data, decoded.len());
        *(*str_ptr).data.add(decoded.len()) = 0;

        str_ptr
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

    fn text_of(result: *mut TrqString) -> String {
        unsafe {
            let s = &*result;
            if s.data.is_null() || s.len <= 0 {
                return String::new();
            }
            let bytes = std::slice::from_raw_parts(s.data as *const u8, s.len as usize);
            String::from_utf8_lossy(bytes).into_owned()
        }
    }

    fn owned(text: &str) -> *mut TrqString {
        trq_string_new(text.as_ptr(), text.len() as i64)
    }

    #[test]
    fn test_base64_encode_covers_every_padding_length() {
        // RFC 4648 §10 vectors: 0, 1 and 2 bytes of padding.
        assert_eq!(text_of(trq_base64_encode(owned("foo"))), "Zm9v");
        assert_eq!(text_of(trq_base64_encode(owned("fo"))), "Zm8=");
        assert_eq!(text_of(trq_base64_encode(owned("f"))), "Zg==");
        assert_eq!(text_of(trq_base64_encode(owned("foobar"))), "Zm9vYmFy");
        assert_eq!(text_of(trq_base64_encode(owned(""))), "");
    }

    #[test]
    fn test_base64_round_trips_arabic() {
        // Multi-byte UTF-8 is the case a byte-oriented encoder gets wrong.
        for text in ["مرحبا", "ترقيم", "السلام عليكم"] {
            let encoded = trq_base64_encode(owned(text));
            assert_eq!(text_of(trq_base64_decode(encoded)), text);
        }
    }

    #[test]
    fn test_base64_decode_matches_known_vectors() {
        assert_eq!(text_of(trq_base64_decode(owned("Zm9v"))), "foo");
        assert_eq!(text_of(trq_base64_decode(owned("Zm8="))), "fo");
        assert_eq!(text_of(trq_base64_decode(owned("Zg=="))), "f");
    }

    #[test]
    fn test_base64_decode_rejects_malformed_input() {
        // Wrong length, illegal character, and padding in the middle.
        assert_eq!(text_of(trq_base64_decode(owned("Zm9"))), "");
        assert_eq!(text_of(trq_base64_decode(owned("Zm9!"))), "");
        assert_eq!(text_of(trq_base64_decode(owned("Z=9v"))), "");
    }

    #[test]
    fn test_base64_decode_rejects_pathological_padding() {
        // Padding that is not a final one-or-two-character tail.
        for bad in ["====", "AAAA====", "=", "A===", "AB=A"] {
            assert_eq!(
                text_of(trq_base64_decode(owned(bad))),
                "",
                "accepted {bad:?}"
            );
        }
    }

    #[test]
    fn test_base64_encodes_arbitrary_bytes() {
        // 0xFF/0x00 exercise the top bits of the 24-bit accumulator, which
        // UTF-8 text alone never sets.
        let raw = [0xFFu8, 0x00, 0xFF];
        let s = trq_string_new(raw.as_ptr(), 3);
        assert_eq!(text_of(trq_base64_encode(s)), "/wD/");
    }
}
