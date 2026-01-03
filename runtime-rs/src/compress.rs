//! Compression operations for Tarqeem runtime
//!
//! This module implements GZIP compression and decompression operations
//! that are ABI-compatible with the C runtime.
//!
//! # Functions
//!
//! ## String/Bytes Compression
//! - `trq_gzip_compress_string` - Compress string to bytes
//! - `trq_gzip_decompress_to_string` - Decompress bytes to string
//! - `trq_gzip_compress_bytes` - Compress byte array
//! - `trq_gzip_decompress_bytes` - Decompress byte array
//!
//! ## File Compression
//! - `trq_gzip_compress_file` - Compress file to .gz
//! - `trq_gzip_decompress_file` - Decompress .gz file

use crate::helpers::{allocate_array, allocate_string};
use crate::types::{TrqArray, TrqString};
use flate2::read::{GzDecoder, GzEncoder};
use flate2::Compression;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::ptr;

// ============================================================================
// Helper Functions (internal)
// ============================================================================

/// Convert Vec<u8> to TrqArray (each byte stored as int64_t)
unsafe fn vec_to_byte_array(bytes: Vec<u8>) -> *mut TrqArray {
    let len = bytes.len() as i64;
    let arr_ptr = allocate_array(len, 8);
    if arr_ptr.is_null() {
        return ptr::null_mut();
    }

    if len > 0 {
        let data = (*arr_ptr).data as *mut i64;
        for (i, byte) in bytes.iter().enumerate() {
            *data.add(i) = *byte as i64;
        }
    }

    arr_ptr
}

/// Extract bytes from TrqArray (low byte of each int64_t element)
unsafe fn array_to_bytes(arr: *const TrqArray) -> Option<Vec<u8>> {
    if arr.is_null() || (*arr).data.is_null() || (*arr).len <= 0 {
        return None;
    }

    // TrqArray stores int64_t values
    if (*arr).elem_size != 8 {
        return None;
    }

    let data_ptr = (*arr).data as *const i64;
    let len = (*arr).len as usize;
    let mut bytes = Vec::with_capacity(len);

    for i in 0..len {
        let value = *data_ptr.add(i);
        bytes.push((value & 0xFF) as u8);
    }

    Some(bytes)
}

// ============================================================================
// String/Bytes Compression Functions
// ============================================================================

/// Compress a string using GZIP
///
/// # Arguments
/// * `s` - Pointer to TrqString to compress
///
/// # Returns
/// * Pointer to new TrqArray containing compressed bytes (elem_size=8)
/// * Returns null on error
///
/// # C Equivalent
/// ```c
/// TrqArray* trq_gzip_compress_string(const TrqString* s);
/// ```
#[no_mangle]
pub extern "C" fn trq_gzip_compress_string(s: *const TrqString) -> *mut TrqArray {
    unsafe {
        if s.is_null() || (*s).data.is_null() {
            return ptr::null_mut();
        }

        let input = std::slice::from_raw_parts((*s).data, (*s).len as usize);

        // Compress using GZIP
        let mut encoder = GzEncoder::new(input, Compression::default());
        let mut compressed = Vec::new();

        match encoder.read_to_end(&mut compressed) {
            Ok(_) => vec_to_byte_array(compressed),
            Err(_) => ptr::null_mut(),
        }
    }
}

/// Decompress GZIP bytes to a string
///
/// # Arguments
/// * `arr` - Pointer to TrqArray containing compressed bytes (elem_size=8)
///
/// # Returns
/// * Pointer to new TrqString containing decompressed data
/// * Returns null on error (invalid GZIP data, etc.)
///
/// # C Equivalent
/// ```c
/// TrqString* trq_gzip_decompress_to_string(const TrqArray* arr);
/// ```
#[no_mangle]
pub extern "C" fn trq_gzip_decompress_to_string(arr: *const TrqArray) -> *mut TrqString {
    unsafe {
        let compressed = match array_to_bytes(arr) {
            Some(b) => b,
            None => return ptr::null_mut(),
        };

        // Decompress using GZIP
        let mut decoder = GzDecoder::new(compressed.as_slice());
        let mut decompressed = Vec::new();

        match decoder.read_to_end(&mut decompressed) {
            Ok(_) => {
                let len = decompressed.len() as i64;
                let str_ptr = allocate_string(len);
                if str_ptr.is_null() {
                    return ptr::null_mut();
                }

                if len > 0 {
                    ptr::copy_nonoverlapping(decompressed.as_ptr(), (*str_ptr).data, len as usize);
                }
                *(*str_ptr).data.add(len as usize) = 0; // Null-terminate

                str_ptr
            }
            Err(_) => ptr::null_mut(),
        }
    }
}

