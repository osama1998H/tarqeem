//! String Interning for Tarqeem
//!
//! This module provides string interning to reduce memory allocations
//! and speed up string comparisons for identifiers and keywords.
//!
//! # Usage
//!
//! ```
//! use tarqeem::utils::StringInterner;
//!
//! let mut interner = StringInterner::new();
//! let sym1 = interner.intern("متغير");
//! let sym2 = interner.intern("متغير");
//!
//! // Same symbol for same string
//! assert_eq!(sym1, sym2);
//!
//! // Fast comparison (just compare indices)
//! assert!(sym1 == sym2);
//!
//! // Retrieve the original string
//! assert_eq!(interner.resolve(sym1), Some("متغير"));
//! ```

use std::collections::HashMap;

/// A symbol representing an interned string.
///
/// Symbols are lightweight handles that can be compared cheaply.
/// Use `StringInterner::resolve` to get the original string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Symbol(u32);

impl Symbol {
    /// Returns the raw index of this symbol.
    #[inline]
    pub fn as_u32(self) -> u32 {
        self.0
    }

    /// Creates a symbol from a raw index.
    ///
    /// # Safety
    /// The index must be valid for the interner that will be used to resolve it.
    #[inline]
    pub fn from_u32(index: u32) -> Self {
        Symbol(index)
    }
}

/// A string interner that stores strings and returns lightweight symbols.
///
/// The interner ensures that each unique string is stored only once,
/// reducing memory usage and making string comparisons O(1).
#[derive(Debug, Default)]
pub struct StringInterner {
    /// Map from string to symbol for fast lookup
    map: HashMap<String, Symbol>,
    /// Storage for interned strings (indexed by symbol)
    strings: Vec<String>,
}

impl StringInterner {
    /// Creates a new empty string interner.
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
            strings: Vec::new(),
        }
    }

    /// Creates a new string interner with the specified capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            map: HashMap::with_capacity(capacity),
            strings: Vec::with_capacity(capacity),
        }
    }

    /// Interns a string, returning its symbol.
    ///
    /// If the string has already been interned, returns the existing symbol.
    /// Otherwise, stores the string and returns a new symbol.
    pub fn intern(&mut self, string: &str) -> Symbol {
        if let Some(&symbol) = self.map.get(string) {
            return symbol;
        }

        let symbol = Symbol(self.strings.len() as u32);
        let owned = string.to_string();
        self.strings.push(owned.clone());
        self.map.insert(owned, symbol);
        symbol
    }

    /// Interns an owned string, returning its symbol.
    ///
    /// This is more efficient than `intern` when you already have an owned String,
    /// as it avoids an extra allocation.
    pub fn intern_owned(&mut self, string: String) -> Symbol {
        if let Some(&symbol) = self.map.get(&string) {
            return symbol;
        }

        let symbol = Symbol(self.strings.len() as u32);
        self.strings.push(string.clone());
        self.map.insert(string, symbol);
        symbol
    }

    /// Resolves a symbol to its original string.
    ///
    /// Returns `None` if the symbol is invalid (not from this interner).
    #[inline]
    pub fn resolve(&self, symbol: Symbol) -> Option<&str> {
        self.strings.get(symbol.0 as usize).map(|s| s.as_str())
    }

    /// Returns the number of interned strings.
    #[inline]
    pub fn len(&self) -> usize {
        self.strings.len()
    }

    /// Returns true if no strings have been interned.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.strings.is_empty()
    }

    /// Checks if a string has been interned.
    #[inline]
    pub fn contains(&self, string: &str) -> bool {
        self.map.contains_key(string)
    }

    /// Gets the symbol for a string if it has been interned.
    #[inline]
    pub fn get(&self, string: &str) -> Option<Symbol> {
        self.map.get(string).copied()
    }

    /// Clears all interned strings.
    pub fn clear(&mut self) {
        self.map.clear();
        self.strings.clear();
    }
}

/// A thread-safe version of StringInterner for use in concurrent contexts.
#[cfg(feature = "concurrent")]
pub mod concurrent {
    use dashmap::DashMap;
    use std::sync::RwLock;

    use super::Symbol;

