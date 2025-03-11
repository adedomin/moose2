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

use crate::model::app_data::AppData;
use actix_web::{HttpRequest, http::header::IF_NONE_MATCH, web};

pub mod api;
pub mod display;
pub mod oauth2_gh;
pub mod static_files;

pub type MooseWebData = web::Data<AppData>;

pub const JSON_TYPE: (&str, &str) = ("Content-Type", "application/json");

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
            .filter(|x| x.is_ascii_hexdigit())
            .collect::<Vec<u8>>()
    }) {
        etag == comp_md5.as_bytes()
    } else {
        false
    };
    (matched, comp_md5)
}
