use std::collections::HashMap;

pub const PAGE_SIZE: usize = 50;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Screen {
    IndexList,
    Documents,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mode {
    Normal,
    /// Typing a new index name on the IndexList screen.
    CreateInput,
    /// Typing a search query on the Documents screen.
    SearchInput,
}

pub struct IndexRow {
    pub uid: String,
    pub count: usize,
}

/// Results delivered back from spawned async tasks over the channel.
pub enum AppEvent {
    Indexes(Vec<IndexRow>),
    /// (documents-as-lines, total-hits, query-seq). seq lets us drop stale search results.
    Documents {
        lines: Vec<String>,
        total: usize,
        seq: u64,
    },
    Status(String),
    Error(String),
}

pub struct Model {
    pub screen: Screen,
    pub mode: Mode,
    pub running: bool,

    // Index list
    pub indexes: Vec<IndexRow>,
    pub index_sel: usize,
    pub input: String, // shared by create + search input

    // Documents view
    pub current_index: String,
    pub docs: Vec<String>,
    pub doc_sel: usize,
    pub page: usize,
    pub total_hits: usize,
    pub query: String,
    pub query_seq: u64, // monotonic; the last query we fired

    pub status: String,
}

impl Model {
    pub fn new() -> Self {
        Model {
            screen: Screen::IndexList,
            mode: Mode::Normal,
            running: true,
            indexes: Vec::new(),
            index_sel: 0,
            input: String::new(),
            current_index: String::new(),
            docs: Vec::new(),
            doc_sel: 0,
            page: 0,
            total_hits: 0,
            query: String::new(),
            query_seq: 0,
            status: "loading indexes…".into(),
        }
    }

    pub fn selected_index(&self) -> Option<&IndexRow> {
        self.indexes.get(self.index_sel)
    }
}

/// Apply a channel event to the model.
pub fn apply_event(model: &mut Model, ev: AppEvent) {
    match ev {
        AppEvent::Indexes(rows) => {
            model.indexes = rows;
            if model.index_sel >= model.indexes.len() {
                model.index_sel = model.indexes.len().saturating_sub(1);
            }
            model.status = format!("{} index(es)", model.indexes.len());
        }
        AppEvent::Documents { lines, total, seq } => {
            // ponytail: drop stale search responses; only the newest fired query counts.
            if seq != model.query_seq {
                return;
            }
            model.docs = lines;
            model.total_hits = total;
            if model.doc_sel >= model.docs.len() {
                model.doc_sel = model.docs.len().saturating_sub(1);
            }
            model.status = format!("{} document(s)", total);
        }
        AppEvent::Status(s) => model.status = s,
        AppEvent::Error(e) => model.status = format!("error: {e}"),
    }
}

/// Build the index rows from a stats map (uid -> count), sorted by uid.
pub fn rows_from_stats(map: HashMap<String, usize>) -> Vec<IndexRow> {
    let mut rows: Vec<IndexRow> = map
        .into_iter()
        .map(|(uid, count)| IndexRow { uid, count })
        .collect();
    rows.sort_by(|a, b| a.uid.cmp(&b.uid));
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stale_search_results_are_dropped() {
        let mut m = Model::new();
        m.query_seq = 5; // newest fired query
                         // A response tagged with an older seq must be ignored.
        apply_event(
            &mut m,
            AppEvent::Documents {
                lines: vec!["stale".into()],
                total: 1,
                seq: 4,
            },
        );
        assert!(m.docs.is_empty(), "stale response should be dropped");
        // A response matching the current seq is applied.
        apply_event(
            &mut m,
            AppEvent::Documents {
                lines: vec!["fresh".into()],
                total: 1,
                seq: 5,
            },
        );
        assert_eq!(m.docs, vec!["fresh".to_string()]);
    }

    #[test]
    fn rows_sorted_by_uid() {
        let mut map = HashMap::new();
        map.insert("zebra".into(), 3);
        map.insert("apple".into(), 7);
        let rows = rows_from_stats(map);
        assert_eq!(rows[0].uid, "apple");
        assert_eq!(rows[0].count, 7);
        assert_eq!(rows[1].uid, "zebra");
    }
}
