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

use super::{ApiError, HOUR_CACHE, MooseWebData};
use crate::{
    db::{
        MooseDB,
        sqlite3_impl::{Pool, Sqlite3Error},
    },
    model::{
        PAGE_SIZE, author::Author, dimensions::Dimensions, moose::Moose, pages::MooseSearchPage,
        queries::SearchQuery,
    },
    render::{moose_irc, moose_png, moose_term},
    task::notify_new,
    templates,
    web_handlers::JSON_TYPE,
};
use ::time::OffsetDateTime;
use axum::{
    Json, Router,
    extract::{Path, Query, State, rejection::JsonRejection},
    response::{IntoResponse, Response},
    routing::{get, put},
};
use core::time;
use http::{
    StatusCode, Uri,
    header::{CACHE_CONTROL, CONTENT_TYPE, LOCATION},
};
use percent_encoding::{NON_ALPHANUMERIC, percent_encode};
use std::time::Duration;

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
    CustomError(ApiError),
}

impl IntoResponse for ApiResp {
    fn into_response(self) -> axum::response::Response {
        match self {
            ApiResp::Body(body, ctype) => Response::builder()
                .header(CACHE_CONTROL, HOUR_CACHE)
                .header(CONTENT_TYPE, ctype)
                .body(body.into())
                .unwrap(),
            ApiResp::BodyCacheTime(body, ctype, duration) => Response::builder()
                .header(
                    CACHE_CONTROL,
                    format!("max-age={}, stale-if-error=3600", duration.as_secs()),
                )
                .header(CONTENT_TYPE, ctype)
                .body(body.into())
                .unwrap(),
            ApiResp::Redirect(path) => Response::builder()
                .status(StatusCode::SEE_OTHER)
                .header(LOCATION, path)
                .body(().into())
                .unwrap(),
            ApiResp::NotFound(moose_name) => Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(
                    ApiError::new(format!("no such moose: {}", moose_name))
                        .to_json()
                        .into(),
                )
                .unwrap(),
            ApiResp::CustomError(err) => err.into_response(),
        }
    }
}

const RANDOM: &str = "random";
const LATEST: &str = "latest";
const OLDEST: &str = "oldest";

fn special_moose(moose: Result<Option<Moose>, Sqlite3Error>) -> Result<Option<Moose>, String> {
    match moose {
        Ok(Some(moose)) => Err(percent_encode(moose.name.as_bytes(), NON_ALPHANUMERIC).to_string()),
        Ok(None) => Ok(None),
        Err(e) => {
            panic!("DB is broken (trying to get random): {e}");
        }
    }
}

async fn simple_get(db: &Pool, name: &str) -> Result<Option<Moose>, String> {
    if name == RANDOM {
        special_moose(db.random().await)
    } else if name == LATEST {
        special_moose(db.latest().await)
    } else if name == OLDEST {
        special_moose(db.oldest().await)
    } else {
        match db.get_moose(name).await {
            Ok(moose) => Ok(moose),
            Err(e) => {
                panic!("DB is broken (trying to get moose {name}): {e}");
            }
        }
    }
}

async fn resolve_moose(State(db): State<MooseWebData>, Path(moose_name): Path<String>) -> ApiResp {
    let db = &db.db;
    match simple_get(db, &moose_name).await {
        Ok(Some(moose)) => ApiResp::CustomError(ApiError::new_ok(
            percent_encode(moose.name.as_bytes(), NON_ALPHANUMERIC).to_string(),
        )),
        Ok(None) => ApiResp::NotFound(moose_name),
        Err(redir) => ApiResp::CustomError(ApiError::new_ok(redir)),
    }
}

async fn get_moose(
    State(db): State<MooseWebData>,
    Path(moose_name): Path<String>,
    uri: Uri,
) -> ApiResp {
    let db = &db.db;
    let Some(path) = uri.path().split('/').nth(1) else {
        log::error!("Path seems wrong for some call: {:?}", uri.path());
        return ApiResp::CustomError(ApiError::new(
            "Path seems to be missing components; how did you get here?".to_owned(),
        ));
    };
    match simple_get(db, &moose_name).await {
        Ok(Some(moose)) => {
            let (body, ctype) = match path {
                "moose" => (moose.into(), "application/json"),
                "img" => (moose_png(&moose).unwrap(), "image/png"),
                "irc" => (moose_irc(&moose), "text/irc-art"),
                "term" => (moose_term(&moose), "text/ansi-truecolor"),
                _ => {
                    log::error!("Router is passing paths that don't make sense: {path:?}",);
                    return ApiResp::CustomError(ApiError::new(
                        "Cannot fetch moose type: {path:?}".to_owned(),
                    ));
                }
            };
            ApiResp::Body(body, ctype)
        }
        Ok(None) => ApiResp::NotFound(moose_name.to_string()),
        Err(redir) => ApiResp::Redirect(redir),
    }
}

