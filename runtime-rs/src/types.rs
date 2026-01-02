//! FFI-compatible type definitions for Tarqeem runtime
//!
//! These types must match the C runtime exactly for ABI compatibility.
//! Used by JIT, interpreter, and compiled programs.

use std::ptr;

/// Header size in bytes - prepended to all reference-counted allocations
pub const HEADER_SIZE: usize = std::mem::size_of::<RefCountHeader>();

/// Reference count header - prepended to all allocations
///
/// # Memory Layout
/// - Size: 16 bytes (2 × i64)
/// - Alignment: 8 bytes
/// - Offset of `refcount`: 0
/// - Offset of `size`: 8
///
/// # C Equivalent
/// ```c
/// typedef struct RefCountHeader {
///     int64_t refcount;
///     int64_t size;
/// } RefCountHeader;
/// ```
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct RefCountHeader {
    /// Reference count (starts at 1 for new allocations)
    pub refcount: i64,
    /// Size of the user data portion (excludes header)
    pub size: i64,
}

impl RefCountHeader {
    /// Create a new header with refcount=1 and the given size
    #[inline]
    pub fn new(size: i64) -> Self {
        Self { refcount: 1, size }
    }
}

/// FFI-compatible string structure
///
/// # Memory Layout
/// - Size: 24 bytes (3 × i64/pointer)
/// - Alignment: 8 bytes
/// - Offset of `len`: 0
/// - Offset of `cap`: 8
/// - Offset of `data`: 16
///
/// # Memory Model
/// - The TrqString struct itself is reference-counted via RefCountHeader
/// - The `data` buffer is allocated separately via malloc (NOT reference-counted)
/// - String data is UTF-8 encoded and null-terminated
///
/// # C Equivalent
/// ```c
/// typedef struct TrqString {
///     int64_t len;      // Length in bytes (UTF-8)
///     int64_t cap;      // Capacity in bytes
///     char* data;       // UTF-8 data (null-terminated)
/// } TrqString;
/// ```
#[repr(C)]
#[derive(Debug)]
pub struct TrqString {
    /// Length in bytes (not characters for UTF-8)
    pub len: i64,
    /// Capacity in bytes
    pub cap: i64,
    /// Pointer to UTF-8 data (null-terminated)
    pub data: *mut u8,
}

impl TrqString {
    /// Create a new empty TrqString (for internal use)
    #[inline]
    pub fn empty() -> Self {
        Self {
            len: 0,
            cap: 0,
            data: ptr::null_mut(),
        }
    }

    /// Check if the string data pointer is null
    #[inline]
    pub fn is_null(&self) -> bool {
        self.data.is_null()
    }
}

/// FFI-compatible array structure
///
/// # Memory Layout
/// - Size: 32 bytes (4 × i64)
/// - Alignment: 8 bytes
/// - Offset of `len`: 0
/// - Offset of `cap`: 8
/// - Offset of `elem_size`: 16
/// - Offset of `data`: 24
///
/// # Memory Model
/// - The TrqArray struct itself is reference-counted via RefCountHeader
/// - The `data` buffer is allocated separately via malloc (NOT reference-counted)
/// - Elements are stored contiguously with stride = elem_size
///
/// # C Equivalent
/// ```c
/// struct TrqArray {
///     int64_t len;       // Number of elements
///     int64_t cap;       // Capacity
///     int64_t elem_size; // Size of each element in bytes
///     void* data;        // Element data pointer
/// };
/// ```
#[repr(C)]
#[derive(Debug)]
pub struct TrqArray {
    /// Number of elements currently stored
    pub len: i64,
    /// Capacity (number of elements that can be stored)
    pub cap: i64,
    /// Size of each element in bytes
    pub elem_size: i64,
    /// Pointer to element data
    pub data: *mut u8,
}

impl TrqArray {
    /// Initial capacity for new arrays
    pub const INITIAL_CAP: i64 = 8;

    /// Create a new empty TrqArray (for internal use)
    #[inline]
    pub fn empty(elem_size: i64) -> Self {
        Self {
            len: 0,
            cap: 0,
            elem_size,
            data: ptr::null_mut(),
        }
    }

    /// Check if the data pointer is null
    #[inline]
    pub fn is_null(&self) -> bool {
        self.data.is_null()
    }
}

// Compile-time size assertions to ensure ABI compatibility
const _: () = {
    assert!(std::mem::size_of::<RefCountHeader>() == 16);
    assert!(std::mem::align_of::<RefCountHeader>() == 8);
    assert!(std::mem::size_of::<TrqString>() == 24);
    assert!(std::mem::align_of::<TrqString>() == 8);
    assert!(std::mem::size_of::<TrqArray>() == 32);
    assert!(std::mem::align_of::<TrqArray>() == 8);
    assert!(HEADER_SIZE == 16);
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_size() {
        assert_eq!(std::mem::size_of::<RefCountHeader>(), 16);
        assert_eq!(std::mem::align_of::<RefCountHeader>(), 8);
    }

    #[test]
    fn test_string_size() {
        assert_eq!(std::mem::size_of::<TrqString>(), 24);
        assert_eq!(std::mem::align_of::<TrqString>(), 8);
    }

    #[test]
    fn test_array_size() {
        assert_eq!(std::mem::size_of::<TrqArray>(), 32);
        assert_eq!(std::mem::align_of::<TrqArray>(), 8);
    }

    #[test]
    fn test_header_new() {
        let header = RefCountHeader::new(100);
        assert_eq!(header.refcount, 1);
        assert_eq!(header.size, 100);
    }

    #[test]
    fn test_header_size_constant() {
        assert_eq!(HEADER_SIZE, 16);
    }
}
