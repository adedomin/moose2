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

use super::MooseWebData;
use crate::{
    db::{MooseDB, Pool, QueryError},
    model::{
        author::Author, dimensions::Dimensions, moose::Moose, pages::MooseSearchPage,
        queries::SearchQuery, PAGE_SIZE,
    },
    render::{moose_irc, moose_png, moose_term},
    task::notify_new,
    templates,
    web_handlers::JSON_TYPE,
};
use ::time::OffsetDateTime;
use actix_session::Session;
use actix_web::{
    body::BoxBody,
    get,
    http::{
        header::{CacheControl, CacheDirective, LOCATION},
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
use std::{
    sync::atomic::{AtomicU64, Ordering::Relaxed},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

/// TODO: Use a proper rate limiter
static LIMITER: AtomicU64 = AtomicU64::new(0);

pub enum HeadType {
    Found,
    NotFound,
    UnknownError,
}

pub enum ApiResp {
    Body(Vec<u8>, &'static str),
    BodyCacheTime(Vec<u8>, &'static str, time::Duration),
    Redirect(String),
    NotFound(String),
    CustomError(StatusCode, ApiError),
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

    fn new_ok(msg: String) -> Self {
        ApiError { status: "ok", msg }
    }
}

impl Responder for ApiResp {
    type Body = BoxBody;

    fn respond_to(
        self,
        _req: &actix_web::HttpRequest,
    ) -> HttpResponse<<ApiResp as Responder>::Body> {
        match self {
            ApiResp::Body(body, ctype) => HttpResponse::Ok()
                .insert_header(CacheControl(vec![
                    CacheDirective::MaxAge(3600),
                    CacheDirective::Extension("stale-if-error".to_owned(), Some("3600".to_owned())),
                ]))
                .insert_header(("Content-Type", ctype))
                .body(body),
            ApiResp::BodyCacheTime(body, ctype, duration) => HttpResponse::Ok()
                .insert_header(CacheControl(vec![
                    CacheDirective::MaxAge(duration.as_secs() as u32),
                    CacheDirective::Extension("stale-if-error".to_owned(), Some("3600".to_owned())),
                ]))
                .insert_header(("Content-Type", ctype))
                .body(body),
            ApiResp::Redirect(path) => HttpResponse::Ok()
                .status(StatusCode::SEE_OTHER)
                .insert_header((LOCATION, path))
                .body(()),
            ApiResp::NotFound(moose_name) => HttpResponse::Ok()
                .status(StatusCode::NOT_FOUND)
                .json(ApiError::new(format!("no such moose: {}", moose_name))),
            ApiResp::CustomError(code, err) => HttpResponse::Ok().status(code).json(err),
        }
    }
}

const RANDOM: &str = "random";
const LATEST: &str = "latest";
const OLDEST: &str = "oldest";

// NOTE: For the special meese names (see constants above).
//       If the DB is non-empty, they should always return something.
fn special_moose(moose: Result<Option<Moose>, QueryError>) -> Result<Option<Moose>, String> {
    match moose {
        Ok(Some(moose)) => Err(percent_encode(moose.name.as_bytes(), NON_ALPHANUMERIC).to_string()),
        Ok(None) => unreachable!(),
        Err(e) => {
            panic!("DB is broken (trying to get random): {e}");
        }
    }
}

async fn simple_get(db: &Pool, name: &str) -> Result<Option<Moose>, String> {
    let len = db.len().await.expect("Could not get length of database.");
    if len == 0 {
        return Ok(None);
    }

    if name == RANDOM {
        let rand_idx = rand::thread_rng().r#gen_range(0..len);
        special_moose(db.get_moose_idx(rand_idx).await)
    } else if name == LATEST {
        special_moose(db.get_moose_idx(len - 1).await)
    } else if name == OLDEST {
        special_moose(db.get_moose_idx(0).await)
    } else {
        match db.get_moose(name).await {
            Ok(moose) => Ok(moose),
            Err(e) => {
                panic!("DB is broken (trying to get moose {name}): {e}");
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

#[get("/api-helper/resolve/{moose_name}")]
pub async fn resolve_moose(db: MooseWebData, moose_name: web::Path<String>) -> ApiResp {
    let db = &db.db;
    let moose_name = moose_name.into_inner();
    match simple_get(db, &moose_name).await {
        Ok(Some(moose)) => ApiResp::CustomError(
            StatusCode::OK,
            ApiError::new_ok(percent_encode(moose.name.as_bytes(), NON_ALPHANUMERIC).to_string()),
        ),
        Ok(None) => ApiResp::NotFound(moose_name),
        Err(redir) => ApiResp::CustomError(StatusCode::OK, ApiError::new_ok(redir)),
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
        eprintln!("WARN: [WEB/PAGE/COUNT] {err}");
        0
    });
    // response too small to make caching worth it.
    HttpResponse::Ok().insert_header(JSON_TYPE).json(count)
}

#[get("/page/{page_num}")]
pub async fn get_page(db: MooseWebData, page_id: web::Path<usize>) -> ApiResp {
    let db = &db.db;
    let pagenum = page_id.into_inner();
    let meese = db.get_moose_page(pagenum).await.unwrap_or_else(|err| {
        eprintln!("WARN: [WEB/PAGE/{pagenum}] {err}");
        vec![]
    });
    // if the page is full, it probably won't change in hours, if ever.
    // if the page isn't full, it's the last page or a page we haven't gotten to yet and can change.
    let cache_duration = if meese.len() < PAGE_SIZE {
        Duration::from_secs(30) // last page or non-existent page.
    } else {
        Duration::from_secs(3600) // full page
    };

    let meese = serde_json::to_vec(&meese).unwrap();
    ApiResp::BodyCacheTime(meese, "application/json", cache_duration)
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
        eprintln!("WARN: [WEB/SEARCH] {err}");
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

/// Global Moose rate-limiter, prevents a new moose every minute.
/// TODO: make this session based.
fn check_ratelimit() -> Result<(), HttpResponse> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    // nothing depends on stores to the value of LIMITER other than here.
    if let Err(old) = LIMITER.fetch_update(Relaxed, Relaxed, |time| {
        if now - time > 60 {
            Some(now)
        } else {
            None
        }
    }) {
        let retry_after = format!("{}", 60 - (now - old));
        Err(HttpResponse::TooManyRequests()
            .insert_header(("Retry-After", retry_after.as_str()))
            .json(ApiError {
                status: "error",
                msg: format!("Retry again in {retry_after}"),
            }))
    } else {
        Ok(())
    }
}

#[post("/new")]
pub async fn put_new_moose(
    webdata: MooseWebData,
    session: Session,
    mut payload: Payload,
) -> HttpResponse {
    if let Err(e) = check_ratelimit() {
        return e;
    }

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
            return HttpResponse::BadRequest()
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

    let db = webdata.db.clone();
    let moose_name = moose.name.clone();
    if let Err(e) = db.insert_moose(moose).await {
        HttpResponse::UnprocessableEntity()
            .insert_header(JSON_TYPE)
            .json(ApiError {
                status: "error",
                msg: e.to_string(),
            })
    } else {
        notify_new();
        HttpResponse::Ok().insert_header(JSON_TYPE).json(ApiError {
            status: "ok",
            msg: format!("moose {moose_name} saved."),
        })
    }
}

#[cfg(not(feature = "serve_static"))]
const DUMP_PROD_MSG: &str = r###"
Release moose does not implement read and serving file-system content.
You are expected to use a Reverse Proxy to host moose2 over the internet.

To serve the /dump file, Please see the example nginx snippet:

```nginx.conf
location = /dump {
    # moose2 dumps new moose every 5 minutes.
    add_header Cache-Control "max-age=300, public, stale-if-error"
    default_type "application/json";
    alias /var/lib/moose2/dump.json;
}
```
"###;

#[cfg(not(feature = "serve_static"))]
#[get("/dump")]
pub async fn get_dump() -> impl Responder {
    HttpResponse::Ok()
        .insert_header(("Content-Type", "text/plain; charset=utf8"))
        .body(DUMP_PROD_MSG)
}

#[cfg(feature = "serve_static")]
#[get("/dump")]
pub async fn get_dump(data: MooseWebData) -> impl Responder {
    let dump = data.moose_dump.clone();
    actix_files::NamedFile::open_async(dump).await
}
