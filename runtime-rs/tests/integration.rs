//! Integration tests for tarqeem-runtime (runtime-rs)
//!
//! These tests verify cross-module workflows and integration between
//! different runtime components (memory, string, array, I/O, crypto, compress).
//!
//! Phase 8: Testing and Verification

// Import from the trq library (lib.name = "trq" in Cargo.toml)
use trq::*;

// ============================================================================
// String + Memory Workflow Tests
// ============================================================================

mod string_memory_workflow {
    use super::*;

    #[test]
    fn test_string_create_manipulate_free() {
        // Create a string
        let original = "مرحباً بالعالم";
        let s = trq_string_new(original.as_ptr(), original.len() as i64);
        assert!(!s.is_null());

        // Check initial refcount
        let initial_refcount = trq_refcount(s as *mut u8);
        assert_eq!(initial_refcount, 1);

        // Clone the string (should increment refcount or create new)
        let cloned = trq_string_clone(s);
        assert!(!cloned.is_null());

        // Verify content matches
        unsafe {
            assert_eq!((*cloned).len, (*s).len);
        }

        // Manipulate: convert to upper
        let upper = trq_string_to_upper(s);
        assert!(!upper.is_null());

        // Verify it's Arabic text detection works
        assert!(trq_string_is_arabic(s));

        // Free all strings
        unsafe {
            trq_release(upper as *mut u8);
            trq_release(cloned as *mut u8);
            trq_release(s as *mut u8);
        }
    }

    #[test]
    fn test_string_concat_chain() {
        // Build a string through multiple concatenations
        let part1 = "أهلاً";
        let part2 = " و";
        let part3 = " سهلاً";

        let s1 = trq_string_new(part1.as_ptr(), part1.len() as i64);
        let s2 = trq_string_new(part2.as_ptr(), part2.len() as i64);
        let s3 = trq_string_new(part3.as_ptr(), part3.len() as i64);

        // Concat chain
        let temp = trq_string_concat(s1, s2);
        let result = trq_string_concat(temp, s3);

        assert!(!result.is_null());
        unsafe {
            // "أهلاً و سهلاً"
            assert!((*result).len > 0);
        }

        // Cleanup
        unsafe {
            trq_release(result as *mut u8);
            trq_release(temp as *mut u8);
            trq_release(s1 as *mut u8);
            trq_release(s2 as *mut u8);
            trq_release(s3 as *mut u8);
        }
    }

    #[test]
    fn test_string_split_join_roundtrip() {
        // Split a string and join it back
        let original = "واحد،اثنان،ثلاثة";
        let s = trq_string_new(original.as_ptr(), original.len() as i64);

        let delim = "،";
        let delim_str = trq_string_new(delim.as_ptr(), delim.len() as i64);

        // Split
        let parts = trq_string_split(s, delim_str);
        assert!(!parts.is_null());
        unsafe {
            assert_eq!(trq_array_len(parts), 3);
        }

        // Join back
        let rejoined = trq_string_join(parts, delim_str);
        assert!(!rejoined.is_null());

        // Should equal original
        assert!(trq_string_equals(s, rejoined));

        // Cleanup
        unsafe {
            trq_release(rejoined as *mut u8);
            // Free array elements
            for i in 0..3 {
                let elem = trq_array_get(parts, i);
                if !elem.is_null() {
                    let str_ptr = *(elem as *const *mut TrqString);
                    trq_release(str_ptr as *mut u8);
                }
            }
            trq_release(parts as *mut u8);
            trq_release(delim_str as *mut u8);
            trq_release(s as *mut u8);
        }
    }
}

// ============================================================================
// Array Workflow Tests
// ============================================================================

mod array_workflow {
    use super::*;

