//! Compiler context for shared state across compilation phases.
//!
//! The `CompilerContext` holds resources that are shared throughout the entire
//! compilation pipeline, such as the string interner for identifier deduplication.

use super::interner::StringInterner;

/// Shared context for the compilation pipeline.
///
/// This struct holds resources that need to be shared across different phases
/// of compilation (lexing, parsing, semantic analysis, code generation).
///
/// # Example
///
/// ```
/// use tarqeem::utils::CompilerContext;
///
/// let mut ctx = CompilerContext::new();
///
/// // Use the interner to deduplicate strings
/// let sym1 = ctx.interner.intern("متغير");
/// let sym2 = ctx.interner.intern("متغير");
/// assert_eq!(sym1, sym2);
/// ```
#[derive(Debug)]
pub struct CompilerContext {
    /// String interner for identifier and keyword deduplication.
    ///
    /// All identifiers encountered during lexing are interned here,
    /// reducing memory usage and enabling O(1) string comparisons.
    pub interner: StringInterner,
}

impl CompilerContext {
    /// Creates a new compiler context with default capacity.
    pub fn new() -> Self {
        Self {
            interner: StringInterner::with_capacity(10000),
        }
    }

    /// Creates a new compiler context with specified interner capacity.
    ///
    /// Use this when you have an estimate of the number of unique identifiers
    /// in the source code to avoid reallocations.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            interner: StringInterner::with_capacity(capacity),
        }
    }

    /// Returns a reference to the string interner.
    #[inline]
    pub fn interner(&self) -> &StringInterner {
        &self.interner
    }

    /// Returns a mutable reference to the string interner.
    #[inline]
    pub fn interner_mut(&mut self) -> &mut StringInterner {
        &mut self.interner
    }
}

impl Default for CompilerContext {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_creation() {
        let ctx = CompilerContext::new();
        assert!(ctx.interner.is_empty());
    }

    #[test]
    fn test_context_with_capacity() {
        let ctx = CompilerContext::with_capacity(1000);
        assert!(ctx.interner.is_empty());
    }

    #[test]
    fn test_context_interning() {
        let mut ctx = CompilerContext::new();

        let sym1 = ctx.interner_mut().intern("test");
        let sym2 = ctx.interner_mut().intern("test");

        assert_eq!(sym1, sym2);
        assert_eq!(ctx.interner().len(), 1);
    }

    #[test]
    fn test_context_arabic_interning() {
        let mut ctx = CompilerContext::new();

        let sym1 = ctx.interner_mut().intern("متغير");
        let sym2 = ctx.interner_mut().intern("دالة");
        let sym3 = ctx.interner_mut().intern("متغير");

        assert_eq!(sym1, sym3);
        assert_ne!(sym1, sym2);
        assert_eq!(ctx.interner().len(), 2);
    }
}
