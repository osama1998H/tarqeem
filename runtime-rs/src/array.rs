//! Array operations for Tarqeem runtime
//!
//! Implements dynamic arrays with reference counting.
//! This module provides all array manipulation functions needed by
//! compiled, JIT, and interpreted Tarqeem programs.

use crate::memory::trq_alloc;
use crate::types::TrqArray;
use std::ptr;

/// Initial capacity for new arrays
const ARRAY_INITIAL_CAP: i64 = 8;

/// Growth factor for array resizing
const ARRAY_GROWTH_FACTOR: i64 = 2;

// ============================================================================
// Internal Helpers
// ============================================================================

/// Allocate a TrqArray struct and its data buffer.
///
/// The TrqArray struct is allocated via `trq_alloc` (reference-counted).
/// The data buffer is allocated via `libc::malloc` (NOT reference-counted).
///
/// # Safety
/// Caller must ensure `len >= 0` and `elem_size > 0`.
unsafe fn allocate_array(len: i64, elem_size: i64) -> *mut TrqArray {
    // 1. Allocate TrqArray struct via trq_alloc (reference-counted)
    let arr_ptr = trq_alloc(std::mem::size_of::<TrqArray>() as i64) as *mut TrqArray;
    if arr_ptr.is_null() {
        return ptr::null_mut();
    }

    // 2. Calculate capacity (at least ARRAY_INITIAL_CAP)
    let cap = if len > ARRAY_INITIAL_CAP {
        len
    } else {
        ARRAY_INITIAL_CAP
    };
    let data_size = (cap * elem_size) as usize;

    // 3. Allocate data buffer via malloc (NOT reference-counted)
    let data_ptr = libc::malloc(data_size) as *mut u8;
    if data_ptr.is_null() {
        // Free the struct allocation
        crate::memory::trq_free(arr_ptr as *mut u8);
        return ptr::null_mut();
    }

    // 4. Zero-initialize the data buffer
    libc::memset(data_ptr as *mut libc::c_void, 0, data_size);

    // 5. Initialize struct fields
    (*arr_ptr).len = len;
    (*arr_ptr).cap = cap;
    (*arr_ptr).elem_size = elem_size;
    (*arr_ptr).data = data_ptr;

    arr_ptr
}

// ============================================================================
// Array Creation
// ============================================================================

/// Create a new array with the given length and element size.
///
/// # Parameters
/// - `len`: Initial number of elements (can be 0)
/// - `elem_size`: Size of each element in bytes
///
/// # Returns
/// Pointer to new TrqArray, or NULL on allocation failure.
///
/// # Memory
/// - The returned TrqArray is reference-counted (starts at refcount=1)
/// - The data buffer is allocated separately via malloc
/// - Use `trq_release()` to free when done
#[no_mangle]
pub extern "C" fn trq_array_new(len: i64, elem_size: i64) -> *mut TrqArray {
    let len = if len < 0 { 0 } else { len };
    let elem_size = if elem_size <= 0 {
        std::mem::size_of::<*mut u8>() as i64
    } else {
        elem_size
    };

    unsafe { allocate_array(len, elem_size) }
}

// ============================================================================
// Array Access
// ============================================================================

/// Get the length (number of elements) of an array.
///
/// # Parameters
/// - `arr`: Pointer to the array
///
/// # Returns
/// Number of elements, or 0 if array is NULL.
#[no_mangle]
pub extern "C" fn trq_array_len(arr: *const TrqArray) -> i64 {
    if arr.is_null() {
        return 0;
    }
    unsafe { (*arr).len }
}

/// Get a pointer to the element at the given index.
///
/// # Parameters
/// - `arr`: Pointer to the array
/// - `index`: Zero-based index
///
/// # Returns
/// Pointer to the element, or NULL if array is NULL or index is out of bounds.
///
/// # Errors
/// Prints an error message to stderr if:
/// - Array is NULL
/// - Index is out of bounds
#[no_mangle]
pub extern "C" fn trq_array_get(arr: *const TrqArray, index: i64) -> *mut u8 {
    if arr.is_null() {
        eprintln!(
            "Error: Array access on null array / خطأ: الوصول إلى مصفوفة فارغة"
        );
        return ptr::null_mut();
    }

    unsafe {
        let data = (*arr).data;
        if data.is_null() {
            eprintln!(
                "Error: Array access on null array / خطأ: الوصول إلى مصفوفة فارغة"
            );
            return ptr::null_mut();
        }

        let len = (*arr).len;
        if index < 0 || index >= len {
            eprintln!(
                "Error: Array index out of bounds: {} (length: {}) / خطأ: فهرس المصفوفة خارج الحدود",
                index, len
            );
            return ptr::null_mut();
        }

        data.add((index * (*arr).elem_size) as usize)
    }
}

