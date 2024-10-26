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

use rusqlite::{types::FromSql, ToSql};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum Author {
    Anonymous,
    Oauth2(String),
}

pub fn default_author() -> Author {
    Author::Anonymous
}

impl ToSql for Author {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        match self {
            Author::Anonymous => rusqlite::types::Null.to_sql(),
            Author::Oauth2(user) => user.to_sql(),
        }
    }
}

impl FromSql for Author {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        match value.as_str_or_null()? {
            Some(name) => Ok(Author::Oauth2(name.to_string())),
            None => Ok(Author::Anonymous),
        }
    }
}

impl TryInto<String> for Author {
    type Error = ();

    fn try_into(self) -> Result<String, Self::Error> {
        match self {
            Author::Anonymous => Err(()),
            Author::Oauth2(user) => Ok(user),
        }
    }
}
