use rusqlite::{
    types::{FromSql, ToSqlOutput},
    ToSql,
};
use serde::{Deserialize, Serialize};

/// width, height, total
pub const DEFAULT_SIZE: (usize, usize, usize) = (26, 15, 26 * 15);
pub const HD_SIZE: (usize, usize, usize) = (36, 22, 36 * 22);

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub enum Dimensions {
    Default,
    HD,
    Custom(usize, usize),
}

impl Dimensions {
    /// Get the width and height of the given dimension.
    pub fn width_height(&self) -> (usize, usize, usize) {
        match self {
            Dimensions::Default => DEFAULT_SIZE,
            Dimensions::HD => HD_SIZE,
            Dimensions::Custom(width, height) => (*width, *height, *width * *height),
        }
    }

    /// Decipher the likely dimensions of a moose by their 1-D Image size.
    pub fn from_len(image: &[u8]) -> Option<Self> {
        if image.len() == DEFAULT_SIZE.2 {
            Some(Self::Default)
        } else if image.len() == HD_SIZE.2 {
            Some(Self::HD)
        } else {
            None
        }
    }
}

impl Default for Dimensions {
    fn default() -> Self {
        Self::Default
    }
}

impl ToSql for Dimensions {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        Ok(ToSqlOutput::Owned(
            serde_json::to_string(self).unwrap().into(),
        ))
    }
}

impl FromSql for Dimensions {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        Ok(serde_json::from_str(value.as_str()?).unwrap())
    }
}

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
