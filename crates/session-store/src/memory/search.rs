//! Keyword search over `MEMORY.md` + project notes/daily + ranked render.

use std::fs;

use regex::RegexBuilder;

use super::store::Mem;
use super::truncate_cjk;

/// One file's worth of ranked search results.
#[derive(Debug, Clone)]
pub struct SearchHit {
    pub path: std::path::PathBuf,
    /// Distinct query terms that matched, in query order. Primary rank key.
    pub matched_terms: Vec<String>,
    /// Total matching lines in the file. Secondary rank key.
    pub total_hits: usize,
    /// Rendered context windows (content hit) or short preview (filename hit).
    pub body: String,
    /// Matched on filename only, not content: ranked below content hits.
    pub filename_only: bool,
    /// Daily-log date for recency ordering; None for other files.
    pub date: Option<String>,
    /// The global MEMORY.md, which always sorts first.
    pub is_memory_md: bool,
}

/// Ranked search output plus per-term match counts for the summary line.
#[derive(Debug, Clone)]
pub struct SearchResults {
    /// (term, total matching lines across all files), in query order.
    pub terms: Vec<(String, usize)>,
    /// Files with matches, most- to least-relevant.
    pub hits: Vec<SearchHit>,
}

impl SearchResults {
    /// Render hits: a one-line summary, then ranked `path [matched: …]`
    /// blocks, greedily capped at `max_bytes` (least-relevant drop first).
    pub fn render(&self, max_bytes: usize) -> String {
        let file_count = self.hits.len();
        let terms_str = self
            .terms
            .iter()
            .map(|(t, c)| format!("{t}({c})"))
            .collect::<Vec<_>>()
            .join(" ");

        let mut blocks = Vec::with_capacity(self.hits.len());
        for h in &self.hits {
            let mt = if h.matched_terms.is_empty() {
                "—".to_string()
            } else {
                h.matched_terms.join(", ")
            };
            blocks.push(format!(
                "{}  [matched: {}]\n{}",
                h.path.display(),
                mt,
                h.body
            ));
        }

        // Reserve headroom for summary + marker; final truncate is backstop.
        let budget = max_bytes.saturating_sub(256);
        const SEP: &str = "\n\n";
        let mut included: Vec<&str> = Vec::new();
        let mut used = 0usize;
        for b in &blocks {
            let add = if included.is_empty() {
                b.len()
            } else {
                SEP.len() + b.len()
            };
            if !included.is_empty() && used + add > budget {
                break;
            }
            used += add;
            included.push(b.as_str());
        }
        let shown = included.len();
        let omitted = file_count - shown;

        let summary = format!(
            "Searched {} term{}: {} across {} file{}. Showing top {} by relevance.",
            self.terms.len(),
            if self.terms.len() == 1 { "" } else { "s" },
            terms_str,
            file_count,
            if file_count == 1 { "" } else { "s" },
            shown,
        );

        let mut out = String::with_capacity(summary.len() + used + 64);
        out.push_str(&summary);
        out.push_str("\n\n");
        out.push_str(&included.join(SEP));
        if omitted > 0 {
            out.push_str(&format!(
                "\n\n…[search truncated, {omitted} more file{} omitted]",
                if omitted == 1 { "" } else { "s" }
            ));
        }
        truncate_cjk(&out, max_bytes, "\n…[memory truncated]")
    }
}

