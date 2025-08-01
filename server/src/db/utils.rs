use deadpool_sqlite::{Hook, HookError};
use deadpool_sync::SyncWrapper;
use rusqlite::Connection;

use crate::config::{self, RunConfig};

use super::{query::CREATE_TABLE, sqlite3_impl::Pool};

pub async fn open_db(rc: &RunConfig) -> Pool {
    let moose_path = rc.get_moose_path();
    config::create_parent_dirs(&moose_path).unwrap();
    let cfg = deadpool_sqlite::Config::new(&moose_path);
    cfg.builder(deadpool_sqlite::Runtime::Tokio1)
        .expect("Expected to build Sqlite3 pool builder.")
        .post_create(Hook::async_fn(|con: &mut SyncWrapper<Connection>, _| {
            Box::pin(async move {
                con.interact(|con| con.execute_batch(CREATE_TABLE).map_err(HookError::Backend))
                    .await
                    .expect("conn.interact should not fail.")
            })
        }))
        .build()
        .unwrap() // only fails when no runtime given.
}

/// Escapes a search query similar to how legacy moose does. FTS5 syntax is a bit much.
pub fn escape_query(q: &str) -> String {
    let words = q
        .split_whitespace()
        .filter(|substr| !substr.is_empty())
        .collect::<Vec<_>>();
    let len = words.len();
    words
        .into_iter()
        .enumerate()
        .map(|(i, substr)| {
            if substr == "AND" || substr == "OR" {
                if i == 0 || i == (len - 1) {
                    format!("\"{substr}\"")
                } else {
                    substr.to_owned()
                }
            } else {
                format!("\"{}\"", substr.replace("\"", "\"\""))
            }
        })
        .collect::<Vec<String>>()
        .join(" ")
}