/// Set the element at the given index.
///
/// # Parameters
/// - `arr`: Pointer to the array
/// - `index`: Zero-based index
/// - `value`: Pointer to the value to copy
///
/// # Errors
/// Prints an error message to stderr if:
/// - Array is NULL
/// - Index is out of bounds
#[no_mangle]
pub extern "C" fn trq_array_set(arr: *mut TrqArray, index: i64, value: *const u8) {
    if arr.is_null() {
        eprintln!(
            "Error: Array access on null array / خطأ: الوصول إلى مصفوفة فارغة"
        );
        return;
    }

    unsafe {
        let data = (*arr).data;
        if data.is_null() {
            eprintln!(
                "Error: Array access on null array / خطأ: الوصول إلى مصفوفة فارغة"
            );
            return;
        }

        let len = (*arr).len;
        if index < 0 || index >= len {
            eprintln!(
                "Error: Array index out of bounds: {} (length: {}) / خطأ: فهرس المصفوفة خارج الحدود",
                index, len
            );
            return;
        }

        let elem_size = (*arr).elem_size as usize;
        let dest = data.add((index * (*arr).elem_size) as usize);
        libc::memcpy(
            dest as *mut libc::c_void,
            value as *const libc::c_void,
            elem_size,
        );
    }
}

// ============================================================================
// Array Modification
// ============================================================================

/// Ensure the array has capacity for at least `new_cap` elements.
///
/// Uses 2x growth factor when expanding.
///
/// # Parameters
/// - `arr`: Pointer to the array
/// - `new_cap`: Required minimum capacity
///
/// # Returns
/// `true` if successful (capacity is now >= new_cap), `false` on failure.
#[no_mangle]
pub extern "C" fn trq_array_ensure_capacity(arr: *mut TrqArray, new_cap: i64) -> bool {
    if arr.is_null() {
        return false;
    }

    unsafe {
        let current_cap = (*arr).cap;
        if new_cap <= current_cap {
            return true;
        }

        // Calculate new capacity using growth factor
        let mut cap = current_cap;
        while cap < new_cap {
            cap *= ARRAY_GROWTH_FACTOR;
        }

        let elem_size = (*arr).elem_size;
        let new_size = (cap * elem_size) as usize;

        // Reallocate the data buffer
        let new_data = libc::realloc((*arr).data as *mut libc::c_void, new_size) as *mut u8;
        if new_data.is_null() {
            return false;
        }

        // Zero-initialize the new portion
        let old_size = (current_cap * elem_size) as usize;
        libc::memset(
            new_data.add(old_size) as *mut libc::c_void,
            0,
            new_size - old_size,
        );

        (*arr).data = new_data;
        (*arr).cap = cap;
        true
    }
}

/// Append an element to the end of the array.
///
/// Grows the array if necessary.
///
/// # Parameters
/// - `arr`: Pointer to the array
/// - `value`: Pointer to the value to append
/// - `elem_size`: Size of the element (use 0 to use array's stored elem_size)
///
/// # Errors
/// Prints an error message to stderr if:
/// - Array is NULL
/// - Failed to grow the array
#[no_mangle]
pub extern "C" fn trq_array_push(arr: *mut TrqArray, value: *const u8, _elem_size: i64) {
    if arr.is_null() {
        eprintln!(
            "Error: Push on null array / خطأ: إضافة إلى مصفوفة فارغة"
        );
        return;
    }

    unsafe {
        // Note: elem_size parameter is ignored; we always use the array's stored elem_size
        // This matches the C implementation behavior

        // Ensure we have capacity for one more element
        if !trq_array_ensure_capacity(arr, (*arr).len + 1) {
            eprintln!(
                "Error: Failed to grow array / خطأ: فشل في توسيع المصفوفة"
            );
            return;
        }

        // Copy the value to the end
        let dest = (*arr).data.add(((*arr).len * (*arr).elem_size) as usize);
        libc::memcpy(
            dest as *mut libc::c_void,
            value as *const libc::c_void,
            (*arr).elem_size as usize,
        );
        (*arr).len += 1;
    }
}