    #[test]
    fn test_array_create_push_pop_free() {
        // Create array for i64 elements
        let elem_size = std::mem::size_of::<i64>() as i64;
        let arr = trq_array_new(0, elem_size);
        assert!(!arr.is_null());

        // Push elements
        for i in 1..=10i64 {
            trq_array_push(arr, &i as *const i64 as *const u8, elem_size);
        }
        assert_eq!(trq_array_len(arr), 10);

        // Pop and verify order (LIFO)
        for expected in (1..=10i64).rev() {
            let ptr = trq_array_pop(arr);
            assert!(!ptr.is_null());
            unsafe {
                let value = *(ptr as *const i64);
                assert_eq!(value, expected);
            }
        }

        assert_eq!(trq_array_len(arr), 0);

        // Free
        unsafe {
            trq_release(arr as *mut u8);
        }
    }

    #[test]
    fn test_array_capacity_growth() {
        let elem_size = std::mem::size_of::<i64>() as i64;
        let arr = trq_array_new(0, elem_size);

        // Push many elements to trigger capacity growth
        for i in 0..100i64 {
            trq_array_push(arr, &i as *const i64 as *const u8, elem_size);
        }

        assert_eq!(trq_array_len(arr), 100);

        // Capacity should have grown
        unsafe {
            assert!((*arr).cap >= 100);
        }

        // Verify all elements are correct
        for i in 0..100i64 {
            let ptr = trq_array_get(arr, i);
            assert!(!ptr.is_null());
            unsafe {
                let value = *(ptr as *const i64);
                assert_eq!(value, i);
            }
        }

        unsafe {
            trq_release(arr as *mut u8);
        }
    }

    #[test]
    fn test_array_slice_and_concat() {
        let elem_size = std::mem::size_of::<i64>() as i64;
        let arr = trq_array_new(0, elem_size);

        // Push 0-9
        for i in 0..10i64 {
            trq_array_push(arr, &i as *const i64 as *const u8, elem_size);
        }

        // Slice [2, 5)
        let slice = trq_array_slice(arr, 2, 5, elem_size);
        assert!(!slice.is_null());
        assert_eq!(trq_array_len(slice), 3);

        // Verify slice contents: 2, 3, 4
        for (i, expected) in [2i64, 3, 4].iter().enumerate() {
            let ptr = trq_array_get(slice, i as i64);
            assert!(!ptr.is_null());
            unsafe {
                assert_eq!(*(ptr as *const i64), *expected);
            }
        }

        // Concat original with slice
        let combined = trq_array_concat(arr, slice, elem_size);
        assert!(!combined.is_null());
        assert_eq!(trq_array_len(combined), 13); // 10 + 3

        unsafe {
            trq_release(combined as *mut u8);
            trq_release(slice as *mut u8);
            trq_release(arr as *mut u8);
        }
    }
}

// ============================================================================
// File I/O Workflow Tests
// ============================================================================

mod file_io_workflow {
    use super::*;

    #[test]
    fn test_file_write_read_delete() {
        let test_path = "/tmp/tarqeem_integration_test_file.txt";
        let content = "محتوى الاختبار العربي\nسطر ثاني";

        let path = trq_string_new(test_path.as_ptr(), test_path.len() as i64);
        let content_str = trq_string_new(content.as_ptr(), content.len() as i64);

        // Write file
        assert!(trq_file_write(path, content_str));

        // Verify exists
        assert!(trq_file_exists(path));
        assert!(trq_file_is_file(path));
        assert!(!trq_file_is_dir(path));

        // Read back
        let read_content = trq_file_read(path);
        assert!(!read_content.is_null());
        assert!(trq_string_equals(content_str, read_content));

        // Get size
        let size = trq_file_size(path);
        assert!(size > 0);
        unsafe {
            assert_eq!(size, (*content_str).len);
        }

        // Delete
        assert!(trq_file_delete(path));
        assert!(!trq_file_exists(path));

        unsafe {
            trq_release(read_content as *mut u8);
            trq_release(content_str as *mut u8);
            trq_release(path as *mut u8);
        }
    }

