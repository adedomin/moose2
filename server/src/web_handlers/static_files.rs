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

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::OnceLock,
};

use super::{ApiError, MooseWebData};
use crate::{
    middleware::etag::etag,
    model::mime::get_mime,
    shared_data::{COLORS_JS, ERR_JS, SIZ_JS},
};
use axum::{
    Router,
    extract::{Path as AxumPath, Request},
    response::{IntoResponse, Response},
    routing::get,
};
use http::{
    StatusCode,
    header::{CACHE_CONTROL, CONTENT_TYPE, ETAG},
};
use include_dir::{Dir, include_dir};

const CLIENT_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/../client/src");
static CLIENT_ETAGS: OnceLock<HashMap<&'static Path, String>> = OnceLock::new();
const COLORS_JS_PATH: &str = "\0COLORS_JS";
const ERR_JS_PATH: &str = "\0ERR_JS";
const SIZ_JS_PATH: &str = "\0SIZ_JS";

fn get_static_etag<T: AsRef<Path> + std::fmt::Debug>(p: T) -> &'static str {
    let Some(etag) = CLIENT_ETAGS
        .get_or_init(|| {
            let mut map = HashMap::new();
            map.insert(Path::new(COLORS_JS_PATH), etag(COLORS_JS));
            map.insert(Path::new(ERR_JS_PATH), etag(ERR_JS));
            map.insert(Path::new(SIZ_JS_PATH), etag(ERR_JS));

            let mut stack = vec![&CLIENT_DIR];
            while let Some(dir) = stack.pop() {
                dir.files().for_each(|f| {
                    let body = f.contents();
                    let etag = etag(body);
                    map.insert(f.path(), etag);
                });
                stack.extend(dir.dirs());
            }
            map
        })
        .get(p.as_ref())
    else {
        log::error!("Path {p:?} is missing from CLIENT_ETAGS; FIX IT");
        return "W/\"JUNKETAG\"";
    };
    etag
}

enum Static {
    Content(&'static str, &'static [u8], &'static str),
    NotFound,
}

impl IntoResponse for Static {
    fn into_response(self) -> Response {
        let Static::Content(etag, body, ctype) = self else {
            return ApiError::new_with_status(StatusCode::NOT_FOUND, "No such file.")
                .into_response();
        };
        Response::builder()
            .header(
                CACHE_CONTROL,
                "public, immutable, max-age=3600, stale-while-revalidate=86400, stale-if-error=86400",
            )
            .header(ETAG, etag)
            .header(CONTENT_TYPE, ctype)
            .status(StatusCode::OK)
            .body(body.into()).unwrap()
    }
}

fn get_static_file_from(d: &'static Dir, path: &str, ext: &str) -> Static {
    d.get_file(path)
        .map(|file| {
            let etag = get_static_etag(file.path());
            Static::Content(etag, file.contents(), get_mime(ext))
        })
        .unwrap_or(Static::NotFound)
}

async fn index_page() -> Static {
    get_static_file_from(&CLIENT_DIR, "root/index.html", "html")
}

async fn favicon() -> Static {
    get_static_file_from(&CLIENT_DIR, "root/favicon.ico", "ico")
}

async fn const_js_modules(AxumPath(const_js): AxumPath<String>) -> Static {
    let (path, body) = match const_js.as_str() {
        "colors.js" => (COLORS_JS_PATH, COLORS_JS.as_ref()),
        "sizes.js" => (SIZ_JS_PATH, SIZ_JS),
        _ => return Static::NotFound,
    };
    Static::Content(get_static_etag(path), body, "application/javascript")
}

async fn err_js_script() -> Static {
    Static::Content(
        get_static_etag(ERR_JS_PATH),
        ERR_JS,
        "application/javascript",
    )
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
