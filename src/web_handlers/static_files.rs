use actix_web::{get, web, Responder, body::BoxBody, HttpRequest, HttpResponse, http::{StatusCode, header::{EntityTag, ETag}}};
use super::if_none_match;

const APP_CSS: &[u8] = include_bytes!("../../public/moose2.css");
const APP_ICON: &[u8] = include_bytes!("../../public/favicon.ico");
const APP_JS: &[u8] = include_bytes!("../../public/moose2.js");

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
                .status(StatusCode::NOT_MODIFIED)
                .body(())
        } else {
            HttpResponse::Ok()
                .insert_header(etag_head)
                .insert_header(ctype_head)
                .body(body)
        }
    }
}

#[get("/favicon.ico")]
pub async fn favicon() -> StaticResp {
    StaticResp(Static::Body(APP_ICON, "image/x-icon"))
}

#[get("/public/moose2.{file_ext}")]
pub async fn static_file(t: web::Path<String>) -> StaticResp {
    let (body, ctype) = match t.into_inner().as_str() {
        "css" => (APP_CSS, "text/css"),
        "js" => (APP_JS, "application/javascript"),
        _ => return StaticResp(Static::NotFound),
    };
    StaticResp(Static::Body(body, ctype))
}
