//! Internal helper functions for Tarqeem runtime
//!
//! This module provides shared allocation helpers used by multiple modules
//! (crypto, compress, etc.) to avoid code duplication.

use crate::memory::{trq_alloc, trq_free};
use crate::types::{TrqArray, TrqString};
use std::ptr;

/// Allocate a TrqString struct and its data buffer
///
/// # Arguments
/// * `len` - Length of the string data buffer in bytes
///
/// # Returns
/// * Pointer to new TrqString, or NULL on failure
///
/// # Safety
/// This function is unsafe because it performs raw pointer operations.
/// The caller must ensure proper cleanup via reference counting.
///
/// # Memory Model
/// - The TrqString struct is allocated via `trq_alloc` (reference-counted)
/// - The `data` buffer is allocated via `libc::malloc` (NOT reference-counted)
/// - The data buffer is zero-initialized and null-terminated
pub(crate) unsafe fn allocate_string(len: i64) -> *mut TrqString {
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
        trq_free(str_ptr as *mut u8);
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
/// # Arguments
/// * `len` - Number of elements
/// * `elem_size` - Size of each element in bytes
///
/// # Returns
/// * Pointer to new TrqArray, or NULL on failure
///
/// # Safety
/// This function is unsafe because it performs raw pointer operations.
/// The caller must ensure proper cleanup via reference counting.
///
/// # Memory Model
/// - The TrqArray struct is allocated via `trq_alloc` (reference-counted)
/// - The `data` buffer is allocated via `libc::malloc` (NOT reference-counted)
/// - The data buffer is zero-initialized
pub(crate) unsafe fn allocate_array(len: i64, elem_size: i64) -> *mut TrqArray {
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
    let data_ptr = if data_size > 0 {
        libc::malloc(data_size as usize) as *mut u8
    } else {
        ptr::null_mut()
    };

    if data_size > 0 && data_ptr.is_null() {
        // Free the struct since data allocation failed
        trq_free(arr_ptr as *mut u8);
        return ptr::null_mut();
    }

    // Zero-initialize the data buffer if allocated
    if !data_ptr.is_null() {
        ptr::write_bytes(data_ptr, 0, data_size as usize);
    }

    // Initialize the struct
    (*arr_ptr).len = len;
    (*arr_ptr).cap = len;
    (*arr_ptr).elem_size = elem_size;
    (*arr_ptr).data = data_ptr;

    arr_ptr
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allocate_string() {
        unsafe {
            let s = allocate_string(10);
            assert!(!s.is_null());
            assert_eq!((*s).len, 10);
            assert_eq!((*s).cap, 10);
            assert!(!(*s).data.is_null());
            // Data should be zero-initialized
            for i in 0..10 {
                assert_eq!(*(*s).data.add(i), 0);
            }
        }
    }

    #[test]
    fn test_allocate_string_negative() {
        unsafe {
            let s = allocate_string(-1);
            assert!(s.is_null());
        }
    }

    #[test]
    fn test_allocate_array() {
        unsafe {
            let arr = allocate_array(5, 8);
            assert!(!arr.is_null());
            assert_eq!((*arr).len, 5);
            assert_eq!((*arr).cap, 5);
            assert_eq!((*arr).elem_size, 8);
            assert!(!(*arr).data.is_null());
        }
    }

    #[test]
    fn test_allocate_array_negative() {
        unsafe {
            let arr = allocate_array(-1, 8);
            assert!(arr.is_null());
        }
    }

    #[test]
    fn test_allocate_array_zero_elem_size() {
        unsafe {
            let arr = allocate_array(5, 0);
            assert!(arr.is_null());
        }
    }
}
