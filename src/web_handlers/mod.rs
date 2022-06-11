use actix_web::{http::header::IF_NONE_MATCH, HttpRequest};

pub mod api;
pub mod display;
pub mod static_files;

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
