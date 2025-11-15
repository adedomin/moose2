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

use rusqlite::{
    ToSql,
    types::{FromSql, ToSqlOutput},
};
use serde::{Deserialize, Serialize};

use super::author::Author;

#[derive(Debug, Deserialize, Serialize, Clone, Copy)]
pub enum VoteFlag {
    None = 0,
    Up = 1,
    Down = -1,
}

impl From<i64> for VoteFlag {
    fn from(value: i64) -> Self {
        match value {
            0 => VoteFlag::None,
            1 => VoteFlag::Up,
            -1 => VoteFlag::Down,
            _ => VoteFlag::None,
        }
    }
}

impl ToSql for VoteFlag {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok((*self as i64).into())
    }
}

impl FromSql for VoteFlag {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        match value.as_i64_or_null() {
            Ok(num) => match num {
                Some(num) => Ok(VoteFlag::from(num)),
                None => Ok(VoteFlag::None),
            },
            Err(_) => Ok(VoteFlag::None),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Vote(pub Author, pub VoteFlag);
