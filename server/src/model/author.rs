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
