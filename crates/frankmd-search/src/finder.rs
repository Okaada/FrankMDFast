//! Ctrl+P fuzzy file finder.
//!
//! **[COMPAT]** `file_finder_controller.js`: the query is trimmed and
//! lowercased, every path is scored with `fuzzyScore(path.toLowerCase(),
//! query)` ([`frankmd_core::text::utils::fuzzy_score`]), zero scores are
//! dropped and the rest sorted by score (stable, so the incoming order breaks
//! ties). An empty query lists the first `limit` files. Recent paths come
//! first among equal scores (FrankMD's file list is recency-ordered).

use frankmd_core::text::utils::fuzzy_score;

/// A ranked candidate.
#[derive(Debug, Clone, PartialEq)]
pub struct Ranked {
    /// Vault-relative path.
    pub path: String,
    /// Score (higher is better).
    pub score: u32,
    /// Matched char indices for highlighting.
    pub indices: Vec<u32>,
}

/// Rank `candidates` for `query`, boosting `recent` paths.
pub fn rank(query: &str, candidates: &[String], recent: &[String], limit: usize) -> Vec<Ranked> {
    let query = query.trim().to_lowercase();
    let mut ordered: Vec<&String> = recent.iter().filter(|r| candidates.contains(r)).collect();
    ordered.extend(candidates.iter().filter(|c| !recent.contains(c)));

    if query.is_empty() {
        return ordered
            .into_iter()
            .take(limit)
            .map(|path| Ranked {
                path: path.clone(),
                score: 0,
                indices: Vec::new(),
            })
            .collect();
    }

    let mut scored: Vec<Ranked> = ordered
        .into_iter()
        .filter_map(|path| {
            let lower = path.to_lowercase();
            let score = fuzzy_score(&lower, &query);
            (score > 0).then(|| Ranked {
                path: path.clone(),
                score,
                indices: matched_indices(&lower, &query),
            })
        })
        .collect();
    scored.sort_by_key(|r| std::cmp::Reverse(r.score));
    scored.truncate(limit);
    scored
}

/// Char indices of the greedy left-to-right subsequence match `fuzzyScore` uses.
fn matched_indices(haystack: &str, query: &str) -> Vec<u32> {
    let mut indices = Vec::new();
    let mut hay = haystack.chars().enumerate();
    for q in query.chars() {
        match hay.by_ref().find(|(_, c)| *c == q) {
            Some((ix, _)) => indices.push(u32::try_from(ix).unwrap_or(u32::MAX)),
            None => break,
        }
    }
    indices
}