/// Remove and return a pointer to the last element.
///
/// # Parameters
/// - `arr`: Pointer to the array
///
/// # Returns
/// Pointer to the last element (still in array memory), or NULL if array is empty.
///
/// # Note
/// The returned pointer is valid until the next array modification.
///
/// # Errors
/// Prints an error message to stderr if:
/// - Array is NULL or empty
#[no_mangle]
pub extern "C" fn trq_array_pop(arr: *mut TrqArray) -> *mut u8 {
    if arr.is_null() {
        eprintln!(
            "Error: Pop from empty array / خطأ: إزالة من مصفوفة فارغة"
        );
        return ptr::null_mut();
    }

    unsafe {
        if (*arr).len == 0 {
            eprintln!(
                "Error: Pop from empty array / خطأ: إزالة من مصفوفة فارغة"
            );
            return ptr::null_mut();
        }

        (*arr).len -= 1;
        (*arr).data.add(((*arr).len * (*arr).elem_size) as usize)
    }
}

// ============================================================================
// Array Utilities
// ============================================================================

/// Clone an array (deep copy).
///
/// # Parameters
/// - `arr`: Pointer to the array to clone
/// - `elem_size`: Size of each element (use 0 to use array's stored elem_size)
///
/// # Returns
/// Pointer to the new cloned array, or NULL on failure.
///
/// # Memory
/// The returned array is a new allocation with refcount=1.
#[no_mangle]
pub extern "C" fn trq_array_clone(arr: *const TrqArray, elem_size: i64) -> *mut TrqArray {
    if arr.is_null() {
        return ptr::null_mut();
    }

    unsafe {
        let elem_size = if elem_size <= 0 {
            (*arr).elem_size
        } else {
            elem_size
        };

        let len = (*arr).len;
        let clone = trq_array_new(len, elem_size);
        if clone.is_null() {
            return ptr::null_mut();
        }

        // Copy the data
        libc::memcpy(
            (*clone).data as *mut libc::c_void,
            (*arr).data as *const libc::c_void,
            (len * elem_size) as usize,
        );

        clone
    }
}

/// Concatenate two arrays into a new array.
///
/// # Parameters
/// - `a`: First array (can be NULL)
/// - `b`: Second array (can be NULL)
/// - `elem_size`: Size of each element (use 0 to use array's stored elem_size)
///
/// # Returns
/// Pointer to a new array containing elements from both arrays.
///
/// # Memory
/// The returned array is a new allocation with refcount=1.
#[no_mangle]
pub extern "C" fn trq_array_concat(
    a: *const TrqArray,
    b: *const TrqArray,
    elem_size: i64,
) -> *mut TrqArray {
    if a.is_null() && b.is_null() {
        let elem_size = if elem_size > 0 {
            elem_size
        } else {
            std::mem::size_of::<*mut u8>() as i64
        };
        return trq_array_new(0, elem_size);
    }

    unsafe {
        let elem_size = if elem_size <= 0 {
            if !a.is_null() {
                (*a).elem_size
            } else {
                (*b).elem_size
            }
        } else {
            elem_size
        };

        let len_a = if a.is_null() { 0 } else { (*a).len };
        let len_b = if b.is_null() { 0 } else { (*b).len };
        let total_len = len_a + len_b;

        let result = trq_array_new(total_len, elem_size);
        if result.is_null() {
            return ptr::null_mut();
        }

        // Copy data from a
        if !a.is_null() && len_a > 0 {
            libc::memcpy(
                (*result).data as *mut libc::c_void,
                (*a).data as *const libc::c_void,
                (len_a * elem_size) as usize,
            );
        }

        // Copy data from b
        if !b.is_null() && len_b > 0 {
            let dest = (*result).data.add((len_a * elem_size) as usize);
            libc::memcpy(
                dest as *mut libc::c_void,
                (*b).data as *const libc::c_void,
                (len_b * elem_size) as usize,
            );
        }

        result
    }
}