    /// A thread-safe string interner.
    #[derive(Debug)]
    pub struct ConcurrentInterner {
        map: DashMap<String, Symbol>,
        strings: RwLock<Vec<String>>,
    }

    impl ConcurrentInterner {
        pub fn new() -> Self {
            Self {
                map: DashMap::new(),
                strings: RwLock::new(Vec::new()),
            }
        }

        pub fn intern(&self, string: &str) -> Symbol {
            if let Some(symbol) = self.map.get(string) {
                return *symbol;
            }

            let mut strings = self.strings.write().unwrap();
            // Double-check after acquiring write lock
            if let Some(symbol) = self.map.get(string) {
                return *symbol;
            }

            let symbol = Symbol(strings.len() as u32);
            let owned = string.to_string();
            strings.push(owned.clone());
            self.map.insert(owned, symbol);
            symbol
        }

        pub fn resolve(&self, symbol: Symbol) -> Option<String> {
            let strings = self.strings.read().unwrap();
            strings.get(symbol.0 as usize).cloned()
        }

        pub fn len(&self) -> usize {
            self.strings.read().unwrap().len()
        }

        pub fn is_empty(&self) -> bool {
            self.len() == 0
        }
    }

    impl Default for ConcurrentInterner {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_interning() {
        let mut interner = StringInterner::new();

        let sym1 = interner.intern("hello");
        let sym2 = interner.intern("world");
        let sym3 = interner.intern("hello");

        assert_eq!(sym1, sym3);
        assert_ne!(sym1, sym2);
    }

    #[test]
    fn test_resolve() {
        let mut interner = StringInterner::new();

        let sym = interner.intern("test");
        assert_eq!(interner.resolve(sym), Some("test"));
    }

    #[test]
    fn test_arabic_strings() {
        let mut interner = StringInterner::new();

        let sym1 = interner.intern("متغير");
        let sym2 = interner.intern("دالة");
        let sym3 = interner.intern("متغير");

        assert_eq!(sym1, sym3);
        assert_ne!(sym1, sym2);

        assert_eq!(interner.resolve(sym1), Some("متغير"));
        assert_eq!(interner.resolve(sym2), Some("دالة"));
    }

    #[test]
    fn test_len_and_empty() {
        let mut interner = StringInterner::new();

        assert!(interner.is_empty());
        assert_eq!(interner.len(), 0);

        interner.intern("a");
        interner.intern("b");
        interner.intern("a"); // Duplicate

        assert!(!interner.is_empty());
        assert_eq!(interner.len(), 2);
    }

    #[test]
    fn test_contains_and_get() {
        let mut interner = StringInterner::new();

        assert!(!interner.contains("hello"));
        assert_eq!(interner.get("hello"), None);

        let sym = interner.intern("hello");

        assert!(interner.contains("hello"));
        assert_eq!(interner.get("hello"), Some(sym));
    }

    #[test]
    fn test_clear() {
        let mut interner = StringInterner::new();

        interner.intern("a");
        interner.intern("b");

        assert_eq!(interner.len(), 2);

        interner.clear();

        assert!(interner.is_empty());
        assert!(!interner.contains("a"));
    }

    #[test]
    fn test_intern_owned() {
        let mut interner = StringInterner::new();

        let sym1 = interner.intern_owned("hello".to_string());
        let sym2 = interner.intern("hello");

        assert_eq!(sym1, sym2);
    }

    #[test]
    fn test_symbol_roundtrip() {
        let sym = Symbol::from_u32(42);
        assert_eq!(sym.as_u32(), 42);
    }

    #[test]
    fn test_many_strings() {
        let mut interner = StringInterner::with_capacity(1000);

        let symbols: Vec<_> = (0..1000)
            .map(|i| interner.intern(&format!("str_{}", i)))
            .collect();

        // Verify all symbols are unique
        let unique: std::collections::HashSet<_> = symbols.iter().collect();
        assert_eq!(unique.len(), 1000);

        // Verify resolution
        for (i, sym) in symbols.iter().enumerate() {
            assert_eq!(interner.resolve(*sym), Some(format!("str_{}", i).as_str()));
        }
    }
}
