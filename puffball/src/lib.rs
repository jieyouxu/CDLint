//! This is a wrapper deserialization library over `serde_json`. The intention is to hand-roll our
//! recursive-descent parser for better error reporting. We still delegate string parsing to
//! `serde_json` because handling string escapes are complicated and is better to reuse the existing
//! mature solution if we can.
//!
//! We will deviate from typical parsing/deserialization libraries in that our primary goal is not
//! to be *fast*, but to provide good error reporting for a non-programmer user. To this end, we
//! should try to preserve spans and contextual information where possible. We will take
//! inspiration from compiler lexing/parsing diagnostics and try to learn from them, and treat JSON
//! more like a "programming language".

mod span;

pub use indexmap::IndexMap;
pub use span::Span;

/// A JSON node. Note that we deviate from usual JSON (de-)serialization libraries in that
/// we store span information. We're more like an AST node.
#[derive(Debug)]
pub struct Json {
    pub span: Span,
    pub kind: JsonKind,
}

#[derive(Debug)]
pub enum JsonKind {
    String {
        span: Span,
        value: String,
    },
    Number {
        span: Span,
        value: f64,
    },
    Bool {
        span: Span,
        value: bool,
    },
    Null {
        span: Span,
    },
    Object {
        span: Span,
        /// This is not a hashmap or anything that relies on key equality. It is possible to have
        /// duplicate entries in terms of the key name, which would be semantically malformed JSON
        /// but we retain this formation so downstream users can report errors with this
        /// information.
        value: Vec<(SpannedString, Json)>,
    },
}

#[derive(Debug)]
pub struct SpannedString {
    pub span: Span,
    pub value: String,
}