impl Mem {
    /// Case-insensitive multi-term keyword search over global `MEMORY.md` +
    /// the current project's `notes/` and `daily/` (including older logs).
    /// Terms split on whitespace and match literally (escaped, never regex);
    /// a line matches on ANY term. Per file, matches expand to ±3 lines,
    /// merged (max 5 blocks). Rank: distinct terms, then lines, then recency;
    /// filename-only matches get a preview and rank below content hits.
    pub fn search(&self, query: &str) -> SearchResults {
        // Distinct, non-empty terms, preserving query order.
        let mut terms: Vec<String> = Vec::new();
        for t in query.split_whitespace() {
            if !terms.iter().any(|x| x.eq_ignore_ascii_case(t)) {
                terms.push(t.to_string());
            }
        }
        if terms.is_empty() {
            return SearchResults {
                terms: Vec::new(),
                hits: Vec::new(),
            };
        }

        let matchers: Vec<regex::Regex> = terms
            .iter()
            .map(|t| {
                RegexBuilder::new(&regex::escape(t))
                    .case_insensitive(true)
                    .build()
                    .expect("escaped regex always compiles")
            })
            .collect();

        const CONTEXT: usize = 3;
        const MAX_BLOCKS: usize = 5;

        let mut counts = vec![0usize; terms.len()];
        let mut hits: Vec<SearchHit> = Vec::new();

        // Root yields MEMORY.md only (projects/ has no .md); the daily flag
        // marks logs for recency ordering.
        let dirs = [
            (self.root.clone(), false),
            (self.notes_dir(), false),
            (self.daily_dir(), true),
        ];

        for (dir, is_daily) in dirs {
            let rd = match fs::read_dir(&dir) {
                Ok(rd) => rd,
                Err(_) => continue,
            };
            for e in rd.flatten() {
                let path = e.path();
                if path.extension().is_none_or(|x| x != "md") {
                    continue;
                }
                let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
                let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                let is_memory_md = name == "MEMORY.md";
                let date = if is_daily {
                    Some(stem.to_string())
                } else {
                    None
                };

                let content = fs::read_to_string(&path).unwrap_or_default();
                let lines: Vec<&str> = content.lines().collect();

                let mut term_hit = vec![false; terms.len()];
                let mut hit_lines: Vec<usize> = Vec::new();
                for (i, line) in lines.iter().enumerate() {
                    let mut any = false;
                    for (ti, m) in matchers.iter().enumerate() {
                        if m.is_match(line) {
                            any = true;
                            term_hit[ti] = true;
                            counts[ti] += 1;
                        }
                    }
                    if any {
                        hit_lines.push(i);
                    }
                }

                let matched_terms: Vec<String> = terms
                    .iter()
                    .zip(&term_hit)
                    .filter(|(_, hit)| **hit)
                    .map(|(t, _)| t.clone())
                    .collect();

                // Expand to ±CONTEXT, merging adjacent regions, cap blocks.
                let mut windows: Vec<(usize, usize)> = Vec::new();
                for &i in &hit_lines {
                    let lo = i.saturating_sub(CONTEXT);
                    let hi = (i + CONTEXT).min(lines.len().saturating_sub(1));
                    let merged = match windows.last_mut() {
                        Some(w) if lo <= w.1 + 1 => {
                            w.1 = w.1.max(hi);
                            true
                        }
                        _ => false,
                    };
                    if merged {
                        continue;
                    }
                    if windows.len() >= MAX_BLOCKS {
                        break;
                    }
                    windows.push((lo, hi));
                }

                // Filename fallback when nothing in the content matched.
                let name_terms: Vec<String> = terms
                    .iter()
                    .enumerate()
                    .filter(|(ti, _)| matchers[*ti].is_match(name))
                    .map(|(_, t)| t.clone())
                    .collect();

                if !windows.is_empty() {
                    let mut body = String::new();
                    for (lo, hi) in &windows {
                        for l in &lines[*lo..=*hi] {
                            body.push_str(l);
                            body.push('\n');
                        }
                        body.push_str("…\n");
                    }
                    hits.push(SearchHit {
                        path,
                        matched_terms,
                        total_hits: hit_lines.len(),
                        body: body.trim_end().to_string(),
                        filename_only: false,
                        date,
                        is_memory_md,
                    });
                } else if !name_terms.is_empty() {
                    // No content hit: fall back to a filename match + preview.
                    let preview: Vec<&str> = lines
                        .iter()
                        .copied()
                        .filter(|l| !l.trim().is_empty())
                        .take(3)
                        .collect();
                    hits.push(SearchHit {
                        path,
                        matched_terms: name_terms,
                        total_hits: 0,
                        body: format!("(filename match)\n{}", preview.join("\n")),
                        filename_only: true,
                        date,
                        is_memory_md,
                    });
                }
            }
        }

        // Rank: MEMORY.md; distinct terms; content > filename; lines; recency; path.
        hits.sort_by(|a, b| {
            b.is_memory_md
                .cmp(&a.is_memory_md)
                .then_with(|| b.matched_terms.len().cmp(&a.matched_terms.len()))
                .then_with(|| a.filename_only.cmp(&b.filename_only))
                .then_with(|| b.total_hits.cmp(&a.total_hits))
                .then_with(|| b.date.cmp(&a.date))
                .then_with(|| a.path.cmp(&b.path))
        });

        let terms = terms.into_iter().zip(counts).collect();
        SearchResults { terms, hits }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_empty() {
        let r = SearchResults {
            terms: vec![],
            hits: vec![],
        };
        assert!(r.render(500).contains("0 files"));
    }
}
