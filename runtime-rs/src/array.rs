//! Array operations for the Tarqeem runtime.
//!
//! Implements dynamic arrays for compiled, JIT, and interpreted programs.
//!
//! # Memory Management
//!
//! Each `TrqArray` is reference-counted and allocated with `trq_alloc()`,
//! while its data buffer is allocated with `libc::malloc()`. The mixed
//! allocation model is intentional: it preserves C ABI compatibility and
//! allows direct `libc::realloc()` for array growth.
//!
//! A `TrqArray` exclusively owns its data buffer. Use `trq_array_clone()`
//! when an independent copy is required, and free the buffer with
//! `trq_array_free_data()` before releasing the array.
//!
//! # Safety
//!
//! Pointers returned by `trq_array_get()` and `trq_array_pop()` are borrowed
//! and become invalid after array modification or deallocation. Do not free
//! the data while such pointers are still in use.
//!
//! When using `trq_retain()`/`trq_release()`, only the final release should
//! free the array's data buffer.

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
/// # Safety
///
/// **The returned pointer is a borrowed reference into the array's internal buffer.**
/// This pointer becomes invalid (dangling) after ANY of the following operations:
///
/// - `trq_array_push()` - may reallocate the buffer
/// - `trq_array_pop()` - modifies array length
/// - `trq_array_ensure_capacity()` - may reallocate the buffer
/// - `trq_array_free_data()` - frees the buffer
/// - `trq_release()` on the array - may free the entire array
///
/// Callers MUST NOT store this pointer across array-modifying operations.
/// Copy the data if you need it to persist.
///
/// # Errors
/// Prints an error message to stderr if:
/// - Array is NULL
/// - Index is out of bounds
#[no_mangle]
pub extern "C" fn trq_array_get(arr: *const TrqArray, index: i64) -> *mut u8 {
    if arr.is_null() {
        eprintln!("Error: Array access on null array / خطأ: الوصول إلى مصفوفة فارغة");
        return ptr::null_mut();
    }

    unsafe {
        let data = (*arr).data;
        if data.is_null() {
            eprintln!("Error: Array access on null array / خطأ: الوصول إلى مصفوفة فارغة");
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
        eprintln!("Error: Array access on null array / خطأ: الوصول إلى مصفوفة فارغة");
        return;
    }

    unsafe {
        let data = (*arr).data;
        if data.is_null() {
            eprintln!("Error: Array access on null array / خطأ: الوصول إلى مصفوفة فارغة");
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

        // Doubling cannot start from zero: `0 * 2` is `0`, so a zero-capacity
        // array spun here forever instead of growing. `trq_array_new` never
        // yields one, but `helpers::allocate_array` does — it sets `cap = len` —
        // so `الحق(نص_إلى_ثنائي("")، 5)` hung the process natively while both
        // interpreters answered.
        let mut cap = if current_cap > 0 {
            current_cap
        } else {
            ARRAY_INITIAL_CAP
        };
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
        eprintln!("Error: Push on null array / خطأ: إضافة إلى مصفوفة فارغة");
        return;
    }

    unsafe {
        // Note: elem_size parameter is ignored; we always use the array's stored elem_size
        // This matches the C implementation behavior

        // Ensure we have capacity for one more element
        if !trq_array_ensure_capacity(arr, (*arr).len + 1) {
            eprintln!("Error: Failed to grow array / خطأ: فشل في توسيع المصفوفة");
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
/// Pointer to the last element (still in array memory). Never NULL: a null or
/// empty array answers `POP_EMPTY_ZERO`.
///
/// # Safety
///
/// **The returned pointer is a borrowed reference into the array's internal buffer.**
/// This pointer becomes invalid (dangling) after ANY of the following operations:
///
/// - `trq_array_push()` - may reallocate the buffer
/// - `trq_array_pop()` - another pop changes the valid region
/// - `trq_array_ensure_capacity()` - may reallocate the buffer
/// - `trq_array_free_data()` - frees the buffer
/// - `trq_release()` on the array - may free the entire array
///
/// Callers MUST copy the data immediately if they need it to persist.
/// The element data remains in memory until overwritten by a subsequent push,
/// but relying on this is undefined behavior.
///
/// # Errors
/// None. The operation is total — see `POP_EMPTY_ZERO`.
#[no_mangle]
pub extern "C" fn trq_array_pop(arr: *mut TrqArray) -> *mut u8 {
    if arr.is_null() {
        return pop_empty_zero();
    }

    unsafe {
        if (*arr).len == 0 {
            return pop_empty_zero();
        }

        (*arr).len -= 1;
        (*arr).data.add(((*arr).len * (*arr).elem_size) as usize)
    }
}

/// Eight zero bytes handed out when there is nothing to pop: codegen lowers
/// `احذف_آخر` as `call` + unconditional `load`, so NULL here would segfault
/// natively where both interpreters answer a value; read at any element type
/// it gives that type's zero, so one buffer serves them all. Read-only by
/// contract — unlike `trq_array_get`'s pointer, nothing ever writes to it.
static POP_EMPTY_ZERO: [u64; 1] = [0];

fn pop_empty_zero() -> *mut u8 {
    POP_EMPTY_ZERO.as_ptr() as *mut u8
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
/// This is a cleanup function that frees the array's data buffer.
/// After calling this, the array struct is still valid but has no data
/// (len=0, cap=0, data=NULL).
///
/// # Parameters
/// - `arr`: Pointer to the array
///
/// # Safety
///
/// **This function MUST only be called by the sole owner of the array.**
///
/// If multiple references to the array exist (via `trq_retain()`), calling
/// this function will invalidate all other references' data pointers, leading
/// to use-after-free bugs.
///
/// ## Correct Usage Pattern
///
/// ```c
/// // Single owner pattern
/// TrqArray* arr = trq_array_new(10, 8);
/// // ... use array ...
/// trq_array_free_data(arr);  // Free data first
/// trq_release(arr);          // Then release struct
/// ```
///
/// ## Incorrect Usage (DO NOT DO)
///
/// ```c
/// TrqArray* arr = trq_array_new(10, 8);
/// trq_retain(arr);           // Now refcount = 2
/// trq_array_free_data(arr);  // WRONG! Other reference still exists
/// // Other code using arr now has dangling pointer!
/// ```
///
/// # When to Call
///
/// Call this function:
/// - Before the final `trq_release()` that will free the array struct
/// - Only when you are certain no other code holds pointers into the array
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

            // Pop from empty answers a readable zero, never NULL — codegen
            // loads the returned pointer unconditionally.
            let popped = trq_array_pop(arr);
            assert!(!popped.is_null());
            assert_eq!(*(popped as *const i64), 0);

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

    /// A zero-capacity array must still grow. `trq_array_new` never builds one,
    /// but `helpers::allocate_array` does, and `trq_string_to_bytes("")` returns
    /// exactly that — so this is the shape `الحق` meets from source. A regression
    /// **hangs** this test rather than failing it: the old growth loop doubled
    /// from `0` forever.
    #[test]
    fn test_array_push_onto_zero_capacity() {
        unsafe {
            let arr = crate::helpers::allocate_array(0, 8);
            assert!(!arr.is_null());
            assert_eq!((*arr).cap, 0);

            let value: i64 = 7;
            trq_array_push(arr, &value as *const i64 as *const u8, 8);

            assert_eq!((*arr).len, 1);
            assert!((*arr).cap >= 1);
            assert_eq!(*((*arr).data as *const i64), 7);

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
        let popped = trq_array_pop(ptr::null_mut());
        assert!(!popped.is_null());
        unsafe { assert_eq!(*(popped as *const i64), 0) };
        assert!(trq_array_clone(ptr::null(), 0).is_null());
        trq_array_free_data(ptr::null_mut()); // Should not crash
    }

    // ===== Phase 8: Additional Array Tests =====

    #[test]
    fn test_array_push_many() {
        let elem_size = std::mem::size_of::<i64>() as i64;
        let arr = trq_array_new(0, elem_size);

        unsafe {
            // Push 1000 elements to test capacity growth
            for i in 0..1000 {
                let val: i64 = i;
                trq_array_push(arr, &val as *const i64 as *const u8, elem_size);
            }

            assert_eq!(trq_array_len(arr), 1000);

            // Verify first and last elements
            assert_eq!(*(trq_array_get(arr, 0) as *const i64), 0);
            assert_eq!(*(trq_array_get(arr, 999) as *const i64), 999);

            // Verify capacity grew appropriately
            assert!((*arr).cap >= 1000);

            trq_array_free_data(arr);
            crate::memory::trq_release(arr as *mut u8);
        }
    }

    /// Popping an empty array is a legitimate answer, not a refusal: the
    /// compiler emits an unconditional `load` on the returned pointer, so NULL
    /// here would be a segfault where both interpreters answer a value.
    #[test]
    fn test_array_pop_empty_answers_zero_not_null() {
        let elem_size = std::mem::size_of::<i64>() as i64;
        let arr = trq_array_new(0, elem_size);

        unsafe {
            let result = trq_array_pop(arr);
            assert!(!result.is_null());
            assert_eq!(*(result as *const i64), 0);

            // The array is left alone; the length never goes negative.
            assert_eq!(trq_array_len(arr), 0);

            let val: i64 = 42;
            trq_array_push(arr, &val as *const i64 as *const u8, elem_size);
            assert_eq!(trq_array_len(arr), 1);

            let popped = trq_array_pop(arr);
            assert!(!popped.is_null());
            assert_eq!(*(popped as *const i64), 42);
            assert_eq!(trq_array_len(arr), 0);

            let empty_pop = trq_array_pop(arr);
            assert!(!empty_pop.is_null());
            assert_eq!(*(empty_pop as *const i64), 0);

            trq_array_free_data(arr);
            crate::memory::trq_release(arr as *mut u8);
        }
    }

    /// A float element reads 0.0 and a bool element reads false out of the same
    /// eight bytes, which is what lets one buffer serve every element type.
    #[test]
    fn test_array_pop_empty_zero_reads_as_every_element_type() {
        let arr = trq_array_new(0, std::mem::size_of::<f64>() as i64);

        unsafe {
            let popped = trq_array_pop(arr);
            assert_eq!(*(popped as *const f64), 0.0);
            assert_eq!(*(popped as *const u8), 0);
            assert!((*(popped as *const *const u8)).is_null());

            trq_array_free_data(arr);
            crate::memory::trq_release(arr as *mut u8);
        }
    }

    #[test]
    fn test_array_get_out_of_bounds() {
        let elem_size = std::mem::size_of::<i64>() as i64;
        let arr = trq_array_new(5, elem_size);

        unsafe {
            // Set values
            for i in 0..5 {
                let val: i64 = i * 10;
                trq_array_set(arr, i, &val as *const i64 as *const u8);
            }

            // Negative index should return null
            let neg_result = trq_array_get(arr, -1);
            assert!(neg_result.is_null());

            // Index at length should return null
            let at_len = trq_array_get(arr, 5);
            assert!(at_len.is_null());

            // Index beyond length should return null
            let beyond = trq_array_get(arr, 100);
            assert!(beyond.is_null());

            // Valid indices should work
            assert_eq!(*(trq_array_get(arr, 0) as *const i64), 0);
            assert_eq!(*(trq_array_get(arr, 4) as *const i64), 40);

            trq_array_free_data(arr);
            crate::memory::trq_release(arr as *mut u8);
        }
    }

    #[test]
    fn test_array_slice_edge_cases() {
        let elem_size = std::mem::size_of::<i64>() as i64;
        let arr = trq_array_new(5, elem_size);

        unsafe {
            for i in 0..5 {
                let val: i64 = i;
                trq_array_set(arr, i, &val as *const i64 as *const u8);
            }

            // Slice with start == end -> empty
            let empty1 = trq_array_slice(arr, 2, 2, 0);
            assert_eq!((*empty1).len, 0);

            // Slice with start > end -> empty (should handle gracefully)
            let empty2 = trq_array_slice(arr, 4, 2, 0);
            assert_eq!((*empty2).len, 0);

            // Slice with negative start (should clamp to 0)
            let from_zero = trq_array_slice(arr, -10, 3, 0);
            assert_eq!((*from_zero).len, 3);
            assert_eq!(*(trq_array_get(from_zero, 0) as *const i64), 0);

            // Slice with end beyond length (should clamp to len)
            let to_end = trq_array_slice(arr, 2, 1000, 0);
            assert_eq!((*to_end).len, 3); // [2, 3, 4]
            assert_eq!(*(trq_array_get(to_end, 0) as *const i64), 2);
            assert_eq!(*(trq_array_get(to_end, 2) as *const i64), 4);

            trq_array_free_data(arr);
            trq_array_free_data(empty1);
            trq_array_free_data(empty2);
            trq_array_free_data(from_zero);
            trq_array_free_data(to_end);
            crate::memory::trq_release(arr as *mut u8);
            crate::memory::trq_release(empty1 as *mut u8);
            crate::memory::trq_release(empty2 as *mut u8);
            crate::memory::trq_release(from_zero as *mut u8);
            crate::memory::trq_release(to_end as *mut u8);
        }
    }
}
