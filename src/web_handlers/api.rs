use crate::{
    moosedb::{Moose, MooseDb},
    render::{moose_irc, moose_png, moose_term},
};
use actix_web::{
    body::BoxBody,
    get,
    http::{
        header::{CacheControl, CacheDirective, LOCATION},
        StatusCode,
    },
    web, HttpResponse, Responder,
};
use percent_encoding::{percent_encode, NON_ALPHANUMERIC};
use rand::Rng;
use serde::{Deserialize, Deserializer, Serialize};
use std::sync::{RwLock, RwLockReadGuard};

type MooseWebDb = web::Data<RwLock<MooseDb>>;

pub enum ApiResp {
    Body(Vec<u8>, &'static str),
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

const RANDOM: &str = "random";
const LATEST: &str = "latest";
fn simple_get<'m>(
    db_locked: &'m RwLockReadGuard<MooseDb>,
    name: &str,
) -> Result<Option<&'m Moose>, String> {
    if db_locked.meese.is_empty() {
        return Ok(None);
    }

    if name == RANDOM {
        let rand_idx = rand::thread_rng().gen_range(0..db_locked.meese.len());
        Err(percent_encode(db_locked.meese[rand_idx].name.as_bytes(), NON_ALPHANUMERIC).to_string())
    } else if name == LATEST {
        Err(percent_encode(
            db_locked.meese.last().unwrap().name.as_bytes(),
            NON_ALPHANUMERIC,
        )
        .to_string())
    } else {
        Ok(db_locked.get(name))
    }
}

pub fn get_all_moose_types<'m>(
    db: &'m RwLockReadGuard<MooseDb>,
    moose_name: &str,
    func: fn(&'m Moose) -> ApiResp,
) -> ApiResp {
    match simple_get(db, moose_name) {
        Ok(Some(moose)) => func(moose),
        Ok(None) => ApiResp::NotFound(moose_name.to_string()),
        Err(redir) => ApiResp::Redirect(redir),
    }
}

#[get("/moose/{moose_name}")]
pub async fn get_moose(db: MooseWebDb, moose_name: web::Path<String>) -> ApiResp {
    let db = db.read().unwrap();
    let moose_name = moose_name.into_inner();
    get_all_moose_types(&db, &moose_name, |moose| {
        ApiResp::Body(moose.into(), "application/json")
    })
}

#[get("/img/{moose_name}")]
pub async fn get_moose_img(db: MooseWebDb, moose_name: web::Path<String>) -> ApiResp {
    let db = db.read().unwrap();
    let moose_name = moose_name.into_inner();
    get_all_moose_types(&db, &moose_name, |moose| {
        ApiResp::Body(moose_png(moose).unwrap(), "image/png")
    })
}

#[get("/irc/{moose_name}")]
pub async fn get_moose_irc(db: MooseWebDb, moose_name: web::Path<String>) -> ApiResp {
    let db = db.read().unwrap();
    let moose_name = moose_name.into_inner();
    get_all_moose_types(&db, &moose_name, |moose| {
        ApiResp::Body(moose_irc(moose), "text/irc-art")
    })
}

#[get("/term/{moose_name}")]
pub async fn get_moose_term(db: MooseWebDb, moose_name: web::Path<String>) -> ApiResp {
    let db = db.read().unwrap();
    let moose_name = moose_name.into_inner();
    get_all_moose_types(&db, &moose_name, |moose| {
        ApiResp::Body(moose_term(moose), "text/ansi-truecolor")
    })
}

#[get("/page")]
pub async fn get_page_count(db: MooseWebDb) -> HttpResponse {
    let db = db.read().unwrap();
    let count = db.page_count();
    HttpResponse::Ok()
        .insert_header(("Content-Type", "application/json"))
        .json(count)
}

#[get("/page/{page_num}")]
pub async fn get_page(db: MooseWebDb, page_id: web::Path<usize>) -> HttpResponse {
    let db = db.read().unwrap();
    let meese: Vec<u8> = db.get_page(page_id.into_inner()).into();
    HttpResponse::Ok()
        .insert_header(("Content-Type", "application/json"))
        .body(meese)
}

#[derive(Deserialize)]
pub struct SearchQuery {
    #[serde(alias = "q", deserialize_with = "from_qstring")]
    pub query: String,
}

fn from_qstring<'de, D: Deserializer<'de>>(deserializer: D) -> Result<String, D::Error> {
    String::deserialize(deserializer).and_then(|q| {
        if q.is_empty() {
            Err(serde::de::Error::custom("query is empty"))
        } else if q.len() > 50 {
            Err(serde::de::Error::custom("query too large"))
        } else {
            Ok(q)
        }
    })
}

#[get("/search")]
pub async fn get_search_res(db: MooseWebDb, query: web::Query<SearchQuery>) -> HttpResponse {
    let db = db.read().unwrap();
    let meese: Vec<u8> = db.find_page_with_link(&query.query).into();
    HttpResponse::Ok()
        .insert_header(("Content-Type", "application/json"))
        .body(meese)
}
