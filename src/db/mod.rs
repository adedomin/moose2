use crate::{
    config::{self, get_config},
    model::{moose::Moose, PAGE_SIZE},
};
use actix_web::web;
use r2d2::ManageConnection;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::params;
use std::{io::BufReader, path::PathBuf};

use self::query::{
    CREATE_TABLE, GET_MOOSE, GET_MOOSE_IDX, GET_MOOSE_PAGE, INSERT_MOOSE,
    INSERT_MOOSE_WITH_COMPUTED_POS, LAST_MOOSE, LEN_MOOSE, SEARCH_MOOSE_PAGE,
};

pub mod query;

pub type Pool = r2d2::Pool<SqliteConnectionManager>;
pub type Connection = r2d2::PooledConnection<r2d2_sqlite::SqliteConnectionManager>;

pub fn open_db() -> Pool {
    let moose_path = get_config().get_moose_path();
    config::create_parent_dirs(&moose_path).unwrap();
    let conn_man = SqliteConnectionManager::file(moose_path);
    {
        let con = conn_man.connect().unwrap();
        con.set_prepared_statement_cache_capacity(32);
        con.execute_batch(CREATE_TABLE).unwrap();
    }
    Pool::new(conn_man).unwrap()
}

pub fn moose_bulk_import(moose_in: Option<PathBuf>) {
    let mut moose_in = match moose_in {
        Some(path) => {
            let file = BufReader::new(std::fs::File::open(path).unwrap());
            serde_json::from_reader::<_, Vec<Moose>>(file).unwrap()
        }
        None => serde_json::from_reader::<_, Vec<Moose>>(std::io::stdin().lock()).unwrap(),
    };
    moose_in.sort_unstable_by(|lhs, rhs| lhs.created.cmp(&rhs.created));
    let db = open_db();
    let mut conn = db.get().unwrap();
    let tx = conn.transaction().unwrap();
    for (i, moose) in moose_in.iter().enumerate() {
        tx.prepare_cached(INSERT_MOOSE)
            .unwrap()
            .execute(params![
                moose.name,
                i,
                moose.image,
                moose.dimensions,
                moose.created,
                moose.author,
            ])
            .unwrap();
    }
    tx.commit().unwrap();
}

