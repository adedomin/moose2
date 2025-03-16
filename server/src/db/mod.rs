/* Copyright (C) 2024  Anthony DeDominic
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use time::OffsetDateTime;

use crate::model::{author::Author, dimensions::Dimensions, moose::Moose, pages::MooseSearchPage};

pub mod pool_impl;
pub mod query;
pub mod utils;

pub type Pool = deadpool_sqlite::Pool;
pub type Connection = deadpool_sqlite::Object;

#[derive(thiserror::Error, Debug)]
pub enum QueryError {
    #[error("Pool Connection Interaction Error: {0}")]
    PoolInteractError(#[from] deadpool_sqlite::InteractError),
    #[error("Pool Connection Error: {0}")]
    ConnectionPool(#[from] deadpool_sqlite::PoolError),
    #[error("Sqlite3 Error: {0}")]
    Sqlite3(#[from] rusqlite::Error),
}

// NOTE: You can suppress this lint if you plan to use the trait only in your own code,
//       or do not care about auto traits like `Send` on the `Future`.
//       This code is not exported to other users, other than moose2's code.
#[allow(async_fn_in_trait)]
pub trait MooseDB {
    async fn len(&self) -> Result<usize, QueryError>;
    async fn latest(&self) -> Result<Option<Moose>, QueryError>;
    async fn oldest(&self) -> Result<Option<Moose>, QueryError>;
    async fn random(&self) -> Result<Option<Moose>, QueryError>;
    async fn is_empty(&self) -> bool;
    async fn get_page_count(&self) -> Result<usize, QueryError>;
    async fn get_moose(&self, moose: &str) -> Result<Option<Moose>, QueryError>;
    async fn get_moose_page(&self, page_num: usize) -> Result<Vec<Moose>, QueryError>;
    async fn search_moose(
        &self,
        query: &str,
        page_num: usize,
    ) -> Result<MooseSearchPage, QueryError>;
    async fn insert_moose(&self, moose: Moose) -> Result<(), QueryError>;
}

impl TryFrom<&rusqlite::Row<'_>> for Moose {
    type Error = rusqlite::Error;

    fn try_from(row: &rusqlite::Row<'_>) -> Result<Self, Self::Error> {
        Ok(Moose {
            name: row.get(0)?,
            image: row.get(1)?,
            dimensions: row.get(2)?,
            created: row.get(3)?,
            author: row.get(4)?,
            upvotes: row.get(5)?,
        })
    }
}

pub type MooseToSqlParams<'a> = (
    &'a str,
    &'a [u8],
    &'a Dimensions,
    &'a OffsetDateTime,
    &'a Author,
    &'a i64,
);

impl<'a> From<&'a Moose> for MooseToSqlParams<'a> {
    fn from(moose: &'a Moose) -> Self {
        (
            &moose.name,
            &moose.image,
            &moose.dimensions,
            &moose.created,
            &moose.author,
            &moose.upvotes,
        )
    }
}
