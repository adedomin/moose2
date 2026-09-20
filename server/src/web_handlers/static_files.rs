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

use super::{ApiError, MooseWebData};
use crate::shared_data::{COLORS_JS, SIZ_JS};
use axum::{
    Router,
    extract::Request,
    response::{IntoResponse, Response},
    routing::get,
};
use http::{
    StatusCode,
    header::{CACHE_CONTROL, CONTENT_TYPE},
};
use include_static::{StaticContent, StaticContents, find_static_by_path, include_static};

const CLIENT_DIR: StaticContents = include_static!("../client/src");

enum Static {
    Content(&'static [u8], &'static str),
    NotFound,
}

impl IntoResponse for Static {
    fn into_response(self) -> Response {
        let Static::Content(body, ctype) = self else {
            return ApiError::new_with_status(StatusCode::NOT_FOUND, "No such file.")
                .into_response();
        };
        Response::builder()
            .header(
                CACHE_CONTROL,
                "public, immutable, max-age=86400, stale-while-revalidate=1209600, stale-if-error=1209600",
            )
            .header(CONTENT_TYPE, ctype)
            .status(StatusCode::OK).body(body.into()).unwrap()
    }
}

const fn get_static_file_from(find: &str) -> Static {
    if let Some(StaticContent { content, mime, .. }) = find_static_by_path(CLIENT_DIR, find) {
        Static::Content(content, mime)
    } else {
        Static::NotFound
    }
}

const FAVICON: Static = get_static_file_from("root/favicon.ico");
async fn favicon() -> Static {
    FAVICON
}

const COLORS_JS_RESP: Static = Static::Content(&COLORS_JS, "application/javascript");
async fn colors_js() -> Static {
    COLORS_JS_RESP
}

const SIZ_JS_RESP: Static = Static::Content(SIZ_JS, "application/javascript");
async fn sizes_js() -> Static {
    SIZ_JS_RESP
}

async fn static_content(req: Request) -> Static {
    let loc = req.uri().path();
    let Some(loc) = loc.strip_prefix("/public/") else {
        return Static::NotFound;
    };
    get_static_file_from(loc)
}

pub fn routes() -> Router<MooseWebData> {
    Router::new()
        .route("/favicon.ico", get(favicon))
        .route("/public/const/colors.js", get(colors_js))
        .route("/public/const/sizes.js", get(sizes_js))
        .fallback(static_content)
}
