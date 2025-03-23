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

use super::{api::ApiError, if_none_match};
use crate::{
    model::mime::get_mime,
    shared_data::{COLORS_JS, ERR_JS, SIZ_JS},
};
use actix_web::{
    HttpRequest, HttpResponse, Responder,
    body::BoxBody,
    get,
    http::{
        StatusCode,
        header::{CacheControl, CacheDirective, ETag, EntityTag},
    },
    web,
};
use include_dir::{Dir, include_dir};

const CLIENT_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/../client/src");

pub enum Static {
    Content(&'static [u8], &'static str),
    NotFound,
}

impl Responder for Static {
    type Body = BoxBody;

    fn respond_to(self, req: &HttpRequest) -> HttpResponse<Self::Body> {
        let (body, ctype) = if let Static::Content(body, ctype) = self {
            (body, ctype)
        } else {
            return HttpResponse::Ok()
                .status(StatusCode::NOT_FOUND)
                .json(ApiError::new("No such file.".to_string()));
        };

        let (etag_match, crc32) = if_none_match(body, req);
        let etag_head = ETag(EntityTag::new_strong(crc32.to_string()));
        let ctype_head = ("Content-Type", ctype);

        let mut res_build = HttpResponse::Ok();
        let res_build = res_build
            .insert_header(etag_head)
            .insert_header(ctype_head)
            .insert_header(CacheControl(vec![
                CacheDirective::Public,
                CacheDirective::Extension("immutable".to_owned(), None),
                CacheDirective::MaxAge(3600),
                CacheDirective::Extension(
                    "stale-while-revalidate".to_owned(),
                    Some("86400".to_owned()),
                ),
                CacheDirective::Extension("stale-if-error".to_owned(), Some("86400".to_owned())),
            ]));
        if etag_match {
            res_build.status(StatusCode::NOT_MODIFIED).body(())
        } else {
            res_build.body(body)
        }
    }
}

fn get_static_file_from(d: &'static Dir, path: &str, ext: &str) -> Static {
    d.get_file(path)
        .map(|file| Static::Content(file.contents(), get_mime(ext)))
        .unwrap_or(Static::NotFound)
}

#[get("/")]
async fn index_page() -> Static {
    get_static_file_from(&CLIENT_DIR, "root/index.html", "html")
}

#[get("/favicon.ico")]
async fn favicon() -> Static {
    get_static_file_from(&CLIENT_DIR, "root/favicon.ico", "ico")
}

#[get("/public/const/{const}.js")]
async fn const_js_modules(c: web::Path<String>) -> Static {
    let body = match c.into_inner().as_str() {
        "colors" => COLORS_JS.as_ref(),
        "sizes" => SIZ_JS,
        _ => return Static::NotFound,
    };
    Static::Content(body, "application/javascript")
}

#[get("/public/global-modules/err.js")]
async fn err_js_script() -> Static {
    Static::Content(ERR_JS, "application/javascript")
}

async fn static_content(req: HttpRequest) -> Static {
    let loc = req.path();
    let Some(loc) = loc.strip_prefix("/public/") else {
        return Static::NotFound;
    };
    let locp = PathBuf::from(loc);
    let ext = locp.extension().unwrap_or_default().to_string_lossy();
    get_static_file_from(&CLIENT_DIR, loc, ext.as_ref())
}

pub fn register(conf: &mut web::ServiceConfig) {
    conf.service(index_page)
        .service(favicon)
        .service(err_js_script)
        .service(const_js_modules)
        .default_service(actix_web::web::to(static_content));
}
