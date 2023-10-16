use rusqlite::{
    types::{FromSql, ToSqlOutput},
    ToSql,
};
use serde::{Deserialize, Serialize};

use super::author::Author;

#[derive(Debug, Deserialize, Serialize, Clone, Copy)]
pub enum VoteFlag {
    None = 0isize,
    Up = 1isize,
    Down = -1isize,
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
