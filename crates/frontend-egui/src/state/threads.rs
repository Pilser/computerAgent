//! Sidebar thread list: rows, active id, search filter (egui-free).

/// One sidebar row: identity + title + model.
#[derive(Debug, Clone, PartialEq)]
pub struct ThreadRow {
    pub id: String,
    pub title: String,
    pub model: String,
}

impl ThreadRow {
    /// Build a row from its parts.
    pub fn new(id: &str, title: &str, model: &str) -> Self {
        Self {
            id: id.to_string(),
            title: title.to_string(),
            model: model.to_string(),
        }
    }
}

/// Thread rows plus selection and search text.
#[derive(Debug, Default)]
pub struct ThreadList {
    rows: Vec<ThreadRow>,
    pub active_id: Option<String>,
    pub search: String,
}

impl ThreadList {
    /// Empty list with no selection.
    pub fn new() -> Self {
        Self::default()
    }

    /// Replace all rows (engine order: most recent first).
    pub fn set_rows(&mut self, rows: Vec<ThreadRow>) {
        self.rows = rows;
    }

    /// All rows, most recent first.
    #[must_use]
    pub fn rows(&self) -> &[ThreadRow] {
        &self.rows
    }

    /// Rows matching `search` (case-insensitive title substring; empty = all).
    #[must_use]
    pub fn filtered(&self) -> Vec<&ThreadRow> {
        let needle = self.search.trim().to_lowercase();
        if needle.is_empty() {
            return self.rows.iter().collect();
        }
        self.rows
            .iter()
            .filter(|r| r.title.to_lowercase().contains(&needle))
            .collect()
    }

    /// Select a thread (unknown ids still stick; the feed shows empty).
    pub fn select(&mut self, id: &str) {
        self.active_id = Some(id.to_string());
    }

    /// Drop one row; clears the selection when it pointed at it.
    pub fn remove(&mut self, id: &str) {
        self.rows.retain(|r| r.id != id);
        if self.active_id.as_deref() == Some(id) {
            self.active_id = None;
        }
    }
}