    #[test]
    fn test_file_handle_line_by_line() {
        let test_path = "/tmp/tarqeem_integration_lines.txt";
        let lines = ["السطر الأول", "السطر الثاني", "السطر الثالث"];

        let path = trq_string_new(test_path.as_ptr(), test_path.len() as i64);

        // Write lines using handle
        let write_handle = trq_file_open_write(path);
        assert!(write_handle > 0);

        for line in &lines {
            let line_str = trq_string_new(line.as_ptr(), line.len() as i64);
            assert!(trq_file_write_line(write_handle, line_str));
            unsafe {
                trq_release(line_str as *mut u8);
            }
        }
        assert!(trq_file_close(write_handle));

        // Read lines back
        let read_handle = trq_file_open_read(path);
        assert!(read_handle > 0);

        for expected in &lines {
            let read_line = trq_file_read_line(read_handle);
            assert!(!read_line.is_null());
            unsafe {
                let slice =
                    std::slice::from_raw_parts((*read_line).data, (*read_line).len as usize);
                let read_str = std::str::from_utf8(slice).unwrap();
                assert_eq!(read_str, *expected);
                trq_release(read_line as *mut u8);
            }
        }

        assert!(trq_file_eof(read_handle));
        assert!(trq_file_close(read_handle));

        // Cleanup
        assert!(trq_file_delete(path));
        unsafe {
            trq_release(path as *mut u8);
        }
    }

    #[test]
    fn test_directory_operations() {
        let test_dir = "/tmp/tarqeem_test_dir_integration";
        let test_subdir = "/tmp/tarqeem_test_dir_integration/subdir";

        let dir = trq_string_new(test_dir.as_ptr(), test_dir.len() as i64);
        let subdir = trq_string_new(test_subdir.as_ptr(), test_subdir.len() as i64);

        // Clean up if exists
        let _ = std::fs::remove_dir_all(test_dir);

        // Create nested directories
        assert!(trq_dir_create_all(subdir));
        assert!(trq_file_exists(subdir));
        assert!(trq_file_is_dir(subdir));

        // Create a file in the directory
        let file_name = "test.txt";
        let file_path = trq_path_join(
            dir,
            trq_string_new(file_name.as_ptr(), file_name.len() as i64),
        );
        let content = trq_string_new("test".as_ptr(), 4);
        assert!(trq_file_write(file_path, content));

        // List directory
        let entries = trq_dir_list(dir);
        assert!(!entries.is_null());
        assert!(trq_array_len(entries) >= 1); // At least subdir and test.txt

        // Cleanup
        let _ = std::fs::remove_dir_all(test_dir);

        unsafe {
            // Free array entries
            let len = trq_array_len(entries);
            for i in 0..len {
                let elem = trq_array_get(entries, i);
                if !elem.is_null() {
                    let str_ptr = *(elem as *const *mut TrqString);
                    trq_release(str_ptr as *mut u8);
                }
            }
            trq_release(entries as *mut u8);
            trq_release(content as *mut u8);
            trq_release(file_path as *mut u8);
            trq_release(subdir as *mut u8);
            trq_release(dir as *mut u8);
        }
    }
}

// ============================================================================
// Crypto Workflow Tests
// ============================================================================

mod crypto_workflow {
    use super::*;

    #[test]
    fn test_sha256_string_workflow() {
        // Hash a string
        let input = "مرحباً بالعالم";
        let s = trq_string_new(input.as_ptr(), input.len() as i64);

        // Get SHA256 hash
        let hash = trq_sha256_string(s);
        assert!(!hash.is_null());

        // Hash is 64 hex chars
        unsafe {
            assert_eq!((*hash).len, 64);
        }

        // Encode hash as hex (should be same as hash since sha256_string returns hex)
        let hex = trq_hex_encode(hash);
        assert!(!hex.is_null());

        // Hashing same content twice should give same result
        let hash2 = trq_sha256_string(s);
        assert!(trq_sha256_compare(hash, hash2));

        // Different content should give different hash
        let different = "مختلف";
        let s2 = trq_string_new(different.as_ptr(), different.len() as i64);
        let hash3 = trq_sha256_string(s2);
        assert!(!trq_sha256_compare(hash, hash3));

        unsafe {
            trq_release(hash3 as *mut u8);
            trq_release(s2 as *mut u8);
            trq_release(hash2 as *mut u8);
            trq_release(hex as *mut u8);
            trq_release(hash as *mut u8);
            trq_release(s as *mut u8);
        }
    }

