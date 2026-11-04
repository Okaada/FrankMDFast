//! The span an operator acts on.

use std::ops::Range;

use crate::text::{line_end, line_index, line_start_of};

/// What an operator acts on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Region {
    /// A byte range (charwise).
    Chars(Range<usize>),
    /// Whole lines `first..=last` (zero-based).
    Lines { first: usize, last: usize },
    /// One byte range per line of a blockwise selection, top to bottom.
    Block(Vec<Range<usize>>),
}

impl Region {
    /// First and last line touched.
    pub(crate) fn lines(&self, t: &str) -> (usize, usize) {
        match self {
            Self::Chars(r) => (line_index(t, r.start), line_index(t, r.end.max(r.start))),
            Self::Lines { first, last } => (*first, *last),
            Self::Block(pieces) => {
                let first = pieces.first().map_or(0, |r| line_index(t, r.start));
                let last = pieces.last().map_or(first, |r| line_index(t, r.start));
                (first, last)
            }
        }
    }

    /// Byte span covering everything (lines: without the final newline).
    pub(crate) fn span(&self, t: &str) -> Range<usize> {
        match self {
            Self::Chars(r) => r.clone(),
            Self::Lines { first, last } => {
                line_start_of(t, *first)..line_end(t, line_start_of(t, *last))
            }
            Self::Block(pieces) => {
                let start = pieces.first().map_or(0, |r| r.start);
                let end = pieces.last().map_or(start, |r| r.end);
                start..end
            }
        }
    }

    /// The pieces of text the region covers (one per line for lines and blocks).
    pub(crate) fn pieces(&self, t: &str) -> Vec<Range<usize>> {
        match self {
            Self::Chars(r) => vec![r.clone()],
            Self::Lines { first, last } => (*first..=*last)
                .map(|l| {
                    let ls = line_start_of(t, l);
                    ls..line_end(t, ls)
                })
                .collect(),
            Self::Block(pieces) => pieces.clone(),
        }
    }
}

/// Rebuild `t[span]` with every piece passed through `f` (pieces are sorted and inside `span`).
pub(crate) fn map_pieces(
    t: &str,
    span: &Range<usize>,
    pieces: &[Range<usize>],
    mut f: impl FnMut(&str) -> String,
) -> String {
    let mut out = String::with_capacity(span.len());
    let mut at = span.start;
    for p in pieces {
        out.push_str(&t[at..p.start]);
        out.push_str(&f(&t[p.clone()]));
        at = p.end;
    }
    out.push_str(&t[at..span.end]);
    out
}