/// Compress a byte array using GZIP
///
/// # Arguments
/// * `arr` - Pointer to TrqArray containing bytes to compress (elem_size=8)
///
/// # Returns
/// * Pointer to new TrqArray containing compressed bytes (elem_size=8)
/// * Returns null on error
///
/// # C Equivalent
/// ```c
/// TrqArray* trq_gzip_compress_bytes(const TrqArray* arr);
/// ```
#[no_mangle]
pub extern "C" fn trq_gzip_compress_bytes(arr: *const TrqArray) -> *mut TrqArray {
    unsafe {
        let input = match array_to_bytes(arr) {
            Some(b) => b,
            None => return ptr::null_mut(),
        };

        // Compress using GZIP
        let mut encoder = GzEncoder::new(input.as_slice(), Compression::default());
        let mut compressed = Vec::new();

        match encoder.read_to_end(&mut compressed) {
            Ok(_) => vec_to_byte_array(compressed),
            Err(_) => ptr::null_mut(),
        }
    }
}

/// Decompress GZIP bytes to a byte array
///
/// # Arguments
/// * `arr` - Pointer to TrqArray containing compressed bytes (elem_size=8)
///
/// # Returns
/// * Pointer to new TrqArray containing decompressed bytes (elem_size=8)
/// * Returns null on error (invalid GZIP data, etc.)
///
/// # C Equivalent
/// ```c
/// TrqArray* trq_gzip_decompress_bytes(const TrqArray* arr);
/// ```
#[no_mangle]
pub extern "C" fn trq_gzip_decompress_bytes(arr: *const TrqArray) -> *mut TrqArray {
    unsafe {
        let compressed = match array_to_bytes(arr) {
            Some(b) => b,
            None => return ptr::null_mut(),
        };

        // Decompress using GZIP
        let mut decoder = GzDecoder::new(compressed.as_slice());
        let mut decompressed = Vec::new();

        match decoder.read_to_end(&mut decompressed) {
            Ok(_) => vec_to_byte_array(decompressed),
            Err(_) => ptr::null_mut(),
        }
    }
}

// ============================================================================
// File Compression Functions
// ============================================================================

/// Compress a file using GZIP
///
/// Reads the input file and writes compressed data to the output file.
/// Uses chunked I/O (16KB buffer) for memory efficiency with large files.
///
/// # Arguments
/// * `input_path` - Path to the input file
/// * `output_path` - Path to the output .gz file
///
/// # Returns
/// * `true` on success, `false` on error (file not found, write error, etc.)
///
/// # C Equivalent
/// ```c
/// bool trq_gzip_compress_file(const TrqString* input_path, const TrqString* output_path);
/// ```
#[no_mangle]
pub extern "C" fn trq_gzip_compress_file(
    input_path: *const TrqString,
    output_path: *const TrqString,
) -> bool {
    unsafe {
        // Validate input paths
        if input_path.is_null()
            || (*input_path).data.is_null()
            || output_path.is_null()
            || (*output_path).data.is_null()
        {
            return false;
        }

        // Convert paths to Rust strings
        let input_bytes =
            std::slice::from_raw_parts((*input_path).data, (*input_path).len as usize);
        let input_str = match std::str::from_utf8(input_bytes) {
            Ok(s) => s,
            Err(_) => return false,
        };

        let output_bytes =
            std::slice::from_raw_parts((*output_path).data, (*output_path).len as usize);
        let output_str = match std::str::from_utf8(output_bytes) {
            Ok(s) => s,
            Err(_) => return false,
        };

        // Open input file
        let input_file = match File::open(input_str) {
            Ok(f) => f,
            Err(_) => return false,
        };

        // Create output file
        let output_file = match File::create(output_str) {
            Ok(f) => f,
            Err(_) => return false,
        };

        // Create buffered reader and GZIP encoder
        let mut reader = BufReader::with_capacity(16384, input_file);
        let mut encoder = flate2::write::GzEncoder::new(
            BufWriter::with_capacity(16384, output_file),
            Compression::default(),
        );

        // Read and compress in chunks
        let mut buffer = [0u8; 16384];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => break, // EOF
                Ok(n) => {
                    if encoder.write_all(&buffer[..n]).is_err() {
                        return false;
                    }
                }
                Err(_) => return false,
            }
        }

        // Finish compression and flush
        match encoder.finish() {
            Ok(mut writer) => writer.flush().is_ok(),
            Err(_) => false,
        }
    }
}

