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

use serde::{Deserialize, Deserializer};

use super::PAGE_SEARCH_LIM;

#[derive(Deserialize)]
pub struct SearchQuery {
    #[serde(
        alias = "q",
        deserialize_with = "from_qstring",
        default = "default_query"
    )]
    pub query: String,
    #[serde(
        alias = "p",
        deserialize_with = "from_page_num",
        default = "page_num_default"
    )]
    pub page: usize,
    #[serde(default = "nojs_default")]
    pub nojs: bool,
}

fn from_qstring<'de, D: Deserializer<'de>>(deserializer: D) -> Result<String, D::Error> {
    String::deserialize(deserializer).and_then(|q| {
        if q.is_empty() {
            Err(serde::de::Error::custom("query is empty"))
        } else if q.len() > 64 {
            Err(serde::de::Error::custom("query too large"))
        } else {
            Ok(q)
        }
    })
}

fn from_page_num<'de, D: Deserializer<'de>>(deserializer: D) -> Result<usize, D::Error> {
    usize::deserialize(deserializer).and_then(|p| {
        if PAGE_SEARCH_LIM <= p {
            Err(serde::de::Error::custom(format!(
                "Page number limit exceeded. limit: {PAGE_SEARCH_LIM}"
            )))
        } else {
            Ok(p)
        }
    })
}

fn default_query() -> String {
    String::new()
}

fn page_num_default() -> usize {
    0
}

fn nojs_default() -> bool {
    false
}

// #[derive(Deserialize)]
// pub struct LoginRedir {
//     #[serde(default = "redir_default")]
//     pub redir: String,
//     #[serde(default = "debug_default")]
//     pub debug: bool,
// }

// fn redir_default() -> String {
//     "/".to_owned()
// }

// fn debug_default() -> bool {
//     false
// }
