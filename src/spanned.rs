use chumsky::span::SimpleSpan;
use serde::Deserialize;

#[derive(Debug, PartialEq, Eq, Deserialize, Clone, Hash)]
pub struct Spanned<T> {
    #[serde(skip_serializing)]
    pub span: SimpleSpan<usize>,
    pub val: T,
}

impl<T: PartialOrd> PartialOrd for Spanned<T> {
    fn partial_cmp(&self, other: &Spanned<T>) -> Option<std::cmp::Ordering> {
        self.val.partial_cmp(&other.val)
    }
}

impl<T: Ord> Ord for Spanned<T> {
    fn cmp(&self, other: &Spanned<T>) -> std::cmp::Ordering {
        self.val.cmp(&other.val)
    }
}
