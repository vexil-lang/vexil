/// A byte-offset range in source text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub offset: u32,
    pub len: u32,
}

impl Span {
    pub fn new(offset: usize, len: usize) -> Self {
        Self {
            offset: offset as u32,
            len: len as u32,
        }
    }

    pub fn empty(offset: usize) -> Self {
        Self::new(offset, 0)
    }

    pub fn range(&self) -> std::ops::Range<usize> {
        let start = self.offset as usize;
        start..start + self.len as usize
    }
}

/// A value with an associated source span.
#[derive(Debug, Clone, PartialEq)]
pub struct Spanned<T> {
    pub node: T,
    pub span: Span,
}

impl<T> Spanned<T> {
    pub fn new(node: T, span: Span) -> Self {
        Self { node, span }
    }
}