/// Decompress a GZIP file
///
/// Reads the compressed input file and writes decompressed data to the output file.
/// Uses chunked I/O (16KB buffer) for memory efficiency with large files.
///
/// # Arguments
/// * `input_path` - Path to the input .gz file
/// * `output_path` - Path to the output file
///
/// # Returns
/// * `true` on success, `false` on error (invalid GZIP, file not found, etc.)
///
/// # C Equivalent
/// ```c
/// bool trq_gzip_decompress_file(const TrqString* input_path, const TrqString* output_path);
/// ```
#[no_mangle]
pub extern "C" fn trq_gzip_decompress_file(
    input_path: *const TrqString,
    output_path: *const TrqString,
) -> bool {
    unsafe {
        // Validate input paths
        if input_path.is_null()
            || (*input_path).data.is_null()
            || output_path.is_null()
            || (*output_path).data.is_null()
        {
            return false;
        }

        // Convert paths to Rust strings
        let input_bytes =
            std::slice::from_raw_parts((*input_path).data, (*input_path).len as usize);
        let input_str = match std::str::from_utf8(input_bytes) {
            Ok(s) => s,
            Err(_) => return false,
        };

        let output_bytes =
            std::slice::from_raw_parts((*output_path).data, (*output_path).len as usize);
        let output_str = match std::str::from_utf8(output_bytes) {
            Ok(s) => s,
            Err(_) => return false,
        };

        // Open input file
        let input_file = match File::open(input_str) {
            Ok(f) => f,
            Err(_) => return false,
        };

        // Create output file
        let output_file = match File::create(output_str) {
            Ok(f) => f,
            Err(_) => return false,
        };

        // Create GZIP decoder and buffered writer
        let mut decoder = GzDecoder::new(BufReader::with_capacity(16384, input_file));
        let mut writer = BufWriter::with_capacity(16384, output_file);

        // Read and decompress in chunks
        let mut buffer = [0u8; 16384];
        loop {
            match decoder.read(&mut buffer) {
                Ok(0) => break, // EOF
                Ok(n) => {
                    if writer.write_all(&buffer[..n]).is_err() {
                        return false;
                    }
                }
                Err(_) => return false,
            }
        }

        // Flush output
        writer.flush().is_ok()
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
    fn test_gzip_compress_string() {
        unsafe {
            let s = trq_string_new(b"Hello, World!".as_ptr(), 13);
            let compressed = trq_gzip_compress_string(s);
            assert!(!compressed.is_null());
            assert!((*compressed).len > 0);
            // Compressed data should be present
            assert!(!(*compressed).data.is_null());
        }
    }

    #[test]
    fn test_gzip_decompress_to_string() {
        unsafe {
            let original = trq_string_new(b"Hello, World!".as_ptr(), 13);
            let compressed = trq_gzip_compress_string(original);
            assert!(!compressed.is_null());

            let decompressed = trq_gzip_decompress_to_string(compressed);
            assert!(!decompressed.is_null());
            assert_eq!((*decompressed).len, 13);

            let result = std::slice::from_raw_parts((*decompressed).data, 13);
            assert_eq!(result, b"Hello, World!");
        }
    }

    #[test]
    fn test_gzip_roundtrip_string() {
        unsafe {
            let test_data = b"This is a test string that should be compressed and then decompressed back to the original.";
            let original = trq_string_new(test_data.as_ptr(), test_data.len() as i64);

            let compressed = trq_gzip_compress_string(original);
            assert!(!compressed.is_null());

            let decompressed = trq_gzip_decompress_to_string(compressed);
            assert!(!decompressed.is_null());
            assert_eq!((*decompressed).len, test_data.len() as i64);

            let result = std::slice::from_raw_parts((*decompressed).data, test_data.len());
            assert_eq!(result, test_data);
        }
    }

    #[test]
    fn test_gzip_compress_bytes() {
        unsafe {
            // Create a byte array
            let arr = allocate_array(5, 8);
            assert!(!arr.is_null());
            let data = (*arr).data as *mut i64;
            *data.add(0) = 72; // 'H'
            *data.add(1) = 101; // 'e'
            *data.add(2) = 108; // 'l'
            *data.add(3) = 108; // 'l'
            *data.add(4) = 111; // 'o'

            let compressed = trq_gzip_compress_bytes(arr);
            assert!(!compressed.is_null());
            assert!((*compressed).len > 0);
        }
    }

    #[test]
    fn test_gzip_roundtrip_bytes() {
        unsafe {
            // Create a byte array
            let arr = allocate_array(5, 8);
            assert!(!arr.is_null());
            let data = (*arr).data as *mut i64;
            *data.add(0) = 72; // 'H'
            *data.add(1) = 101; // 'e'
            *data.add(2) = 108; // 'l'
            *data.add(3) = 108; // 'l'
            *data.add(4) = 111; // 'o'

            let compressed = trq_gzip_compress_bytes(arr);
            assert!(!compressed.is_null());

            let decompressed = trq_gzip_decompress_bytes(compressed);
            assert!(!decompressed.is_null());
            assert_eq!((*decompressed).len, 5);

            let result = (*decompressed).data as *const i64;
            assert_eq!(*result.add(0), 72);
            assert_eq!(*result.add(1), 101);
            assert_eq!(*result.add(2), 108);
            assert_eq!(*result.add(3), 108);
            assert_eq!(*result.add(4), 111);
        }
    }

    #[test]
    fn test_gzip_decompress_invalid() {
        unsafe {
            // Create an array with invalid GZIP data
            let arr = allocate_array(5, 8);
            assert!(!arr.is_null());
            let data = (*arr).data as *mut i64;
            *data.add(0) = 1;
            *data.add(1) = 2;
            *data.add(2) = 3;
            *data.add(3) = 4;
            *data.add(4) = 5;

            let result = trq_gzip_decompress_to_string(arr);
            assert!(result.is_null());
        }
    }

    #[test]
    fn test_gzip_compress_empty() {
        unsafe {
            let s = trq_string_new(b"".as_ptr(), 0);
            let compressed = trq_gzip_compress_string(s);
            // Empty input should still produce valid GZIP output
            assert!(!compressed.is_null());

            let decompressed = trq_gzip_decompress_to_string(compressed);
            assert!(!decompressed.is_null());
            assert_eq!((*decompressed).len, 0);
        }
    }

    #[test]
    fn test_gzip_file_roundtrip() {
        use std::fs;

        unsafe {
            // Create test file
            let test_content = b"Test file content for GZIP compression test.";
            let input_path = "/tmp/tarqeem_gzip_test_input.txt";
            let compressed_path = "/tmp/tarqeem_gzip_test.gz";
            let output_path = "/tmp/tarqeem_gzip_test_output.txt";

            fs::write(input_path, test_content).unwrap();

            // Create path strings
            let input_str = trq_string_new(input_path.as_ptr(), input_path.len() as i64);
            let compressed_str =
                trq_string_new(compressed_path.as_ptr(), compressed_path.len() as i64);
            let output_str = trq_string_new(output_path.as_ptr(), output_path.len() as i64);

            // Compress
            assert!(trq_gzip_compress_file(input_str, compressed_str));

            // Decompress
            assert!(trq_gzip_decompress_file(compressed_str, output_str));

            // Verify content
            let result = fs::read(output_path).unwrap();
            assert_eq!(result.as_slice(), test_content);

            // Cleanup
            let _ = fs::remove_file(input_path);
            let _ = fs::remove_file(compressed_path);
            let _ = fs::remove_file(output_path);
        }
    }
}
