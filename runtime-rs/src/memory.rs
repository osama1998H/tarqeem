//! Memory management functions for Tarqeem runtime
//!
//! This module implements reference-counted memory allocation that is
//! ABI-compatible with the C runtime. All functions use the C calling
//! convention and can be linked with compiled Tarqeem programs.
//!
//! # Memory Layout
//!
//! All allocations have a RefCountHeader prepended:
//!
//! ```text
//! ┌─────────────────────────────────────┐
//! │    RefCountHeader (16 bytes)        │
//! ├─────────────┬───────────────────────┤
//! │  refcount   │  size                 │
//! │  (int64_t)  │  (int64_t)            │
//! ├─────────────────────────────────────┤
//! │  User Data (N bytes)                │
//! │  (zero-initialized)                 │
//! └─────────────────────────────────────┘
//!   ^                         ^
//!   |                         |
//!   malloc'd ptr          returned ptr
//! ```

use crate::types::{RefCountHeader, HEADER_SIZE};
use std::alloc::{alloc_zeroed, dealloc, realloc, Layout};
use std::ptr;

/// Get the header pointer from a user data pointer
///
/// # Safety
/// The pointer must have been allocated by `trq_alloc`
#[inline]
unsafe fn get_header(ptr: *mut u8) -> *mut RefCountHeader {
    ptr.sub(HEADER_SIZE) as *mut RefCountHeader
}

/// Get the user data pointer from a header pointer
#[inline]
fn get_data(header: *mut RefCountHeader) -> *mut u8 {
    unsafe { (header as *mut u8).add(HEADER_SIZE) }
}

/// Allocate memory with a reference count header
///
/// # Arguments
/// * `size` - Size of the user data portion in bytes
///
/// # Returns
/// * Pointer to the user data (after the header), or NULL on failure
///
/// # Behavior
/// - Allocates `HEADER_SIZE + size` bytes
/// - Initializes refcount to 1
/// - Zero-initializes the user data
/// - Returns NULL if size <= 0 or allocation fails
///
/// # C Equivalent
/// ```c
/// void* trq_alloc(int64_t size);
/// ```
#[no_mangle]
pub extern "C" fn trq_alloc(size: i64) -> *mut u8 {
    // Validate size
    if size <= 0 {
        return ptr::null_mut();
    }

    let total_size = HEADER_SIZE + size as usize;

    // Create layout for the allocation
    let layout = match Layout::from_size_align(total_size, 8) {
        Ok(layout) => layout,
        Err(_) => {
            eprintln!("خطأ: فشل في إنشاء تخطيط الذاكرة ({} بايت)", total_size);
            return ptr::null_mut();
        }
    };

    unsafe {
        // Allocate zeroed memory
        let header_ptr = alloc_zeroed(layout) as *mut RefCountHeader;

        if header_ptr.is_null() {
            eprintln!("خطأ: فشل تخصيص الذاكرة ({} بايت)", total_size);
            return ptr::null_mut();
        }

        // Initialize the header
        (*header_ptr).refcount = 1;
        (*header_ptr).size = size;

        // Return pointer to user data (after header)
        get_data(header_ptr)
    }
}