    #[test]
    fn test_hex_encode_decode_roundtrip() {
        let original = "Test Data 123";
        let s = trq_string_new(original.as_ptr(), original.len() as i64);

        // Encode to hex
        let hex = trq_hex_encode(s);
        assert!(!hex.is_null());

        // Decode back
        let decoded = trq_hex_decode(hex);
        assert!(!decoded.is_null());

        // Should match original
        assert!(trq_string_equals(s, decoded));

        unsafe {
            trq_release(decoded as *mut u8);
            trq_release(hex as *mut u8);
            trq_release(s as *mut u8);
        }
    }

    #[test]
    fn test_sha256_file() {
        // Create a test file
        let test_path = "/tmp/tarqeem_crypto_test.txt";
        let content = "Test content for SHA256 file hashing";

        std::fs::write(test_path, content).unwrap();

        let path = trq_string_new(test_path.as_ptr(), test_path.len() as i64);
        let hash = trq_sha256_file(path);

        assert!(!hash.is_null());
        unsafe {
            assert_eq!((*hash).len, 64); // 64 hex chars
        }

        // Hash content directly and compare
        let content_str = trq_string_new(content.as_ptr(), content.len() as i64);
        let content_hash = trq_sha256_string(content_str);
        assert!(trq_sha256_compare(hash, content_hash));

        // Cleanup
        std::fs::remove_file(test_path).ok();

        unsafe {
            trq_release(content_hash as *mut u8);
            trq_release(content_str as *mut u8);
            trq_release(hash as *mut u8);
            trq_release(path as *mut u8);
        }
    }
}

// ============================================================================
// Compression Workflow Tests
// ============================================================================

mod compress_workflow {
    use super::*;

    #[test]
    fn test_gzip_compress_decompress_string() {
        let original = "هذا نص عربي طويل سيتم ضغطه ثم فك ضغطه للتحقق من صحة العملية. ";
        // Repeat to make it worth compressing
        let repeated = original.repeat(10);
        let s = trq_string_new(repeated.as_ptr(), repeated.len() as i64);

        // Compress
        let compressed = trq_gzip_compress_string(s);
        assert!(!compressed.is_null());

        // Compressed should be smaller than original
        assert!(trq_array_len(compressed) < repeated.len() as i64);

        // Decompress
        let decompressed = trq_gzip_decompress_to_string(compressed);
        assert!(!decompressed.is_null());

        // Should match original
        assert!(trq_string_equals(s, decompressed));

        unsafe {
            trq_release(decompressed as *mut u8);
            trq_release(compressed as *mut u8);
            trq_release(s as *mut u8);
        }
    }

    #[test]
    fn test_gzip_compress_file() {
        let test_path = "/tmp/tarqeem_compress_test.txt";
        let compressed_path = "/tmp/tarqeem_compress_test.txt.gz";
        let decompressed_path = "/tmp/tarqeem_compress_test_restored.txt";

        // Create test file with Arabic content
        let content = "محتوى الملف للضغط\n".repeat(100);
        std::fs::write(test_path, &content).unwrap();

        let src = trq_string_new(test_path.as_ptr(), test_path.len() as i64);
        let dst = trq_string_new(compressed_path.as_ptr(), compressed_path.len() as i64);
        let restored = trq_string_new(decompressed_path.as_ptr(), decompressed_path.len() as i64);

        // Compress file
        assert!(trq_gzip_compress_file(src, dst));
        assert!(trq_file_exists(dst));

        // Compressed file should be smaller
        let orig_size = trq_file_size(src);
        let comp_size = trq_file_size(dst);
        assert!(comp_size < orig_size);

        // Decompress file
        assert!(trq_gzip_decompress_file(dst, restored));
        assert!(trq_file_exists(restored));

        // Restored file should match original
        let restored_size = trq_file_size(restored);
        assert_eq!(restored_size, orig_size);

        // Cleanup
        std::fs::remove_file(test_path).ok();
        std::fs::remove_file(compressed_path).ok();
        std::fs::remove_file(decompressed_path).ok();

        unsafe {
            trq_release(restored as *mut u8);
            trq_release(dst as *mut u8);
            trq_release(src as *mut u8);
        }
    }
}