/// Extract a slice of the array.
///
/// # Parameters
/// - `arr`: Pointer to the source array
/// - `start`: Start index (inclusive)
/// - `end`: End index (exclusive)
/// - `elem_size`: Size of each element (use 0 to use array's stored elem_size)
///
/// # Returns
/// Pointer to a new array containing the slice.
///
/// # Memory
/// The returned array is a new allocation with refcount=1.
#[no_mangle]
pub extern "C" fn trq_array_slice(
    arr: *const TrqArray,
    start: i64,
    end: i64,
    elem_size: i64,
) -> *mut TrqArray {
    if arr.is_null() {
        let elem_size = if elem_size > 0 {
            elem_size
        } else {
            std::mem::size_of::<*mut u8>() as i64
        };
        return trq_array_new(0, elem_size);
    }

    unsafe {
        let data = (*arr).data;
        if data.is_null() {
            let elem_size = if elem_size > 0 {
                elem_size
            } else {
                std::mem::size_of::<*mut u8>() as i64
            };
            return trq_array_new(0, elem_size);
        }

        let elem_size = if elem_size <= 0 {
            (*arr).elem_size
        } else {
            elem_size
        };

        // Clamp bounds
        let len = (*arr).len;
        let start = if start < 0 { 0 } else { start };
        let end = if end > len { len } else { end };

        if start >= end {
            return trq_array_new(0, elem_size);
        }

        let slice_len = end - start;
        let result = trq_array_new(slice_len, elem_size);
        if result.is_null() {
            return ptr::null_mut();
        }

        // Copy the slice data
        let src = data.add((start * elem_size) as usize);
        libc::memcpy(
            (*result).data as *mut libc::c_void,
            src as *const libc::c_void,
            (slice_len * elem_size) as usize,
        );

        result
    }
}