/// Reallocate memory while preserving the header
///
/// # Arguments
/// * `ptr` - Pointer to existing user data, or NULL
/// * `new_size` - New size for the user data portion
///
/// # Returns
/// * Pointer to the new user data, or NULL on failure
///
/// # Behavior
/// - If ptr is NULL, behaves like trq_alloc(new_size)
/// - If new_size <= 0, frees the memory and returns NULL
/// - Preserves refcount during reallocation
/// - Zero-initializes any new memory if growing
///
/// # C Equivalent
/// ```c
/// void* trq_realloc(void* ptr, int64_t new_size);
/// ```
#[no_mangle]
pub extern "C" fn trq_realloc(ptr: *mut u8, new_size: i64) -> *mut u8 {
    // If ptr is NULL, behave like trq_alloc
    if ptr.is_null() {
        return trq_alloc(new_size);
    }

    // If new_size <= 0, free and return NULL
    if new_size <= 0 {
        trq_free(ptr);
        return ptr::null_mut();
    }

    unsafe {
        let header_ptr = get_header(ptr);
        let old_size = (*header_ptr).size;
        let old_total = HEADER_SIZE + old_size as usize;
        let new_total = HEADER_SIZE + new_size as usize;

        // Create layouts for reallocation
        let old_layout = match Layout::from_size_align(old_total, 8) {
            Ok(layout) => layout,
            Err(_) => return ptr::null_mut(),
        };

        // Reallocate
        let new_header_ptr = realloc(header_ptr as *mut u8, old_layout, new_total);

        if new_header_ptr.is_null() {
            eprintln!("خطأ: فشل إعادة تخصيص الذاكرة ({} بايت)",
                new_total
            );
            return ptr::null_mut();
        }

        let new_header = new_header_ptr as *mut RefCountHeader;

        // Zero-initialize new memory if growing
        if new_size > old_size {
            let new_data = get_data(new_header);
            let start_offset = old_size as usize;
            let bytes_to_zero = (new_size - old_size) as usize;
            ptr::write_bytes(new_data.add(start_offset), 0, bytes_to_zero);
        }

        // Update size in header (refcount is preserved)
        (*new_header).size = new_size;

        get_data(new_header)
    }
}

/// Free memory (raw free, no reference count check)
///
/// # Arguments
/// * `ptr` - Pointer to user data, or NULL
///
/// # Behavior
/// - If ptr is NULL, does nothing
/// - Frees the entire allocation (header + data)
/// - Does NOT check reference count
///
/// # Warning
/// Use `trq_release` for normal memory management.
/// This function should only be used internally.
///
/// # C Equivalent
/// ```c
/// void trq_free(void* ptr);
/// ```
#[no_mangle]
pub extern "C" fn trq_free(ptr: *mut u8) {
    if ptr.is_null() {
        return;
    }

    unsafe {
        let header_ptr = get_header(ptr);
        let size = (*header_ptr).size;
        let total_size = HEADER_SIZE + size as usize;

        let layout = match Layout::from_size_align(total_size, 8) {
            Ok(layout) => layout,
            Err(_) => return, // Can't free if we can't recreate the layout
        };

        dealloc(header_ptr as *mut u8, layout);
    }
}

/// Increment the reference count
///
/// # Arguments
/// * `ptr` - Pointer to user data, or NULL
///
/// # Behavior
/// - If ptr is NULL, does nothing
/// - Increments refcount by 1
///
/// # C Equivalent
/// ```c
/// void trq_retain(void* ptr);
/// ```
#[no_mangle]
pub extern "C" fn trq_retain(ptr: *mut u8) {
    if ptr.is_null() {
        return;
    }

    unsafe {
        let header_ptr = get_header(ptr);
        (*header_ptr).refcount += 1;
    }
}

/// Decrement the reference count, freeing if it reaches zero
///
/// # Arguments
/// * `ptr` - Pointer to user data, or NULL
///
/// # Behavior
/// - If ptr is NULL, does nothing
/// - Decrements refcount by 1
/// - If refcount reaches 0 or less, frees the memory
///
/// # C Equivalent
/// ```c
/// void trq_release(void* ptr);
/// ```
#[no_mangle]
pub extern "C" fn trq_release(ptr: *mut u8) {
    if ptr.is_null() {
        return;
    }

    unsafe {
        let header_ptr = get_header(ptr);
        (*header_ptr).refcount -= 1;

        if (*header_ptr).refcount <= 0 {
            trq_free(ptr);
        }
    }
}

