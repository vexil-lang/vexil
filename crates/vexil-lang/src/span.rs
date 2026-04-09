//! # Stability: Tier 2
//!
//! Source span types for mapping compiler outputs back to source locations.

/// A byte-offset range in source text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    /// Byte offset from the start of the source string.
    pub offset: u32,
    /// Length in bytes.
    pub len: u32,
}

impl Span {
    /// Create a new span from an offset and length.
    pub fn new(offset: usize, len: usize) -> Self {
        Self {
            offset: offset as u32,
            len: len as u32,
        }
    }

    /// Create a zero-length (empty) span at the given offset.
    pub fn empty(offset: usize) -> Self {
        Self::new(offset, 0)
    }

    /// Convert to a `Range<usize>` for slicing source text.
    pub fn range(&self) -> std::ops::Range<usize> {
        let start = self.offset as usize;
        start..start + self.len as usize
    }
}

/// A value with an associated source span, used throughout the AST and IR
/// for error reporting and source mapping.
#[derive(Debug, Clone, PartialEq)]
pub struct Spanned<T> {
    /// The wrapped value.
    pub node: T,
    /// Source location of this value.
    pub span: Span,
}

impl<T> Spanned<T> {
    /// Create a new spanned value.
    pub fn new(node: T, span: Span) -> Self {
        Self { node, span }
    }
}
