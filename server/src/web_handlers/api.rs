use super::{if_none_match_md5, MooseWebData};
use crate::{
    db::{MooseDB, Pool},
    model::{
        author::Author, dimensions::Dimensions, moose::Moose, pages::MooseSearchPage,
        queries::SearchQuery,
    },
    render::{moose_irc, moose_png, moose_term},
    templates,
};
use ::time::OffsetDateTime;
use actix_session::Session;
use actix_web::{
    body::BoxBody,
    get,
    http::{
        header::{CacheControl, CacheDirective, ETag, EntityTag, LOCATION},
        StatusCode,
    },
    post,
    web::{self, Payload},
    HttpResponse, Responder,
};
use core::time;
use futures::stream::StreamExt;
use percent_encoding::{percent_encode, NON_ALPHANUMERIC};
use rand::Rng;
use serde::Serialize;
use std::time::Duration;

pub enum ApiResp {
    Body(Vec<u8>, &'static str),
    BodyCacheTime(Vec<u8>, &'static str, time::Duration),
    Redirect(String),
    NotFound(String),
}

#[derive(Serialize)]
pub struct ApiError {
    status: &'static str,
    msg: String,
}

impl ApiError {
    fn new(msg: String) -> Self {
        ApiError {
            status: "error",
            msg,
        }
    }
}

const JSON_TYPE: (&str, &str) = ("Content-Type", "application/json");

impl Responder for ApiResp {
    type Body = BoxBody;

    fn respond_to(
        self,
        _req: &actix_web::HttpRequest,
    ) -> HttpResponse<<ApiResp as Responder>::Body> {
        match self {
            ApiResp::Body(body, ctype) => HttpResponse::Ok()
                .insert_header(CacheControl(vec![CacheDirective::MaxAge(3600)]))
                .insert_header(("Content-Type", ctype))
                .body(body),
            ApiResp::BodyCacheTime(body, ctype, duration) => HttpResponse::Ok()
                .insert_header(CacheControl(vec![CacheDirective::MaxAge(
                    duration.as_secs() as u32,
                )]))
                .insert_header(("Content-Type", ctype))
                .body(body),
            ApiResp::Redirect(path) => HttpResponse::Ok()
                .status(StatusCode::SEE_OTHER)
                .insert_header((LOCATION, path))
                .body(()),
            ApiResp::NotFound(moose_name) => HttpResponse::Ok()
                .status(StatusCode::NOT_FOUND)
                .json(ApiError::new(format!("no such moose: {}", moose_name))),
        }
    }
}

pub enum VarBody {
    Found(Vec<u8>, &'static str),
    NotFound,
}

impl Responder for VarBody {
    type Body = BoxBody;

    fn respond_to(self, req: &actix_web::HttpRequest) -> HttpResponse<Self::Body> {
        let (body, ctype) = if let VarBody::Found(body, ctype) = self {
            (body, ctype)
        } else {
            return HttpResponse::Ok()
                .status(StatusCode::NOT_FOUND)
                .body("No such file or directory.");
        };

        let (etag_match, md5_hex) = if_none_match_md5(&body, req);
        let etag_head = ETag(EntityTag::new_strong(md5_hex));
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

const RANDOM: &str = "random";
const LATEST: &str = "latest";
async fn simple_get<'m>(db: &'m Pool, name: &str) -> Result<Option<Moose>, String> {
    if db.is_empty().await {
        return Ok(None);
    }

    if name == RANDOM {
        let rand_idx = rand::thread_rng().gen_range(0..db.len().await.unwrap());
        match db.get_moose_idx(rand_idx).await {
            Ok(Some(moose)) => {
                Err(percent_encode(moose.name.as_bytes(), NON_ALPHANUMERIC).to_string())
            }
            Ok(None) => unreachable!(),
            Err(e) => {
                panic!("DB is broken (trying to get random): {}", e);
            }
        }
    } else if name == LATEST {
        match db.last().await {
            Ok(Some(moose)) => {
                Err(percent_encode(moose.name.as_bytes(), NON_ALPHANUMERIC).to_string())
            }
            Ok(None) => unreachable!(),
            Err(e) => {
                panic!("DB is broken (trying to get latest): {}", e);
            }
        }
    } else {
        match db.get_moose(name).await {
            Ok(moose) => Ok(moose),
            Err(e) => {
                panic!("DB is broken (trying to get moose {}): {}", name, e);
            }
        }
    }
}

pub async fn get_all_moose_types(
    db: &Pool,
    moose_name: &str,
    func: fn(Moose) -> ApiResp,
) -> ApiResp {
    match simple_get(db, moose_name).await {
        Ok(Some(moose)) => func(moose),
        Ok(None) => ApiResp::NotFound(moose_name.to_string()),
        Err(redir) => ApiResp::Redirect(redir),
    }
}

#[get("/moose/{moose_name}")]
pub async fn get_moose(db: MooseWebData, moose_name: web::Path<String>) -> ApiResp {
    let db = &db.db;
    let moose_name = moose_name.into_inner();
    get_all_moose_types(db, &moose_name, |moose| {
        ApiResp::Body(moose.into(), "application/json")
    })
    .await
}

#[get("/img/{moose_name}")]
pub async fn get_moose_img(db: MooseWebData, moose_name: web::Path<String>) -> ApiResp {
    let db = &db.db;
    let moose_name = moose_name.into_inner();
    get_all_moose_types(db, &moose_name, |moose| {
        ApiResp::Body(moose_png(&moose).unwrap(), "image/png")
    })
    .await
}

#[get("/irc/{moose_name}")]
pub async fn get_moose_irc(db: MooseWebData, moose_name: web::Path<String>) -> ApiResp {
    let db = &db.db;
    let moose_name = moose_name.into_inner();
    get_all_moose_types(db, &moose_name, |moose| {
        ApiResp::Body(moose_irc(&moose), "text/irc-art")
    })
    .await
}

#[get("/term/{moose_name}")]
pub async fn get_moose_term(db: MooseWebData, moose_name: web::Path<String>) -> ApiResp {
    let db = &db.db;
    let moose_name = moose_name.into_inner();
    get_all_moose_types(db, &moose_name, |moose| {
        ApiResp::Body(moose_term(&moose), "text/ansi-truecolor")
    })
    .await
}

#[get("/page")]
pub async fn get_page_count(db: MooseWebData) -> HttpResponse {
    let db = &db.db;
    let count = db.get_page_count().await.unwrap_or_else(|err| {
        eprintln!("{}", err);
        0
    });
    // response too small to make caching worth it.
    HttpResponse::Ok().insert_header(JSON_TYPE).json(count)
}

#[get("/page/{page_num}")]
pub async fn get_page(db: MooseWebData, page_id: web::Path<usize>) -> ApiResp {
    let db = &db.db;
    let meese = db
        .get_moose_page(page_id.into_inner())
        .await
        .unwrap_or_else(|err| {
            eprintln!("{}", err);
            vec![]
        });
    let meese = serde_json::to_vec(&meese).unwrap();
    ApiResp::BodyCacheTime(meese, "application/json", Duration::from_secs(300))
}

#[get("/nav/{page_num}")]
pub async fn get_page_nav_range(db: MooseWebData, page_id: web::Path<usize>) -> HttpResponse {
    let db = &db.db;
    let page_num = page_id.into_inner();
    let meese = templates::page_range(page_num, db.get_page_count().await.unwrap_or(page_num));
    // response too small to make caching worth it.
    HttpResponse::Ok()
        .insert_header(JSON_TYPE)
        .json(meese.collect::<Vec<usize>>())
}

#[get("/search")]
pub async fn get_search_page(db: MooseWebData, query: web::Query<SearchQuery>) -> ApiResp {
    let db = &db.db;
    let SearchQuery { page, query, .. } = query.into_inner();
    let meese = db.search_moose(&query, page).await.unwrap_or_else(|err| {
        eprintln!("{}", err);
        MooseSearchPage::default()
    });
    let meese = serde_json::to_vec(&meese).unwrap();
    ApiResp::BodyCacheTime(meese, "application/json", Duration::from_secs(300))
}

pub const MAX_BODY_SIZE: usize = 2usize.pow(14);

fn moose_validation_err(msg: &str) -> HttpResponse {
    HttpResponse::BadRequest()
        .insert_header(JSON_TYPE)
        .json(ApiError {
            status: "error",
            msg: msg.to_string(),
        })
}

#[post("/new")]
pub async fn put_new_moose(
    db: MooseWebData,
    session: Session,
    mut payload: Payload,
) -> HttpResponse {
    let mut body = web::BytesMut::new();
    while let Some(chunk) = payload.next().await {
        let chunk = match chunk {
            Ok(chunk) => chunk,
            Err(e) => {
                return HttpResponse::InternalServerError()
                    .insert_header(JSON_TYPE)
                    .json(ApiError {
                        status: "error",
                        msg: e.to_string(),
                    });
            }
        };
        if body.len().saturating_add(chunk.len()) > MAX_BODY_SIZE {
            return HttpResponse::PayloadTooLarge()
                .insert_header(JSON_TYPE)
                .json(ApiError {
                    status: "error",
                    msg: "Payload too large.".to_string(),
                });
        }
        body.extend_from_slice(&chunk);
    }

    let mut moose = match serde_json::from_slice::<Moose>(&body) {
        Ok(moose) => moose,
        Err(msg) => {
            return HttpResponse::UnprocessableEntity()
                .insert_header(JSON_TYPE)
                .json(ApiError {
                    status: "error",
                    msg: msg.to_string(),
                });
        }
    };
    if let Ok(Some(author)) = session.get::<Author>("login") {
        moose.author = author;
    } else {
        moose.author = Author::Anonymous;
    }
    moose.created = OffsetDateTime::now_utc();

    if let Dimensions::Custom(_, _) = moose.dimensions {
        return moose_validation_err("Custom dimensions are not allowed through the public API.");
    }

    let db = db.db.clone();
    let moose_name = moose.name.clone();
    if let Err(e) = db.insert_moose(moose).await {
        HttpResponse::BadRequest()
            .insert_header(JSON_TYPE)
            .json(ApiError {
                status: "error",
                msg: e.to_string(),
            })
    } else {
        HttpResponse::Ok().insert_header(JSON_TYPE).json(ApiError {
            status: "ok",
            msg: format!("moose {moose_name} saved."),
        })
    }
}