#[derive(thiserror::Error, Debug)]
pub enum QueryError {
    #[error("Pool Connection Error: {0}")]
    ConnectionPool(#[from] r2d2::Error),
    #[error("Sqlite3 Error: {0}")]
    Sqlite3(#[from] rusqlite::Error),
    #[error("Runtime Blocking Error: {0}")]
    Block(#[from] actix_web::error::BlockingError),
}

#[async_trait::async_trait]
pub trait MooseDB {
    async fn len(&self) -> Result<usize, QueryError>;
    async fn last(&self) -> Result<Option<Moose>, QueryError>;
    async fn is_empty(&self) -> bool;
    async fn get_page_count(&self) -> Result<usize, QueryError>;
    async fn get_moose(&self, moose: &str) -> Result<Option<Moose>, QueryError>;
    async fn get_moose_idx(&self, idx: usize) -> Result<Option<Moose>, QueryError>;
    async fn get_moose_page(&self, page_num: usize) -> Result<Vec<Moose>, QueryError>;
    async fn search_moose(&self, query: &str) -> Result<Vec<(usize, Moose)>, QueryError>;
    async fn insert_moose(&self, moose: Moose) -> Result<(), QueryError>;
}

fn handle_opt_q(res: Result<Moose, rusqlite::Error>) -> Result<Option<Moose>, QueryError> {
    match res {
        Ok(m) => Ok(Some(m)),
        Err(e) if e == rusqlite::Error::QueryReturnedNoRows => Ok(None),
        Err(e) => Err(e.into()),
    }
}

#[async_trait::async_trait]
impl MooseDB for Pool {
    async fn len(&self) -> Result<usize, QueryError> {
        let pool = self.clone();
        let conn = web::block(move || pool.get()).await??;
        Ok(web::block(move || -> Result<usize, rusqlite::Error> {
            conn.prepare_cached(LEN_MOOSE)?
                .query_row([], |row| row.get(0))
                .or(Ok(0usize))
        })
        .await??)
    }

    async fn last(&self) -> Result<Option<Moose>, QueryError> {
        let pool = self.clone();
        let conn = web::block(move || pool.get()).await??;
        web::block(move || {
            let q = conn.prepare_cached(LAST_MOOSE)?.query_row([], |row| {
                Ok(Moose {
                    name: row.get(0)?,
                    image: row.get(1)?,
                    dimensions: row.get(2)?,
                    created: row.get(3)?,
                    author: row.get(4)?,
                })
            });
            handle_opt_q(q)
        })
        .await?
    }

    async fn is_empty(&self) -> bool {
        let count = self.get_page_count().await.unwrap_or(0);
        count == 0
    }

    async fn get_page_count(&self) -> Result<usize, QueryError> {
        let moose_count = self.len().await?;
        Ok(moose_count / PAGE_SIZE + usize::from(moose_count % PAGE_SIZE > 0))
    }

    async fn get_moose(&self, moose: &str) -> Result<Option<Moose>, QueryError> {
        let pool = self.clone();
        let conn = web::block(move || pool.get()).await??;
        let moose = moose.to_owned();
        web::block(move || {
            let q = conn.prepare_cached(GET_MOOSE)?.query_row([moose], |row| {
                Ok(Moose {
                    name: row.get(0)?,
                    image: row.get(1)?,
                    dimensions: row.get(2)?,
                    created: row.get(3)?,
                    author: row.get(4)?,
                })
            });
            handle_opt_q(q)
        })
        .await?
    }

    async fn get_moose_idx(&self, idx: usize) -> Result<Option<Moose>, QueryError> {
        let pool = self.clone();
        let conn = web::block(move || pool.get()).await??;
        web::block(move || {
            let q = conn.prepare_cached(GET_MOOSE_IDX)?.query_row([idx], |row| {
                Ok(Moose {
                    name: row.get(0)?,
                    image: row.get(1)?,
                    dimensions: row.get(2)?,
                    created: row.get(3)?,
                    author: row.get(4)?,
                })
            });
            handle_opt_q(q)
        })
        .await?
    }

    async fn get_moose_page(&self, page_num: usize) -> Result<Vec<Moose>, QueryError> {
        let pool = self.clone();
        let conn = web::block(move || pool.get()).await??;
        let q = web::block(move || -> Result<Vec<Moose>, rusqlite::Error> {
            let start = page_num * 12;
            let end = page_num * 12 + 12;
            Ok(conn
                .prepare_cached(GET_MOOSE_PAGE)?
                .query_map([start, end], |row| {
                    Ok(Moose {
                        name: row.get(0)?,
                        image: row.get(1)?,
                        dimensions: row.get(2)?,
                        created: row.get(3)?,
                        author: row.get(4)?,
                    })
                })?
                .flat_map(|m| match m {
                    Ok(moose) => Some(moose),
                    Err(e) => {
                        eprintln!("{}", e);
                        None
                    }
                })
                .collect::<Vec<Moose>>())
        })
        .await?;
        match q {
            Ok(m) => Ok(m),
            Err(e) if e == rusqlite::Error::QueryReturnedNoRows => Ok(vec![]),
            Err(e) => Err(e.into()),
        }
    }

    async fn search_moose(&self, query: &str) -> Result<Vec<(usize, Moose)>, QueryError> {
        let pool = self.clone();
        let conn = web::block(move || pool.get()).await??;
        let query = query.to_owned();
        let q = web::block(move || -> Result<Vec<(usize, Moose)>, rusqlite::Error> {
            Ok(conn
                .prepare_cached(SEARCH_MOOSE_PAGE)?
                .query_map([query], |row| {
                    Ok((
                        row.get::<_, usize>(0)? / PAGE_SIZE,
                        Moose {
                            name: row.get(1)?,
                            image: row.get(2)?,
                            dimensions: row.get(3)?,
                            created: row.get(4)?,
                            author: row.get(5)?,
                        },
                    ))
                })?
                .flat_map(|m| match m {
                    Ok(res) => Some(res),
                    Err(e) => {
                        eprintln!("{}", e);
                        None
                    }
                })
                .collect::<Vec<(usize, Moose)>>())
        })
        .await?;
        match q {
            Ok(m) => Ok(m),
            Err(e) if e == rusqlite::Error::QueryReturnedNoRows => Ok(vec![]),
            Err(e) => Err(e.into()),
        }
    }

    async fn insert_moose(&self, moose: Moose) -> Result<(), QueryError> {
        let pool = self.clone();
        let conn = web::block(move || pool.get()).await??;
        let _ = web::block(move || {
            conn.prepare_cached(INSERT_MOOSE_WITH_COMPUTED_POS)
                .unwrap()
                .execute(params![
                    moose.name,
                    moose.image,
                    moose.dimensions,
                    moose.created,
                    moose.author,
                ])
        })
        .await??;
        Ok(())
    }
}
