//! Source-line ⇄ block mapping for bidirectional scroll sync and typewriter
//! alignment (frontend.md §5.10). The preview is a virtualized list of
//! top-level blocks, so "scroll position" is `(block index, offset)`.
//!
//! **[COMPAT]** FrankMD annotates every top-level block and every list item
//! (nested ones too) with `data-source-line` (`markdown_line_mapper.js`); the
//! [`Anchor`]s here are those annotations. Golden: `js_markdown_line_mapper.json`.

use crate::RenderDoc;
use crate::ir::{Block, ListItem};

/// What an anchor points at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnchorTarget {
    /// Top-level block `doc.blocks[ix]`.
    Block(usize),
    /// A list item: top-level list block index, then the item index path
    /// through nested lists (`[2, 0]` = first child item of the third item).
    ListItem {
        /// Index of the top-level list block.
        block: usize,
        /// Item indices, outermost first.
        path: Vec<usize>,
    },
}

/// One `data-source-line` annotation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Anchor {
    /// 1-based source line.
    pub line: u32,
    /// Annotated node.
    pub target: AnchorTarget,
}

/// Index from source lines to top-level block indices.
#[derive(Debug, Clone, Default)]
pub struct LineMap {
    starts: Vec<u32>,
    anchors: Vec<Anchor>,
}

impl LineMap {
    /// Build from a rendered document. Anchors are in document order: a block,
    /// then (for lists) its items depth-first, an item before its nested items.
    pub fn new(doc: &RenderDoc) -> Self {
        let mut starts = Vec::with_capacity(doc.blocks.len());
        let mut anchors = Vec::new();
        for (ix, block) in doc.blocks.iter().enumerate() {
            let line = block.span().start_line;
            starts.push(line);
            anchors.push(Anchor { line, target: AnchorTarget::Block(ix) });
            if let Block::List { items, .. } = block {
                push_items(ix, items, &mut Vec::new(), &mut anchors);
            }
        }
        Self { starts, anchors }
    }

    /// Build from bare anchor lines (document order), for geometry-only callers.
    pub fn from_lines(lines: &[u32]) -> Self {
        let anchors = lines
            .iter()
            .enumerate()
            .map(|(ix, &line)| Anchor { line, target: AnchorTarget::Block(ix) })
            .collect();
        Self { starts: lines.to_vec(), anchors }
    }

    /// All anchors in document order.
    pub fn anchors(&self) -> &[Anchor] {
        &self.anchors
    }

    /// Block that contains (or follows) 1-based `line`.
    pub fn block_for_line(&self, line: u32) -> Option<usize> {
        if self.starts.is_empty() {
            return None;
        }
        // Number of blocks starting at or before `line`.
        let at_or_before = self.starts.partition_point(|&start| start <= line);
        Some(at_or_before.saturating_sub(1))
    }

    /// First source line of block `ix`.
    pub fn line_for_block(&self, ix: usize) -> Option<u32> {
        self.starts.get(ix).copied()
    }

    /// **[COMPAT]** `findElementByLine`: walk anchors in order keeping the one
    /// with the smallest `|line - target|` (earlier wins ties) and stop at the
    /// first anchor with `line >= target`. `None` without anchors.
    pub fn nearest_anchor(&self, target: i64) -> Option<&Anchor> {
        let mut closest = None;
        let mut closest_distance = u64::MAX;
        for anchor in &self.anchors {
            let line = i64::from(anchor.line);
            let distance = line.abs_diff(target);
            if distance < closest_distance {
                closest_distance = distance;
                closest = Some(anchor);
            }
            if line >= target {
                break;
            }
        }
        closest
    }
}

/// Depth-first item anchors: an item, then the items of lists nested in it.
fn push_items(block: usize, items: &[ListItem], path: &mut Vec<usize>, anchors: &mut Vec<Anchor>) {
    for (ix, item) in items.iter().enumerate() {
        path.push(ix);
        anchors.push(Anchor {
            line: item.span.start_line,
            target: AnchorTarget::ListItem { block, path: path.clone() },
        });
        for child in &item.children {
            if let Block::List { items: nested, .. } = child {
                push_items(block, nested, path, anchors);
            }
        }
        path.pop();
    }
}

