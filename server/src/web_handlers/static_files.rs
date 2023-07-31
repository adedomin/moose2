use super::if_none_match;
use crate::{
    config::get_config,
    model::mime,
    shared_data::{COLORS_JS, SIZ_JS},
};
use actix_files::NamedFile;
use actix_web::{
    body::BoxBody,
    get,
    http::{
        header::{CacheControl, CacheDirective, ETag, EntityTag},
        StatusCode,
    },
    web, HttpRequest, HttpResponse, Responder,
};
use include_dir::{include_dir, Dir};
use std::io;

const CLIENT_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/../client/js");
const WASM_JUNK: Dir = include_dir!("$OUT_DIR/client_subbuild/wasm-bindgen");

pub enum Static {
    Body(&'static [u8], &'static str),
    NotFound,
}

pub struct StaticResp(pub Static);

impl Responder for StaticResp {
    type Body = BoxBody;

    fn respond_to(self, req: &HttpRequest) -> HttpResponse<Self::Body> {
        let (body, ctype) = if let Static::Body(body, ctype) = self.0 {
            (body, ctype)
        } else {
            return HttpResponse::Ok()
                .status(StatusCode::NOT_FOUND)
                .body("No such file or directory.");
        };

        let (etag_match, crc32) = if_none_match(body, req);
        let etag_head = ETag(EntityTag::new_strong(crc32.to_string()));
        let ctype_head = ("Content-Type", ctype);

        if etag_match {
            HttpResponse::Ok()
                .insert_header(etag_head)
                .insert_header(ctype_head)
                .insert_header(CacheControl(vec![
                    CacheDirective::Public,
                    CacheDirective::MaxAge(3600),
                ]))
                .status(StatusCode::NOT_MODIFIED)
                .body(())
        } else {
            HttpResponse::Ok()
                .insert_header(etag_head)
                .insert_header(ctype_head)
                .insert_header(CacheControl(vec![
                    CacheDirective::Public,
                    CacheDirective::MaxAge(3600),
                ]))
                .body(body)
        }
    }
}

fn get_static_file_from(d: &'static Dir, file: &str, ext: &str) -> Static {
    d.get_file(format!("{file}.{ext}"))
        .map(|file| {
            Static::Body(
                file.contents(),
                *mime::MIME.get(ext).unwrap_or(&"application/octet-string"),
            )
        })
        .unwrap_or(Static::NotFound)
}

#[get("/")]
pub async fn index_page() -> StaticResp {
    StaticResp(get_static_file_from(&CLIENT_DIR, "root/index", "html"))
}

#[get("/wasm_test")]
pub async fn wasm_test_page() -> StaticResp {
    StaticResp(get_static_file_from(&CLIENT_DIR, "root/wasm_test", "html"))
}

#[get("/favicon.ico")]
pub async fn favicon() -> StaticResp {
    StaticResp(get_static_file_from(&CLIENT_DIR, "root/favicon", "ico"))
}

#[get("/gallery/public/{file}.{ext}")]
pub async fn static_gallery_file(file: web::Path<(String, String)>) -> StaticResp {
    let gallery_fname = format!("gallery/{}", file.0.as_str());
    let gallery_body = get_static_file_from(&CLIENT_DIR, gallery_fname.as_str(), file.1.as_str());
    StaticResp(gallery_body)
}

#[get("/public/const/{const}.js")]
pub async fn const_js_modules(c: web::Path<String>) -> StaticResp {
    let body = match c.into_inner().as_str() {
        "colors" => COLORS_JS.as_ref(),
        "sizes" => SIZ_JS.as_ref(),
        _ => return StaticResp(Static::NotFound),
    };
    StaticResp(Static::Body(body, "application/javascript"))
}

#[get("/wasm_test/{file}.{ext}")]
pub async fn static_wasm_file(file: web::Path<(String, String)>) -> StaticResp {
    let fname = format!("{}", file.0.as_str());
    let body = get_static_file_from(&WASM_JUNK, fname.as_str(), file.1.as_str());
    StaticResp(body)
}

#[get("/dump")]
pub async fn db_dump() -> io::Result<NamedFile> {
    NamedFile::open_async(get_config().get_moose_path()).await
}
