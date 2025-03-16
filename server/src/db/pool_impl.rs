use crate::model::{
    PAGE_SEARCH_LIM, PAGE_SIZE,
    moose::Moose,
    pages::{MooseSearch, MooseSearchPage},
};

use super::{
    MooseDB, MooseToSqlParams, Pool, QueryError,
    query::{
        GET_MOOSE, GET_MOOSE_IDX, GET_MOOSE_PAGE, INSERT_MOOSE_WITH_COMPUTED_POS, LAST_MOOSE,
        LEN_MOOSE, SEARCH_MOOSE_PAGE,
    },
    utils::escape_query,
};

use rand::Rng;
use rusqlite::{Connection, OptionalExtension, Params};

fn query_moose<P: Params>(
    conn: &Connection,
    sql: &'static str,
    params: P,
) -> Result<Option<Moose>, QueryError> {
    conn.prepare_cached(sql)?
        .query_row(params, |row| row.try_into())
        .optional()
        .map_err(|e| e.into())
}

impl MooseDB for Pool {
    async fn len(&self) -> Result<usize, QueryError> {
        let conn = self.get().await?;
        conn.interact(|conn| {
            conn.prepare_cached(LEN_MOOSE)?
                .query_row([], |row| row.get(0))
                .map_err(|e| e.into())
        })
        .await?
    }

    async fn latest(&self) -> Result<Option<Moose>, QueryError> {
        let conn = self.get().await?;
        conn.interact(|conn| query_moose(conn, LAST_MOOSE, []))
            .await?
    }

    async fn oldest(&self) -> Result<Option<Moose>, QueryError> {
        let conn = self.get().await?;
        conn.interact(|conn| query_moose(conn, GET_MOOSE_IDX, [0]))
            .await?
    }

    async fn random(&self) -> Result<Option<Moose>, QueryError> {
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
        .await?
    }

    async fn is_empty(&self) -> bool {
        let count = self.len().await.unwrap_or(0);
        count == 0
    }

    async fn get_page_count(&self) -> Result<usize, QueryError> {
        let moose_count = self.len().await?;
        Ok(moose_count / PAGE_SIZE + usize::from(moose_count % PAGE_SIZE > 0))
    }

    async fn get_moose(&self, moose: &str) -> Result<Option<Moose>, QueryError> {
        let conn = self.get().await?;
        let moose = moose.to_owned();
        conn.interact(move |conn| query_moose(conn, GET_MOOSE, [moose]))
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
                .execute(MooseToSqlParams::from(&moose))
        })
        .await??;
        Ok(())
    }
}