/// **[COMPAT]** scroll geometry from `app/javascript/lib/scroll_utils.js`
/// (pure `f64` math, JS semantics; golden `js_scroll_utils.json`).
pub mod scroll {
    /// A laid-out anchor: its line and its top in container-content coordinates.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct ScrollEntry {
        /// Source line.
        pub line: u32,
        /// Top, container-content coordinates.
        pub top: f64,
    }

    /// `scrollTop / (scrollHeight - clientHeight)` clamped to `[0, 1]`; 0 when content fits.
    pub fn scroll_ratio(scroll_top: f64, scroll_height: f64, client_height: f64) -> f64 {
        let max_scroll = scroll_height - client_height;
        if max_scroll <= 0.0 {
            return 0.0;
        }
        js_max(0.0, js_min(1.0, scroll_top / max_scroll))
    }

    /// `max(0, (line - 1) * line_height - viewport_height * offset_ratio)`.
    /// FrankMD's default `offset_ratio` is [`DEFAULT_LINE_OFFSET_RATIO`].
    pub fn scroll_for_line(line: f64, line_height: f64, viewport_height: f64, offset_ratio: f64) -> f64 {
        js_max(0.0, (line - 1.0) * line_height - viewport_height * offset_ratio)
    }

    /// Default `offsetRatio` of `calculateScrollForLine` (35% from the top).
    pub const DEFAULT_LINE_OFFSET_RATIO: f64 = 0.35;

    /// `round((scrollTop + clientHeight/2) / (scrollHeight/totalLines))`
    /// clamped to `[1, totalLines]`; 1 when `total_lines <= 0`. JS `Math.round`
    /// (half rounds up).
    pub fn line_from_scroll(scroll_top: f64, client_height: f64, scroll_height: f64, total_lines: f64) -> f64 {
        if total_lines <= 0.0 {
            return 1.0;
        }
        let line_height = scroll_height / total_lines;
        let center_y = scroll_top + client_height / 2.0;
        let center_line = js_round(center_y / line_height);
        js_max(1.0, js_min(total_lines, center_line))
    }

    /// `max(0, ratio * (scrollHeight - clientHeight))`; 0 when content fits.
    pub fn synced_scroll_position(ratio: f64, scroll_height: f64, client_height: f64) -> f64 {
        let max_scroll = scroll_height - client_height;
        if max_scroll <= 0.0 {
            return 0.0;
        }
        js_max(0.0, ratio * max_scroll)
    }

    /// `(line-1)/max(1,total-1) * (scrollHeight-clientHeight)`, ≥ 0; 0 for
    /// `total <= 0` or when content fits.
    pub fn scroll_to_center_line(line: f64, total_lines: f64, scroll_height: f64, client_height: f64) -> f64 {
        if total_lines <= 0.0 {
            return 0.0;
        }
        let line_ratio = (line - 1.0) / js_max(1.0, total_lines - 1.0);
        let max_scroll = scroll_height - client_height;
        if max_scroll <= 0.0 {
            return 0.0;
        }
        js_max(0.0, line_ratio * max_scroll)
    }

    /// `el_top - container_top + container_scroll_top` (page-offset invariant).
    /// Not clamped (JS returns e.g. `-50 - 50 + 100 = 0` naturally).
    pub fn scroll_top_for_element(el_top: f64, container_top: f64, container_scroll_top: f64) -> f64 {
        el_top - container_top + container_scroll_top
    }

    /// Line of the last-best entry with `top <= scroll_top` (strictly greater
    /// top replaces, so ties keep the earlier entry); the first entry's line
    /// when scrolled above all; `None` when empty.
    pub fn line_at_scroll(entries: &[ScrollEntry], scroll_top: f64) -> Option<u32> {
        let first = entries.first()?;
        let mut closest: Option<&ScrollEntry> = None;
        let mut closest_top = f64::NEG_INFINITY;
        for entry in entries {
            if entry.top <= scroll_top && entry.top > closest_top {
                closest_top = entry.top;
                closest = Some(entry);
            }
        }
        Some(closest.unwrap_or(first).line)
    }

    /// JS `Math.max(a, b)`: NaN if either is NaN.
    fn js_max(a: f64, b: f64) -> f64 {
        if a.is_nan() || b.is_nan() { f64::NAN } else { a.max(b) }
    }

    /// JS `Math.min(a, b)`: NaN if either is NaN.
    fn js_min(a: f64, b: f64) -> f64 {
        if a.is_nan() || b.is_nan() { f64::NAN } else { a.min(b) }
    }

    /// JS `Math.round`: halves round towards +infinity.
    fn js_round(x: f64) -> f64 {
        if !x.is_finite() {
            return x;
        }
        let floor = x.floor();
        if x - floor >= 0.5 { floor + 1.0 } else { floor }
    }
}