/// Free the internal data buffer of an array.
///
/// This is an internal helper function. After calling this,
/// the array struct is still valid but has no data.
///
/// # Parameters
/// - `arr`: Pointer to the array
#[no_mangle]
pub extern "C" fn trq_array_free_data(arr: *mut TrqArray) {
    if arr.is_null() {
        return;
    }

    unsafe {
        if !(*arr).data.is_null() {
            libc::free((*arr).data as *mut libc::c_void);
            (*arr).data = ptr::null_mut();
            (*arr).len = 0;
            (*arr).cap = 0;
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_array_new() {
        let arr = trq_array_new(5, 8);
        assert!(!arr.is_null());
        unsafe {
            assert_eq!((*arr).len, 5);
            assert!((*arr).cap >= 5);
            assert_eq!((*arr).elem_size, 8);
            assert!(!(*arr).data.is_null());
            trq_array_free_data(arr);
            crate::memory::trq_release(arr as *mut u8);
        }
    }

    #[test]
    fn test_array_new_negative_len() {
        let arr = trq_array_new(-5, 8);
        assert!(!arr.is_null());
        unsafe {
            assert_eq!((*arr).len, 0); // Clamped to 0
            trq_array_free_data(arr);
            crate::memory::trq_release(arr as *mut u8);
        }
    }

    #[test]
    fn test_array_len() {
        let arr = trq_array_new(10, 4);
        assert_eq!(trq_array_len(arr), 10);
        assert_eq!(trq_array_len(ptr::null()), 0);
        unsafe {
            trq_array_free_data(arr);
            crate::memory::trq_release(arr as *mut u8);
        }
    }

    #[test]
    fn test_array_get_set() {
        let arr = trq_array_new(3, std::mem::size_of::<i64>() as i64);
        assert!(!arr.is_null());

        unsafe {
            // Set values
            let val1: i64 = 100;
            let val2: i64 = 200;
            let val3: i64 = 300;

            trq_array_set(arr, 0, &val1 as *const i64 as *const u8);
            trq_array_set(arr, 1, &val2 as *const i64 as *const u8);
            trq_array_set(arr, 2, &val3 as *const i64 as *const u8);

            // Get values
            let ptr0 = trq_array_get(arr, 0);
            let ptr1 = trq_array_get(arr, 1);
            let ptr2 = trq_array_get(arr, 2);

            assert!(!ptr0.is_null());
            assert!(!ptr1.is_null());
            assert!(!ptr2.is_null());

            assert_eq!(*(ptr0 as *const i64), 100);
            assert_eq!(*(ptr1 as *const i64), 200);
            assert_eq!(*(ptr2 as *const i64), 300);

            // Out of bounds
            assert!(trq_array_get(arr, 3).is_null());
            assert!(trq_array_get(arr, -1).is_null());

            trq_array_free_data(arr);
            crate::memory::trq_release(arr as *mut u8);
        }
    }

    #[test]
    fn test_array_push_pop() {
        let arr = trq_array_new(0, std::mem::size_of::<i64>() as i64);
        assert!(!arr.is_null());

        unsafe {
            // Push values
            let val1: i64 = 10;
            let val2: i64 = 20;
            let val3: i64 = 30;

            trq_array_push(arr, &val1 as *const i64 as *const u8, 0);
            assert_eq!((*arr).len, 1);

            trq_array_push(arr, &val2 as *const i64 as *const u8, 0);
            assert_eq!((*arr).len, 2);

            trq_array_push(arr, &val3 as *const i64 as *const u8, 0);
            assert_eq!((*arr).len, 3);

            // Pop values (LIFO order)
            let popped = trq_array_pop(arr);
            assert!(!popped.is_null());
            assert_eq!(*(popped as *const i64), 30);
            assert_eq!((*arr).len, 2);

            let popped = trq_array_pop(arr);
            assert_eq!(*(popped as *const i64), 20);
            assert_eq!((*arr).len, 1);

            let popped = trq_array_pop(arr);
            assert_eq!(*(popped as *const i64), 10);
            assert_eq!((*arr).len, 0);

            // Pop from empty
            let popped = trq_array_pop(arr);
            assert!(popped.is_null());

            trq_array_free_data(arr);
            crate::memory::trq_release(arr as *mut u8);
        }
    }

    #[test]
    fn test_array_clone() {
        let arr = trq_array_new(3, std::mem::size_of::<i64>() as i64);
        assert!(!arr.is_null());

        unsafe {
            // Set values
            let val1: i64 = 111;
            let val2: i64 = 222;
            let val3: i64 = 333;

            trq_array_set(arr, 0, &val1 as *const i64 as *const u8);
            trq_array_set(arr, 1, &val2 as *const i64 as *const u8);
            trq_array_set(arr, 2, &val3 as *const i64 as *const u8);

            // Clone
            let clone = trq_array_clone(arr, 0);
            assert!(!clone.is_null());
            assert_eq!((*clone).len, 3);

            // Verify values in clone
            let ptr0 = trq_array_get(clone, 0);
            let ptr1 = trq_array_get(clone, 1);
            let ptr2 = trq_array_get(clone, 2);

            assert_eq!(*(ptr0 as *const i64), 111);
            assert_eq!(*(ptr1 as *const i64), 222);
            assert_eq!(*(ptr2 as *const i64), 333);

            // Modify original, verify clone is independent
            let new_val: i64 = 999;
            trq_array_set(arr, 0, &new_val as *const i64 as *const u8);
            assert_eq!(*(trq_array_get(arr, 0) as *const i64), 999);
            assert_eq!(*(trq_array_get(clone, 0) as *const i64), 111); // Unchanged

            trq_array_free_data(arr);
            trq_array_free_data(clone);
            crate::memory::trq_release(arr as *mut u8);
            crate::memory::trq_release(clone as *mut u8);
        }
    }

    #[test]
    fn test_array_concat() {
        let elem_size = std::mem::size_of::<i64>() as i64;
        let a = trq_array_new(2, elem_size);
        let b = trq_array_new(3, elem_size);

        unsafe {
            // Set values in a
            let val1: i64 = 1;
            let val2: i64 = 2;
            trq_array_set(a, 0, &val1 as *const i64 as *const u8);
            trq_array_set(a, 1, &val2 as *const i64 as *const u8);

            // Set values in b
            let val3: i64 = 3;
            let val4: i64 = 4;
            let val5: i64 = 5;
            trq_array_set(b, 0, &val3 as *const i64 as *const u8);
            trq_array_set(b, 1, &val4 as *const i64 as *const u8);
            trq_array_set(b, 2, &val5 as *const i64 as *const u8);

            // Concat
            let result = trq_array_concat(a, b, 0);
            assert!(!result.is_null());
            assert_eq!((*result).len, 5);

            // Verify all values
            assert_eq!(*(trq_array_get(result, 0) as *const i64), 1);
            assert_eq!(*(trq_array_get(result, 1) as *const i64), 2);
            assert_eq!(*(trq_array_get(result, 2) as *const i64), 3);
            assert_eq!(*(trq_array_get(result, 3) as *const i64), 4);
            assert_eq!(*(trq_array_get(result, 4) as *const i64), 5);

            trq_array_free_data(a);
            trq_array_free_data(b);
            trq_array_free_data(result);
            crate::memory::trq_release(a as *mut u8);
            crate::memory::trq_release(b as *mut u8);
            crate::memory::trq_release(result as *mut u8);
        }
    }

    #[test]
    fn test_array_concat_null() {
        let elem_size = std::mem::size_of::<i64>() as i64;
        let a = trq_array_new(2, elem_size);

        unsafe {
            let val: i64 = 42;
            trq_array_set(a, 0, &val as *const i64 as *const u8);
            trq_array_set(a, 1, &val as *const i64 as *const u8);

            // Concat with null
            let result1 = trq_array_concat(a, ptr::null(), 0);
            assert_eq!((*result1).len, 2);

            let result2 = trq_array_concat(ptr::null(), a, 0);
            assert_eq!((*result2).len, 2);

            let result3 = trq_array_concat(ptr::null(), ptr::null(), elem_size);
            assert_eq!((*result3).len, 0);

            trq_array_free_data(a);
            trq_array_free_data(result1);
            trq_array_free_data(result2);
            trq_array_free_data(result3);
            crate::memory::trq_release(a as *mut u8);
            crate::memory::trq_release(result1 as *mut u8);
            crate::memory::trq_release(result2 as *mut u8);
            crate::memory::trq_release(result3 as *mut u8);
        }
    }

    #[test]
    fn test_array_slice() {
        let elem_size = std::mem::size_of::<i64>() as i64;
        let arr = trq_array_new(5, elem_size);

        unsafe {
            // Set values 0, 1, 2, 3, 4
            for i in 0..5 {
                let val: i64 = i;
                trq_array_set(arr, i, &val as *const i64 as *const u8);
            }

            // Slice [1, 4) -> [1, 2, 3]
            let slice = trq_array_slice(arr, 1, 4, 0);
            assert!(!slice.is_null());
            assert_eq!((*slice).len, 3);

            assert_eq!(*(trq_array_get(slice, 0) as *const i64), 1);
            assert_eq!(*(trq_array_get(slice, 1) as *const i64), 2);
            assert_eq!(*(trq_array_get(slice, 2) as *const i64), 3);

            // Empty slice
            let empty = trq_array_slice(arr, 3, 3, 0);
            assert_eq!((*empty).len, 0);

            // Out of bounds (clamped)
            let clamped = trq_array_slice(arr, -5, 100, 0);
            assert_eq!((*clamped).len, 5);

            trq_array_free_data(arr);
            trq_array_free_data(slice);
            trq_array_free_data(empty);
            trq_array_free_data(clamped);
            crate::memory::trq_release(arr as *mut u8);
            crate::memory::trq_release(slice as *mut u8);
            crate::memory::trq_release(empty as *mut u8);
            crate::memory::trq_release(clamped as *mut u8);
        }
    }

    #[test]
    fn test_array_ensure_capacity() {
        let arr = trq_array_new(0, 8);
        assert!(!arr.is_null());

        unsafe {
            let initial_cap = (*arr).cap;
            assert!(initial_cap >= ARRAY_INITIAL_CAP);

            // Request more capacity
            let success = trq_array_ensure_capacity(arr, 100);
            assert!(success);
            assert!((*arr).cap >= 100);

            // Request less capacity (no change needed)
            let old_cap = (*arr).cap;
            let success = trq_array_ensure_capacity(arr, 50);
            assert!(success);
            assert_eq!((*arr).cap, old_cap); // Unchanged

            trq_array_free_data(arr);
            crate::memory::trq_release(arr as *mut u8);
        }
    }

    #[test]
    fn test_array_free_data() {
        let arr = trq_array_new(5, 8);
        assert!(!arr.is_null());

        unsafe {
            assert!(!(*arr).data.is_null());
            assert_eq!((*arr).len, 5);

            trq_array_free_data(arr);

            assert!((*arr).data.is_null());
            assert_eq!((*arr).len, 0);
            assert_eq!((*arr).cap, 0);

            // Free data again should be safe
            trq_array_free_data(arr);

            crate::memory::trq_release(arr as *mut u8);
        }
    }

    #[test]
    fn test_array_null_safety() {
        // All functions should handle NULL gracefully
        assert_eq!(trq_array_len(ptr::null()), 0);
        assert!(trq_array_get(ptr::null(), 0).is_null());
        trq_array_set(ptr::null_mut(), 0, ptr::null()); // Should not crash
        assert!(!trq_array_ensure_capacity(ptr::null_mut(), 10));
        trq_array_push(ptr::null_mut(), ptr::null(), 0); // Should not crash
        assert!(trq_array_pop(ptr::null_mut()).is_null());
        assert!(trq_array_clone(ptr::null(), 0).is_null());
        trq_array_free_data(ptr::null_mut()); // Should not crash
    }
}
