use std::collections::HashMap;

use meilisearch_sdk::client::Client;
use meilisearch_sdk::documents::DocumentsQuery;
use tokio::sync::mpsc::UnboundedSender;

use crate::app::{rows_from_stats, AppEvent, PAGE_SIZE};

/// Compact one-line JSON for a document row.
fn one_line(v: &serde_json::Value) -> String {
    serde_json::to_string(v).unwrap_or_else(|_| "<unserializable>".into())
}

/// Minimal, tolerant view of `GET /stats`. We only need document counts, so we
/// ignore every other field — this survives server versions that omit or add
/// fields the SDK's strict `IndexStats` would reject (e.g. numberOfEmbeddedDocuments).
// ponytail: hand-rolled /stats to dodge SDK/server field drift; drop it and use
// client.get_stats() once the SDK version matches the server.
#[derive(serde::Deserialize)]
struct StatsResponse {
    indexes: HashMap<String, IndexStat>,
}

#[derive(serde::Deserialize)]
struct IndexStat {
    #[serde(rename = "numberOfDocuments")]
    number_of_documents: usize,
}

/// Fetch stats for all indexes -> index rows with counts.
pub fn load_indexes(url: String, key: Option<String>, tx: UnboundedSender<AppEvent>) {
    tokio::spawn(async move {
        let mut req = reqwest::Client::new().get(format!("{}/stats", url.trim_end_matches('/')));
        if let Some(k) = key {
            req = req.bearer_auth(k);
        }
        let result = async {
            let resp = req.send().await?.error_for_status()?;
            resp.json::<StatsResponse>().await
        }
        .await;
        match result {
            Ok(stats) => {
                let map: HashMap<String, usize> = stats
                    .indexes
                    .into_iter()
                    .map(|(uid, s)| (uid, s.number_of_documents))
                    .collect();
                let _ = tx.send(AppEvent::Indexes(rows_from_stats(map)));
            }
            Err(e) => {
                let _ = tx.send(AppEvent::Error(e.to_string()));
            }
        }
    });
}

/// Create an index, wait for the task to finish, then refresh the index list.
pub fn create_index(
    client: Client,
    url: String,
    key: Option<String>,
    uid: String,
    tx: UnboundedSender<AppEvent>,
) {
    tokio::spawn(async move {
        let _ = tx.send(AppEvent::Status(format!("creating '{uid}'…")));
        match client.create_index(&uid, None).await {
            Ok(task) => match task.wait_for_completion(&client, None, None).await {
                Ok(_) => {
                    let _ = tx.send(AppEvent::Status(format!("created '{uid}'")));
                    load_indexes(url, key, tx);
                }
                Err(e) => {
                    let _ = tx.send(AppEvent::Error(e.to_string()));
                }
            },
            Err(e) => {
                let _ = tx.send(AppEvent::Error(e.to_string()));
            }
        }
    });
}

/// Browse documents for an index (no query) at a given page.
pub fn browse_documents(
    client: Client,
    uid: String,
    page: usize,
    seq: u64,
    tx: UnboundedSender<AppEvent>,
) {
    tokio::spawn(async move {
        let index = client.index(&uid);
        let mut q = DocumentsQuery::new(&index);
        q.with_limit(PAGE_SIZE);
        q.with_offset(page * PAGE_SIZE);
        match index.get_documents_with::<serde_json::Value>(&q).await {
            Ok(res) => {
                let lines = res.results.iter().map(one_line).collect();
                let _ = tx.send(AppEvent::Documents {
                    lines,
                    total: res.total as usize,
                    seq,
                });
            }
            Err(e) => {
                let _ = tx.send(AppEvent::Error(e.to_string()));
            }
        }
    });
}

/// Search documents for an index at a given page.
pub fn search_documents(
    client: Client,
    uid: String,
    query: String,
    page: usize,
    seq: u64,
    tx: UnboundedSender<AppEvent>,
) {
    tokio::spawn(async move {
        let index = client.index(&uid);
        let res = index
            .search()
            .with_query(&query)
            .with_offset(page * PAGE_SIZE)
            .with_limit(PAGE_SIZE)
            .execute::<serde_json::Value>()
            .await;
        match res {
            Ok(r) => {
                let lines = r.hits.iter().map(|h| one_line(&h.result)).collect();
                let total = r.estimated_total_hits.unwrap_or(0);
                let _ = tx.send(AppEvent::Documents { lines, total, seq });
            }
            Err(e) => {
                let _ = tx.send(AppEvent::Error(e.to_string()));
            }
        }
    });
}
