use std::{io::BufReader, path::PathBuf};

use deadpool_sqlite::{Hook, HookError};
use futures::FutureExt;
use rusqlite::Connection;

use crate::{
    config::{self, RunConfig},
    model::moose::{Moose, MooseAny},
};

use super::{
    MooseToSqlParams, Pool, QueryError,
    query::{CREATE_TABLE, INSERT_MOOSE_WITH_COMPUTED_POS, UPDATE_MOOSE},
};

pub async fn open_db(rc: &RunConfig) -> Pool {
    let moose_path = rc.get_moose_path();
    config::create_parent_dirs(&moose_path).unwrap();
    let cfg = deadpool_sqlite::Config::new(&moose_path);
    let poolbuild = cfg
        .builder(deadpool_sqlite::Runtime::Tokio1)
        .expect("Expected to build Sqlite3 pool builder.");
    let pool = poolbuild
        .post_create(Hook::async_fn(
            |con: &mut deadpool_sync::SyncWrapper<Connection>, _| {
                Box::into_pin(Box::new(
                    con.interact(|con| con.execute_batch(CREATE_TABLE))
                        .map(|res| match res {
                            Ok(Ok(_)) => Ok(()),
                            Ok(Err(e)) => Err(HookError::Backend(e)),
                            // InteractErrors are only panics or closure aborts.
                            Err(e) => panic!("{e}"),
                        }),
                ))
            },
        ))
        .build()
        .expect("expected to build a Sqlite3 pool.");

    // {
    //     let con = pool.get().await.unwrap();
    //     con.interact(|con| {
    //         con.execute_batch(CREATE_TABLE).unwrap();
    //     })
    //     .await
    //     .unwrap()
    // }
    pool
}

#[derive(Clone, Copy)]
pub enum BulkModeDupe {
    Fail,
    Ignore,
    Update,
}

pub async fn moose_bulk_import(moose_in: Option<PathBuf>, dup_behavior: BulkModeDupe, db: Pool) {
    let mut moose_in = match moose_in {
        Some(path) => {
            let file = BufReader::new(std::fs::File::open(path).unwrap());
            serde_json::from_reader::<_, Vec<MooseAny>>(file).unwrap()
        }
        None => serde_json::from_reader::<_, Vec<MooseAny>>(std::io::stdin().lock()).unwrap(),
    }
    .drain(..)
    .map(|m| m.into())
    .collect::<Vec<Moose>>();

    moose_in.sort_unstable_by(|lhs, rhs| lhs.created.cmp(&rhs.created));
    let conn = db.get().await.unwrap();
    conn.interact(move |conn| {
        let tx = conn
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .unwrap();
        moose_in.iter().try_for_each(|moose| {
            let pm: MooseToSqlParams = moose.into();
            if let Err(e) = tx
                .prepare_cached(INSERT_MOOSE_WITH_COMPUTED_POS)
                .unwrap()
                .execute(pm)
            {
                match &e {
                    rusqlite::Error::SqliteFailure(err, _reason) => match err.code {
                        rusqlite::ErrorCode::ConstraintViolation => match dup_behavior {
                            BulkModeDupe::Fail => Err(e),
                            BulkModeDupe::Ignore => Ok(()),
                            BulkModeDupe::Update => {
                                let _ = tx.prepare_cached(UPDATE_MOOSE).unwrap().execute(pm)?;
                                Ok(())
                            }
                        },
                        _ => Err(e),
                    },
                    _ => Err(e),
                }
            } else {
                Ok(())
            }
        })?;
        tx.commit()
    })
    .await
    .unwrap()
    .unwrap();
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

pub fn handle_opt_q(res: Result<Moose, rusqlite::Error>) -> Result<Option<Moose>, QueryError> {
    match res {
        Ok(m) => Ok(Some(m)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}