// ============================================================================
// Math Workflow Tests
// ============================================================================

mod math_workflow {
    use super::*;

    const EPSILON: f64 = 1e-10;

    fn approx_eq(a: f64, b: f64) -> bool {
        (a - b).abs() < EPSILON
    }

    #[test]
    fn test_trigonometry_identities() {
        // Test sin^2(x) + cos^2(x) = 1 for various angles
        for angle_deg in [0.0, 30.0, 45.0, 60.0, 90.0, 180.0, 270.0, 360.0] {
            let angle_rad = trq_to_radians(angle_deg);
            let sin_val = trq_sin(angle_rad);
            let cos_val = trq_cos(angle_rad);
            let sum = sin_val * sin_val + cos_val * cos_val;
            assert!(
                approx_eq(sum, 1.0),
                "sin^2 + cos^2 != 1 for {} degrees",
                angle_deg
            );
        }

        // Test tan(x) = sin(x) / cos(x)
        let angle = trq_to_radians(45.0);
        let tan_val = trq_tan(angle);
        let sin_cos = trq_sin(angle) / trq_cos(angle);
        assert!(approx_eq(tan_val, sin_cos));
    }

    #[test]
    fn test_inverse_functions() {
        // Test that asin(sin(x)) = x for x in [-pi/2, pi/2]
        let x = 0.5;
        assert!(approx_eq(trq_asin(trq_sin(x)), x));

        // Test that acos(cos(x)) = x for x in [0, pi]
        let y = 1.0;
        assert!(approx_eq(trq_acos(trq_cos(y)), y));

        // Test that atan(tan(x)) = x for x in (-pi/2, pi/2)
        let z = 0.7;
        assert!(approx_eq(trq_atan(trq_tan(z)), z));
    }

    #[test]
    fn test_exp_log_relationship() {
        // Test log(exp(x)) = x
        for x in [0.0, 1.0, 2.0, 5.0, 10.0] {
            assert!(approx_eq(trq_log(trq_exp(x)), x));
        }

        // Test exp(log(x)) = x for x > 0
        for x in [0.5, 1.0, 2.0, trq_e(), 10.0] {
            assert!(approx_eq(trq_exp(trq_log(x)), x));
        }
    }

    #[test]
    fn test_constants() {
        // Verify pi and e are correct
        assert!(approx_eq(trq_pi(), std::f64::consts::PI));
        assert!(approx_eq(trq_e(), std::f64::consts::E));
    }
}

// ============================================================================
// Runtime Initialization Workflow Tests
// ============================================================================

mod runtime_workflow {
    use super::*;

    #[test]
    fn test_runtime_init_cleanup() {
        // Initialize runtime (should be idempotent)
        trq_runtime_init();
        trq_runtime_init(); // Call twice should be safe

        // Get version
        let version = trq_version();
        assert!(!version.is_null());
        unsafe {
            assert!((*version).len > 0);
            trq_release(version as *mut u8);
        }

        // Cleanup (should be idempotent)
        trq_runtime_cleanup();
        trq_runtime_cleanup(); // Call twice should be safe
    }

    #[test]
    fn test_environment_variables() {
        let key = "TARQEEM_TEST_VAR";
        let value = "قيمة الاختبار";

        let key_str = trq_string_new(key.as_ptr(), key.len() as i64);
        let value_str = trq_string_new(value.as_ptr(), value.len() as i64);

        // Set environment variable
        assert!(trq_env_set(key_str, value_str));

        // Get and verify
        let retrieved = trq_env_get(key_str);
        assert!(!retrieved.is_null());
        assert!(trq_string_equals(value_str, retrieved));

        // Remove
        assert!(trq_env_remove(key_str));

        // Should no longer exist
        let after_remove = trq_env_get(key_str);
        unsafe {
            if !after_remove.is_null() {
                assert_eq!((*after_remove).len, 0);
                trq_release(after_remove as *mut u8);
            }
        }

        unsafe {
            trq_release(retrieved as *mut u8);
            trq_release(value_str as *mut u8);
            trq_release(key_str as *mut u8);
        }
    }
}
