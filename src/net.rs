use std::collections::HashMap;

use meilisearch_sdk::client::Client;
use meilisearch_sdk::documents::DocumentsQuery;
use tokio::sync::mpsc::UnboundedSender;

use crate::app::{rows_from_stats, AppEvent, PAGE_SIZE};

/// Compact one-line JSON for a document row.
fn one_line(v: &serde_json::Value) -> String {
    serde_json::to_string(v).unwrap_or_else(|_| "<unserializable>".into())
}

/// Fetch stats for all indexes -> index rows with counts.
pub fn load_indexes(client: Client, tx: UnboundedSender<AppEvent>) {
    tokio::spawn(async move {
        match client.get_stats().await {
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
pub fn create_index(client: Client, uid: String, tx: UnboundedSender<AppEvent>) {
    tokio::spawn(async move {
        let _ = tx.send(AppEvent::Status(format!("creating '{uid}'…")));
        match client.create_index(&uid, None).await {
            Ok(task) => match task.wait_for_completion(&client, None, None).await {
                Ok(_) => {
                    let _ = tx.send(AppEvent::Status(format!("created '{uid}'")));
                    load_indexes(client, tx);
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
