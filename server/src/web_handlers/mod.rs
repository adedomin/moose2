use crate::{model::PAGE_SEARCH_LIM, AppData};
use actix_web::{http::header::IF_NONE_MATCH, web, HttpRequest};
use serde::{Deserialize, Deserializer};

pub mod api;
pub mod display;
pub mod oauth2_gh;
pub mod static_files;

pub type MooseWebData = web::Data<AppData>;

/// Intended to be used with: .iter().fold(num, fold_decimal)
fn fold_decimal(acc: u32, chr: &u8) -> u32 {
    acc * 10 + (chr - b'0') as u32
}

pub fn if_none_match(body: &[u8], req: &HttpRequest) -> (bool, u32) {
    let crc32 = crc32fast::hash(body);
    let matched = if let Some(etag) = req
        .headers()
        .get(IF_NONE_MATCH)
        .map(|header| header.as_bytes())
    {
        let etag: u32 = etag
            .iter()
            .filter(|&&chr| chr > (b'0' - 1) && chr < (b'9' + 1))
            .fold(0, fold_decimal);

        etag == crc32
    } else {
        false
    };
    (matched, crc32)
}

pub fn if_none_match_md5(body: &[u8], req: &HttpRequest) -> (bool, String) {
    let comp_md5 = format!("{:x}", md5::compute(body));
    let matched = if let Some(etag) = req.headers().get(IF_NONE_MATCH).map(|header| {
        header
            .as_bytes()
            .iter()
            .cloned()
            .filter(|x| matches!(x, b'0'..=b'9' | b'a'..=b'f' | b'A'..=b'F'))
            .collect::<Vec<u8>>()
    }) {
        etag == comp_md5.as_bytes()
    } else {
        false
    };
    (matched, comp_md5)
}

#[derive(Deserialize)]
pub struct SearchQuery {
    #[serde(alias = "q", deserialize_with = "from_qstring")]
    pub query: String,
    #[serde(
        alias = "p",
        deserialize_with = "from_page_num",
        default = "page_num_default"
    )]
    pub page: usize,
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

fn page_num_default() -> usize {
    0
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
