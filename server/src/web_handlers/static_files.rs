use super::if_none_match;
use crate::{
    model::mime,
    shared_data::{COLORS_JS, ERR_JS, SIZ_JS},
};
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

const CLIENT_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/../client/js");

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
            ]));
        if etag_match {
            res_build.status(StatusCode::NOT_MODIFIED).body(())
        } else {
            res_build.body(body)
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

#[get("/favicon.ico")]
pub async fn favicon() -> StaticResp {
    StaticResp(get_static_file_from(&CLIENT_DIR, "root/favicon", "ico"))
}

#[get("/root/public/{file}.{ext}")]
pub async fn static_root_file(file: web::Path<(String, String)>) -> StaticResp {
    let root_fname = format!("root/{}", file.0.as_str());
    let root_body = get_static_file_from(&CLIENT_DIR, root_fname.as_str(), file.1.as_str());
    StaticResp(root_body)
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

#[get("/public/gridpaint/index.js")]
pub async fn gridpaint_modules() -> StaticResp {
    let gridpaint = get_static_file_from(&CLIENT_DIR, "gridpaint/index", "js");
    StaticResp(gridpaint)
}

#[get("/public/gridpaint/lib/{module}.js")]
pub async fn gridpaint_lib_modules(gp: web::Path<String>) -> StaticResp {
    let gridpaint_fname = format!("gridpaint/lib/{}", gp.into_inner().as_str());
    let gridpaint = get_static_file_from(&CLIENT_DIR, gridpaint_fname.as_str(), "js");
    StaticResp(gridpaint)
}

#[get("/public/global-modules/err.js")]
pub async fn err_js_script() -> StaticResp {
    StaticResp(Static::Body(ERR_JS, "application/javascript"))
}