/// Get the current reference count
///
/// # Arguments
/// * `ptr` - Pointer to user data, or NULL
///
/// # Returns
/// * The current reference count, or 0 if ptr is NULL
///
/// # C Equivalent
/// ```c
/// int64_t trq_refcount(void* ptr);
/// ```
#[no_mangle]
pub extern "C" fn trq_refcount(ptr: *const u8) -> i64 {
    if ptr.is_null() {
        return 0;
    }

    unsafe {
        let header_ptr = (ptr as *mut u8).sub(HEADER_SIZE) as *const RefCountHeader;
        (*header_ptr).refcount
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alloc_and_free() {
        let ptr = trq_alloc(100);
        assert!(!ptr.is_null());

        // Check refcount is 1
        assert_eq!(trq_refcount(ptr), 1);

        // Free the memory
        trq_free(ptr);
    }

    #[test]
    fn test_alloc_zero_size() {
        let ptr = trq_alloc(0);
        assert!(ptr.is_null());
    }

    #[test]
    fn test_alloc_negative_size() {
        let ptr = trq_alloc(-1);
        assert!(ptr.is_null());
    }

    #[test]
    fn test_refcount_null() {
        assert_eq!(trq_refcount(ptr::null()), 0);
    }

    #[test]
    fn test_retain_release() {
        let ptr = trq_alloc(100);
        assert_eq!(trq_refcount(ptr), 1);

        // Retain increases refcount
        trq_retain(ptr);
        assert_eq!(trq_refcount(ptr), 2);

        trq_retain(ptr);
        assert_eq!(trq_refcount(ptr), 3);

        // Release decreases refcount
        trq_release(ptr);
        assert_eq!(trq_refcount(ptr), 2);

        trq_release(ptr);
        assert_eq!(trq_refcount(ptr), 1);

        // Final release should free
        trq_release(ptr);
        // Note: We can't check after this as memory is freed
    }

    #[test]
    fn test_retain_null() {
        // Should not crash
        trq_retain(ptr::null_mut());
    }

    #[test]
    fn test_release_null() {
        // Should not crash
        trq_release(ptr::null_mut());
    }

    #[test]
    fn test_free_null() {
        // Should not crash
        trq_free(ptr::null_mut());
    }

    #[test]
    fn test_realloc_from_null() {
        // realloc(NULL, size) should behave like alloc(size)
        let ptr = trq_realloc(ptr::null_mut(), 100);
        assert!(!ptr.is_null());
        assert_eq!(trq_refcount(ptr), 1);
        trq_free(ptr);
    }

    #[test]
    fn test_realloc_to_zero() {
        // realloc(ptr, 0) should free and return NULL
        let ptr = trq_alloc(100);
        let result = trq_realloc(ptr, 0);
        assert!(result.is_null());
    }

    #[test]
    fn test_realloc_grow() {
        let ptr = trq_alloc(100);

        // Write some data
        unsafe {
            *ptr = 42;
            *ptr.add(99) = 99;
        }

        // Grow the allocation
        let new_ptr = trq_realloc(ptr, 200);
        assert!(!new_ptr.is_null());

        // Check data is preserved
        unsafe {
            assert_eq!(*new_ptr, 42);
            assert_eq!(*new_ptr.add(99), 99);
            // New memory should be zeroed
            assert_eq!(*new_ptr.add(150), 0);
        }

        // Refcount should be preserved
        assert_eq!(trq_refcount(new_ptr), 1);

        trq_free(new_ptr);
    }

    #[test]
    fn test_realloc_shrink() {
        let ptr = trq_alloc(200);

        // Write some data
        unsafe {
            *ptr = 42;
            *ptr.add(50) = 50;
        }

        // Shrink the allocation
        let new_ptr = trq_realloc(ptr, 100);
        assert!(!new_ptr.is_null());

        // Check data is preserved
        unsafe {
            assert_eq!(*new_ptr, 42);
            assert_eq!(*new_ptr.add(50), 50);
        }

        // Refcount should be preserved
        assert_eq!(trq_refcount(new_ptr), 1);

        trq_free(new_ptr);
    }

    #[test]
    fn test_data_is_zeroed() {
        let ptr = trq_alloc(100);
        assert!(!ptr.is_null());

        // All bytes should be zero
        unsafe {
            for i in 0..100 {
                assert_eq!(*ptr.add(i), 0, "Byte {} is not zero", i);
            }
        }

        trq_free(ptr);
    }

    #[test]
    fn test_header_size_constant() {
        // Ensure our header size matches expected
        assert_eq!(HEADER_SIZE, 16);
    }
}
