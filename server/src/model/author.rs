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

use axum::extract::{FromRef, FromRequestParts, OptionalFromRequestParts};
use http::{StatusCode, request::Parts};
use rusqlite::{
    ToSql,
    types::{FromSql, ToSqlOutput},
};
use serde::{Deserialize, Deserializer, Serialize};
use tower_cookies::Cookies;

use crate::web_handlers::{ApiError, LOGIN_COOKIE, MooseWebData};

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub enum Author {
    #[default]
    Anonymous,
    Alias(#[serde(deserialize_with = "github_valid_user")] String),
    GitHub(#[serde(deserialize_with = "github_valid_user")] String),
}

impl Author {
    pub fn new_alias(author: String) -> Result<Self, &'static str> {
        gh_valid_user(&author)?;
        Ok(Self::Alias(author))
    }

    pub fn new_gh(author: String) -> Result<Self, &'static str> {
        gh_valid_user(&author)?;
        Ok(Self::GitHub(author))
    }

    pub fn displayable(self) -> Option<String> {
        match self {
            Author::Anonymous => None,
            Author::Alias(a) => Some(format!("(Alias) {a}")),
            Author::GitHub(a) => Some(a),
        }
    }
}

const GH_MAX_BYTE_LEN: usize = 39;

fn gh_valid_user(author: &str) -> Result<(), &'static str> {
    if author.is_empty() {
        return Err("Author name cannot be empty.");
    }

    if author.len() > GH_MAX_BYTE_LEN {
        return Err("Author name is too long: >39 bytes.");
    }

    author.split('-').try_for_each(|word| {
        if word.is_empty() {
            return Err("Author name can only have hyphens between words.");
        }

        if word.contains(|c: char| !c.is_ascii_alphanumeric()) {
            return Err("Author name must be alphanumeric with optional hyphens.");
        }

        Ok(())
    })
}

fn github_valid_user<'de, D: Deserializer<'de>>(deserializer: D) -> Result<String, D::Error> {
    String::deserialize(deserializer).and_then(|author| {
        gh_valid_user(&author).map_err(serde::de::Error::custom)?;
        Ok(author)
    })
}

pub fn default_author() -> Author {
    Author::Anonymous
}

impl ToSql for Author {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        match self {
            Author::Anonymous => rusqlite::types::Null.to_sql(),
            Author::Alias(author) => Ok(ToSqlOutput::from(format!("Alias__{author}"))),
            Author::GitHub(author) => Ok(ToSqlOutput::from(format!("GitHub__{author}"))),
        }
    }
}

impl FromSql for Author {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        match value.as_str_or_null()? {
            Some(author) => {
                if let Some(a) = author.strip_prefix("Alias__") {
                    Ok(Author::Alias(a.to_owned()))
                } else if let Some(a) = author.strip_prefix("GitHub__") {
                    Ok(Author::GitHub(a.to_owned()))
                } else {
                    // fallback for legacy
                    Ok(Author::GitHub(author.to_owned()))
                }
            }
            None => Ok(Author::Anonymous),
        }
    }
}

impl<S> FromRequestParts<S> for Author
where
    MooseWebData: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Some(cookies) = parts.extensions.get::<Cookies>() else {
            return Err(ApiError::new(
                "Can't extract cookies. Is tower-cookies applied!?",
            ));
        };
        let state = MooseWebData::from_ref(state);
        let author = cookies
            .private(&state.cookie_key)
            .get(LOGIN_COOKIE)
            .and_then(|c| serde_json::from_str::<Author>(c.value()).ok())
            .unwrap_or_default();
        Ok(author)
    }
}

pub enum AuthenticatedAuthor {
    GitHub(String),
}

impl TryFrom<Author> for AuthenticatedAuthor {
    type Error = ();

    fn try_from(value: Author) -> Result<Self, Self::Error> {
        match value {
            Author::Anonymous => Err(()),
            Author::Alias(_) => Err(()),
            Author::GitHub(a) => Ok(Self::GitHub(a)),
        }
    }
}

impl From<AuthenticatedAuthor> for Author {
    fn from(value: AuthenticatedAuthor) -> Self {
        match value {
            AuthenticatedAuthor::GitHub(a) => Author::GitHub(a),
        }
    }
}

impl<S> FromRequestParts<S> for AuthenticatedAuthor
where
    MooseWebData: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Some(cookies) = parts.extensions.get::<Cookies>() else {
            return Err(ApiError::new(
                "Can't extract cookies. Is tower-cookies applied!?",
            ));
        };
        let state = MooseWebData::from_ref(state);
        cookies
            .private(&state.cookie_key)
            .get(LOGIN_COOKIE)
            .and_then(|c| serde_json::from_str::<Author>(c.value()).ok())
            .ok_or(ApiError::new_with_status(
                StatusCode::UNAUTHORIZED,
                "Authentication Required.",
            ))? // no auth cookie
            .try_into()
            .map_err(|_| {
                ApiError::new_with_status(StatusCode::FORBIDDEN, "Aliases are not authenticated.")
            }) // only GitHub authenticated are currently authenticated.
    }
}

impl<S> OptionalFromRequestParts<S> for AuthenticatedAuthor
where
    MooseWebData: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> Result<Option<Self>, Self::Rejection> {
        let Some(cookies) = parts.extensions.get::<Cookies>() else {
            return Err(ApiError::new(
                "Can't extract cookies. Is tower-cookies applied!?",
            ));
        };
        let state = MooseWebData::from_ref(state);
        Ok(cookies
            .private(&state.cookie_key)
            .get(LOGIN_COOKIE)
            .and_then(|c| serde_json::from_str::<Author>(c.value()).ok())
            .and_then(|author| author.try_into().ok()))
    }
}

#[cfg(test)]
mod tests {
    use super::{GH_MAX_BYTE_LEN, gh_valid_user};

    #[test]
    fn test_allowed_author_names() {
        assert!(gh_valid_user("-kebab-name-").is_err());
        assert!(gh_valid_user("kebab-name-no-trailing-leading").is_ok());
        assert!(gh_valid_user("kebab-name-no-leading-").is_err());
        assert!(gh_valid_user("-kebab-name-leading-only").is_err());
        assert!(gh_valid_user("kebab--double-hyphen").is_err());
        assert!(gh_valid_user("" /* empty case */).is_err());
        let too_big = "a".repeat(GH_MAX_BYTE_LEN * 2);
        assert!(gh_valid_user(&too_big).is_err());
        assert!(gh_valid_user(&too_big[0..GH_MAX_BYTE_LEN]).is_ok());
        assert!(gh_valid_user("  spaces in name  ").is_err());
        assert!(gh_valid_user("spaces in name").is_err());
        assert!(gh_valid_user("._*%#").is_err());
    }
}
