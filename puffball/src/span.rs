/// A [`Span`] represents a contiguous region of the input. It is used to correspond a parsed
/// JSON syntax tree node to its source. An invariant to be maintained is that `lo <= hi`. It
/// is typically the case that your code has logic bugs if this invariant is violated.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Span {
    pub lo: usize,
    pub hi: usize,
}

impl Span {
    /// Construct a new span. Will panic if `lo > hi`. Prefer this constructor to construct a new
    /// [`Span`] over using direct struct initialization.
    pub const fn new(lo: usize, hi: usize) -> Self {
        assert!(lo <= hi, "`lo` must not be larger than `hi`");
        Span { lo, hi }
    }
}
