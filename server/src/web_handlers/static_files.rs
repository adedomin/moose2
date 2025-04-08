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

use std::path::PathBuf;

use super::{ApiError, MooseWebData};
use crate::{
    middleware::etag::crc32_etag,
    model::mime::get_mime,
    shared_data::{COLORS_JS, ERR_JS, SIZ_JS},
};
use axum::{
    Router,
    extract::{Path, Request},
    response::{IntoResponse, Response},
    routing::get,
};
use http::{
    StatusCode,
    header::{CACHE_CONTROL, CONTENT_TYPE, ETAG},
};
use include_dir::{Dir, include_dir};

const CLIENT_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/../client/src");

pub enum Static {
    Content(&'static [u8], &'static str),
    NotFound,
}

impl IntoResponse for Static {
    fn into_response(self) -> Response {
        let (body, ctype) = if let Static::Content(body, ctype) = self {
            (body, ctype)
        } else {
            return ApiError::new_with_status(StatusCode::NOT_FOUND, "No such file.")
                .into_response();
        };
        Response::builder()
            .header(
                CACHE_CONTROL,
                "public, immutable, max-age=3600, stale-while-revalidate=86400, stale-if-error=86400",
            )
            .header(ETAG, crc32_etag(body))
            .header(CONTENT_TYPE, ctype)
            .status(StatusCode::OK)
            .body(body.into()).unwrap()
    }
}

fn get_static_file_from(d: &'static Dir, path: &str, ext: &str) -> Static {
    d.get_file(path)
        .map(|file| Static::Content(file.contents(), get_mime(ext)))
        .unwrap_or(Static::NotFound)
}

async fn index_page() -> Static {
    get_static_file_from(&CLIENT_DIR, "root/index.html", "html")
}

async fn favicon() -> Static {
    get_static_file_from(&CLIENT_DIR, "root/favicon.ico", "ico")
}

async fn const_js_modules(Path(const_js): Path<String>) -> Static {
    let body = match const_js.as_str() {
        "colors.js" => COLORS_JS.as_ref(),
        "sizes.js" => SIZ_JS,
        _ => return Static::NotFound,
    };
    Static::Content(body, "application/javascript")
}

async fn err_js_script() -> Static {
    Static::Content(ERR_JS, "application/javascript")
}

async fn static_content(req: Request) -> Static {
    let loc = req.uri().path();
    let Some(loc) = loc.strip_prefix("/public/") else {
        return Static::NotFound;
    };
    let locp = PathBuf::from(loc);
    let ext = locp.extension().unwrap_or_default().to_string_lossy();
    get_static_file_from(&CLIENT_DIR, loc, ext.as_ref())
}

pub fn routes() -> Router<MooseWebData> {
    Router::new()
        .route("/favicon.ico", get(favicon))
        .route("/", get(index_page))
        .route("/index.html", get(index_page))
        .route("/public/global-modules/err.js", get(err_js_script))
        .route("/public/const/{const}", get(const_js_modules))
        .fallback(static_content)
}