async fn get_page_count(State(db): State<MooseWebData>) -> Response {
    let db = &db.db;
    let count = db.get_page_count().await.unwrap_or_else(|err| {
        log::error!("{err}");
        0
    });
    let count = serde_json::to_vec(&count).unwrap();
    // response too small to make caching worth it.
    Response::builder()
        .status(StatusCode::OK)
        .header(JSON_TYPE.0, JSON_TYPE.1)
        .body(count.into())
        .unwrap()
}

// #[get("/page/{page_num}")]
async fn get_page(State(db): State<MooseWebData>, Path(page_num): Path<usize>) -> ApiResp {
    let db = &db.db;
    let meese = db.get_moose_page(page_num).await.unwrap_or_else(|err| {
        log::error!("{err}");
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

async fn get_page_nav_range(
    State(db): State<MooseWebData>,
    Path(page_num): Path<usize>,
) -> Response {
    let db = &db.db;
    let meese = templates::page_range(page_num, db.get_page_count().await.unwrap_or(page_num));
    let meese = serde_json::to_vec(&meese.collect::<Vec<usize>>()).unwrap();
    // response too small to make caching worth it.
    Response::builder()
        .status(StatusCode::OK)
        .header(JSON_TYPE.0, JSON_TYPE.1)
        .body(meese.into())
        .unwrap()
}

// #[get("/search")]
async fn get_search_page(
    State(db): State<MooseWebData>,
    Query(SearchQuery { query, page, .. }): Query<SearchQuery>,
) -> ApiResp {
    let db = &db.db;
    let meese = db.search_moose(&query, page).await.unwrap_or_else(|err| {
        log::warn!("{err}");
        MooseSearchPage::default()
    });
    let meese = serde_json::to_vec(&meese).unwrap();
    ApiResp::BodyCacheTime(meese, "application/json", Duration::from_secs(300))
}

pub const MAX_BODY_SIZE: usize = 2usize.pow(14);

async fn put_new_moose(
    State(webdata): State<MooseWebData>,
    session_author: Author,
    payload: Result<Json<Moose>, JsonRejection>,
) -> ApiError {
    let Json(mut moose) = match payload {
        Ok(moose) => moose,
        Err(e) => {
            return ApiError::new_with_status(StatusCode::BAD_REQUEST, e);
        }
    };

    moose.author = session_author;

    // Ignore these user fields by replacing them with defaults.
    moose.created = OffsetDateTime::now_utc();
    moose.upvotes = 0;

    if let Dimensions::Custom(_, _) = moose.dimensions {
        return ApiError::new_with_status(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Custom dimensions are not allowed through the public API.",
        );
    }

    let db = webdata.db.clone();
    let moose_name = moose.name.clone();
    if let Err(e) = db.insert_moose(moose).await {
        if let Sqlite3Error::Sqlite3(rusqlite::Error::SqliteFailure(e, _)) = e {
            if matches!(e.code, rusqlite::ErrorCode::ConstraintViolation) {
                return ApiError::new_with_status(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    format!("{moose_name} already exists."),
                );
            }
        }
        ApiError::new_with_status(StatusCode::UNPROCESSABLE_ENTITY, e)
    } else {
        notify_new();
        log::debug!("New moose: {moose_name}");
        ApiError::new_ok(format!("Saved {moose_name}."))
    }
}

#[cfg(not(feature = "serve-static"))]
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

#[cfg(not(feature = "serve-static"))]
async fn get_dump() -> Response {
    Response::builder()
        .status(http::StatusCode::OK)
        .header(CONTENT_TYPE, "text/plain; charset=utf8")
        .body(DUMP_PROD_MSG.into())
        .unwrap()
}

pub fn routes() -> Router<MooseWebData> {
    Router::new()
        .route("/api-helper/resolve/{moose_name}", get(resolve_moose))
        .route("/moose/{moose_name}", get(get_moose))
        .route("/img/{moose_name}", get(get_moose))
        .route("/irc/{moose_name}", get(get_moose))
        .route("/term/{moose_name}", get(get_moose))
        .route("/page", get(get_page_count))
        .route("/page/{page_num}", get(get_page))
        .route("/nav/{page_num}", get(get_page_nav_range))
        .route("/search", get(get_search_page))
        .route("/new", put(put_new_moose).post(put_new_moose))
}

pub fn dump_route<T: AsRef<std::path::Path>>(_dump_path: T) -> Router<MooseWebData> {
    let r = Router::new();
    #[cfg(feature = "serve-static")]
    // NOTE: 256KiB was chosen based on performance testing
    //
    // `ab -k -n 1000 -c 8 http://localhost:5921/dump`
    // -----------------------------------------------
    //  64KiB:   ~225 req/sec  ServeFile default buffer size.
    // 128KiB:  ~1500 req/sec
    // 256KiB:  ~1600 req/sec  Current GNU Coreutils read(..., BUFSIZ) default.
    // 512KiB:  ~1300 req/sec
    //   1MiB:  ~1000 req/sec
    let r = r.route_service(
        "/dump",
        tower_http::services::ServeFile::new(_dump_path).with_buf_chunk_size(256 * 1024),
    );
    #[cfg(not(feature = "serve-static"))]
    let r = r.route("/dump", get(get_dump));
    r
}
