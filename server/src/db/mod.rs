use crate::{
    config::{self, RunConfig},
    model::{
        moose::Moose,
        pages::{MooseSearch, MooseSearchPage},
        PAGE_SEARCH_LIM, PAGE_SIZE,
    },
};
use rusqlite::params;
use std::{io::BufReader, path::PathBuf};

use self::query::{
    CREATE_TABLE, GET_MOOSE, GET_MOOSE_IDX, GET_MOOSE_PAGE, INSERT_MOOSE,
    INSERT_MOOSE_WITH_COMPUTED_POS, LAST_MOOSE, LEN_MOOSE, SEARCH_MOOSE_PAGE,
};

pub mod query;

pub type Pool = deadpool_sqlite::Pool;
pub type Connection = deadpool_sqlite::Object;

pub async fn open_db(rc: &RunConfig) -> Pool {
    let moose_path = rc.get_moose_path();
    config::create_parent_dirs(&moose_path).unwrap();
    let cfg = deadpool_sqlite::Config::new(&moose_path);
    let pool = cfg
        .create_pool(deadpool_sqlite::Runtime::Tokio1)
        .expect("Expected to build Sqlite3 pool.");

    {
        let con = pool.get().await.unwrap();
        con.interact(|con| {
            con.set_prepared_statement_cache_capacity(32);
            con.execute_batch(CREATE_TABLE).unwrap();
        })
        .await
        .unwrap()
    }
    pool
}

pub async fn moose_bulk_import(moose_in: Option<PathBuf>, rc: &RunConfig) {
    let mut moose_in = match moose_in {
        Some(path) => {
            let file = BufReader::new(std::fs::File::open(path).unwrap());
            serde_json::from_reader::<_, Vec<Moose>>(file).unwrap()
        }
        None => serde_json::from_reader::<_, Vec<Moose>>(std::io::stdin().lock()).unwrap(),
    };
    moose_in.sort_unstable_by(|lhs, rhs| lhs.created.cmp(&rhs.created));
    let db = open_db(rc).await;
    let conn = db.get().await.unwrap();
    conn.interact(move |conn| {
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
    })
    .await
    .unwrap();
}

#[derive(thiserror::Error, Debug)]
pub enum QueryError {
    #[error("Pool Connection Error: {0}")]
    PoolInteractError(#[from] deadpool_sqlite::InteractError),
    #[error("Pool Connection Error: {0}")]
    ConnectionPool(#[from] deadpool_sqlite::PoolError),
    #[error("Sqlite3 Error: {0}")]
    Sqlite3(#[from] rusqlite::Error),
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
    async fn search_moose(
        &self,
        query: &str,
        page_num: usize,
    ) -> Result<MooseSearchPage, QueryError>;
    async fn insert_moose(&self, moose: Moose) -> Result<(), QueryError>;
}

fn handle_opt_q(res: Result<Moose, rusqlite::Error>) -> Result<Option<Moose>, QueryError> {
    match res {
        Ok(m) => Ok(Some(m)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

#[async_trait::async_trait]
impl MooseDB for Pool {
    async fn len(&self) -> Result<usize, QueryError> {
        let conn = self.get().await?;
        conn.interact(|conn| {
            conn.prepare_cached(LEN_MOOSE)?
                .query_row([], |row| row.get(0))
                .or(Ok(0usize))
        })
        .await?
    }

    async fn last(&self) -> Result<Option<Moose>, QueryError> {
        let conn = self.get().await?;
        conn.interact(|conn| {
            let q = conn.prepare_cached(LAST_MOOSE)?.query_row([], |row| {
                Ok(Moose {
                    name: row.get(0)?,
                    image: row.get(1)?,
                    dimensions: row.get(2)?,
                    created: row.get(3)?,
                    author: row.get(4)?,
                    upvotes: row.get(5)?,
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
        let conn = self.get().await?;
        let moose = moose.to_owned();
        conn.interact(move |conn| {
            let q = conn.prepare_cached(GET_MOOSE)?.query_row([moose], |row| {
                Ok(Moose {
                    name: row.get(0)?,
                    image: row.get(1)?,
                    dimensions: row.get(2)?,
                    created: row.get(3)?,
                    author: row.get(4)?,
                    upvotes: row.get(5)?,
                })
            });
            handle_opt_q(q)
        })
        .await?
    }

    async fn get_moose_idx(&self, idx: usize) -> Result<Option<Moose>, QueryError> {
        let conn = self.get().await?;
        conn.interact(move |conn| {
            let q = conn.prepare_cached(GET_MOOSE_IDX)?.query_row([idx], |row| {
                Ok(Moose {
                    name: row.get(0)?,
                    image: row.get(1)?,
                    dimensions: row.get(2)?,
                    created: row.get(3)?,
                    author: row.get(4)?,
                    upvotes: row.get(5)?,
                })
            });
            handle_opt_q(q)
        })
        .await?
    }

    async fn get_moose_page(&self, page_num: usize) -> Result<Vec<Moose>, QueryError> {
        let conn = self.get().await?;
        let q = conn
            .interact(move |conn| -> Result<Vec<Moose>, rusqlite::Error> {
                let start = page_num * PAGE_SIZE;
                let end = page_num * PAGE_SIZE + PAGE_SIZE;
                Ok(conn
                    .prepare_cached(GET_MOOSE_PAGE)?
                    .query_map([start, end], |row| {
                        Ok(Moose {
                            name: row.get(0)?,
                            image: row.get(1)?,
                            dimensions: row.get(2)?,
                            created: row.get(3)?,
                            author: row.get(4)?,
                            upvotes: row.get(5)?,
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
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(vec![]),
            Err(e) => Err(e.into()),
        }
    }

    async fn search_moose(
        &self,
        query: &str,
        page_num: usize,
    ) -> Result<MooseSearchPage, QueryError> {
        let conn = self.get().await?;
        let query = query.to_owned();
        let q = conn
            .interact(move |conn| -> Result<MooseSearchPage, rusqlite::Error> {
                let result = conn
                    .prepare_cached(SEARCH_MOOSE_PAGE)?
                    .query_map([query], |row| {
                        Ok(MooseSearch {
                            page: row.get::<_, usize>(0)? / PAGE_SIZE,
                            moose: Moose {
                                name: row.get(1)?,
                                image: row.get(2)?,
                                dimensions: row.get(3)?,
                                created: row.get(4)?,
                                author: row.get(5)?,
                                upvotes: row.get(6)?,
                            },
                        })
                    })?
                    .flat_map(|m| match m {
                        Ok(res) => Some(res),
                        Err(e) => {
                            eprintln!("{}", e);
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
                    .get(page_off..page_lim)
                    .map(|mslice| mslice.to_vec())
                    .unwrap_or(vec![]);
                Ok(MooseSearchPage { pages, result })
            })
            .await?;
        match q {
            Ok(m) => Ok(m),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(MooseSearchPage {
                pages: 0,
                result: vec![],
            }),
            Err(e) => Err(e.into()),
        }
    }

    async fn insert_moose(&self, moose: Moose) -> Result<(), QueryError> {
        let conn = self.get().await?;
        conn.interact(move |conn| {
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
