use std::{
    fs::{self, File},
    io::{BufReader, BufWriter, IntoInnerError, Write},
    path::PathBuf,
};

use crate::{
    db::query::DUMP_MOOSE,
    model::{
        PAGE_SEARCH_LIM, PAGE_SIZE,
        moose::{Moose, MooseAny},
        pages::{MooseSearch, MooseSearchPage},
    },
};

use super::{
    BulkModeDupe, MooseDB, MooseToSqlParams,
    query::{
        GET_MOOSE, GET_MOOSE_IDX, GET_MOOSE_PAGE, INSERT_MOOSE_WITH_COMPUTED_POS, LAST_MOOSE,
        LEN_MOOSE, SEARCH_MOOSE_PAGE, UPDATE_MOOSE,
    },
    utils::escape_query,
};

use rand::Rng;
use rusqlite::{Connection, OptionalExtension, Params};

pub type Pool = deadpool_sqlite::Pool;
pub type PoolConnection = deadpool_sqlite::Object;

#[derive(thiserror::Error, Debug)]
pub enum Sqlite3Error {
    #[error("Pool Connection Error: {0}")]
    ConnectionPool(#[from] deadpool_sqlite::PoolError),
    #[error("Sqlite3 Error: {0}")]
    Sqlite3(#[from] rusqlite::Error),
    #[error(
        "Could not open tmp file, sync() it to disk or rename tmp moose to moose_dump path: {0}"
    )]
    StdIO(#[from] std::io::Error),
    #[error("Deserialization Error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("Failed to recover file handle: {0}")]
    IntoInner(#[from] IntoInnerError<BufWriter<File>>),
    #[error("Moose dump path is either \"/\" or an empty string, \"\".")]
    StrangeMooseDumpPath(),
}

fn query_moose<P: Params>(
    conn: &Connection,
    sql: &'static str,
    params: P,
) -> Result<Option<Moose>, Sqlite3Error> {
    conn.prepare_cached(sql)?
        .query_row(params, |row| row.try_into())
        .optional()
        .map_err(|e| e.into())
}

// NOTE: conn.interact only errors on thread panic or thread abort, so just unwrap it and panic if it fails.
impl MooseDB<Sqlite3Error> for Pool {
    async fn len(&self) -> Result<usize, Sqlite3Error> {
        let conn = self.get().await?;
        conn.interact(|conn| {
            conn.prepare_cached(LEN_MOOSE)?
                .query_row([], |row| row.get(0))
                .map_err(|e| e.into())
        })
        .await
        .unwrap()
    }

    async fn latest(&self) -> Result<Option<Moose>, Sqlite3Error> {
        let conn = self.get().await?;
        conn.interact(|conn| query_moose(conn, LAST_MOOSE, []))
            .await
            .unwrap()
    }

    async fn oldest(&self) -> Result<Option<Moose>, Sqlite3Error> {
        let conn = self.get().await?;
        conn.interact(|conn| query_moose(conn, GET_MOOSE_IDX, [0]))
            .await
            .unwrap()
    }

    async fn random(&self) -> Result<Option<Moose>, Sqlite3Error> {
        let conn = self.get().await?;
        conn.interact(|conn| {
            let tx = conn.transaction()?;
            let len: usize = tx
                .prepare_cached(LEN_MOOSE)?
                .query_row([], |row| row.get(0))?;
            if len == 0 {
                return Ok(None);
            }
            let rand_idx = rand::thread_rng().r#gen_range(0..len);
            let res = query_moose(&tx, GET_MOOSE_IDX, [rand_idx])?;
            tx.commit()?;
            Ok(res)
        })
        .await
        .unwrap()
    }

    async fn is_empty(&self) -> bool {
        let count = self.len().await.unwrap_or(0);
        count == 0
    }

    async fn get_page_count(&self) -> Result<usize, Sqlite3Error> {
        let moose_count = self.len().await?;
        Ok(moose_count / PAGE_SIZE + usize::from(moose_count % PAGE_SIZE > 0))
    }

    async fn get_moose(&self, moose: &str) -> Result<Option<Moose>, Sqlite3Error> {
        let conn = self.get().await?;
        let moose = moose.to_owned();
        conn.interact(move |conn| query_moose(conn, GET_MOOSE, [moose]))
            .await
            .unwrap()
    }

    async fn get_moose_page(&self, page_num: usize) -> Result<Vec<Moose>, Sqlite3Error> {
        let conn = self.get().await?;
        let q = conn
            .interact(move |conn| -> Result<Vec<Moose>, rusqlite::Error> {
                let start = page_num * PAGE_SIZE;
                let end = page_num * PAGE_SIZE + PAGE_SIZE;
                Ok(conn
                    .prepare_cached(GET_MOOSE_PAGE)?
                    .query_map([start, end], |row| row.try_into())?
                    .flat_map(|m| match m {
                        Ok(moose) => Some(moose),
                        Err(e) => {
                            eprintln!("{}", e);
                            None
                        }
                    })
                    .collect::<Vec<Moose>>())
            })
            .await
            .unwrap();
        match q {
            Ok(m) => Ok(m),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(vec![]),
            Err(e) => Err(e.into()),
        }
    }

    async fn search_moose(
        &self,
        query: &str,
        page_num: usize,
    ) -> Result<MooseSearchPage, Sqlite3Error> {
        let conn = self.get().await?;
        let query = escape_query(query);
        let q = conn
            .interact(move |conn| -> Result<MooseSearchPage, rusqlite::Error> {
                let result = conn
                    .prepare_cached(SEARCH_MOOSE_PAGE)?
                    .query_map([query], |row| {
                        Ok(MooseSearch {
                            page: row.get::<_, usize>(6)? / PAGE_SIZE,
                            moose: row.try_into()?,
                        })
                    })?
                    .flat_map(|m| match m {
                        Ok(res) => Some(res),
                        Err(e) => {
                            eprintln!("ERROR: [WEB/SEARCH] {}", e);
                            None
                        }
                    })
                    .collect::<Vec<MooseSearch>>();
                let pages = result.len() / PAGE_SIZE;
                if PAGE_SEARCH_LIM <= page_num {
                    return Ok(MooseSearchPage {
                        pages,
                        result: vec![],
                    });
                }
                let page_off = page_num * PAGE_SIZE;
                let page_lim = page_off + PAGE_SIZE;
                let result = result
                    .into_iter()
                    .skip(page_off)
                    .take(page_lim)
                    .collect::<Vec<_>>();
                Ok(MooseSearchPage { pages, result })
            })
            .await
            .unwrap();
        match q {
            Ok(m) => Ok(m),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(MooseSearchPage {
                pages: 0,
                result: vec![],
            }),
            Err(e) => Err(e.into()),
        }
    }

    async fn insert_moose(&self, moose: Moose) -> Result<(), Sqlite3Error> {
        let conn = self.get().await?;
        conn.interact(move |conn| {
            conn.prepare_cached(INSERT_MOOSE_WITH_COMPUTED_POS)
                .unwrap()
                .execute(MooseToSqlParams::from(&moose))
        })
        .await
        .unwrap()?;
        Ok(())
    }

    async fn dump_moose(&self, moose_dump: PathBuf) -> Result<(), Sqlite3Error> {
        let con = self.get().await?;
        con.interact(move |con| {
            // parent only fails when totally rooted.
            let tdir = match moose_dump.parent() {
                Some(p) => p,
                None => return Err(Sqlite3Error::StrangeMooseDumpPath()),
            };
            let r: u64 = rand::random();
            let tdir = tdir.join(format!(".moose.json.{:x}", r));

            let file = File::create(&tdir)?;
            let mut bufw = BufWriter::new(file);
            let mut start = true;

            let mut q = con.prepare_cached(DUMP_MOOSE)?;
            let mut w = q.query([])?;
            while let Ok(Some(row)) = w.next() {
                if start {
                    bufw.write_all(b"[")?;
                    start = false;
                } else {
                    bufw.write_all(b",")?;
                }
                let moose: Moose = row.try_into()?;
                let moose = serde_json::to_vec(&moose)?;
                bufw.write_all(&moose)?;
            }
            bufw.write_all(b"]")?;

            let inner = bufw.into_inner()?;
            inner.sync_data()?;
            drop(inner);
            fs::rename(tdir, moose_dump)?;

            println!("INFO: [DUMP] Done dumping moose.");
            Ok(())
        })
        .await
        .unwrap()
    }

    async fn bulk_import(
        &self,
        moose_in: Option<PathBuf>,
        dup_behavior: super::BulkModeDupe,
    ) -> Result<(), Sqlite3Error> {
        let mut moose_in = match moose_in {
            Some(path) => {
                let file = BufReader::new(std::fs::File::open(path)?);
                serde_json::from_reader::<_, Vec<MooseAny>>(file).unwrap()
            }
            None => serde_json::from_reader::<_, Vec<MooseAny>>(std::io::stdin().lock())?,
        }
        .drain(..)
        .map(|m| m.into())
        .collect::<Vec<Moose>>();

        moose_in.sort_unstable_by(|lhs, rhs| lhs.created.cmp(&rhs.created));
        let conn = self.get().await?;
        conn.interact(move |conn| {
            let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
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
        .map_err(|e| e.into())
    }
}
