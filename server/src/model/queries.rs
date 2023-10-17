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
