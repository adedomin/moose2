use crate::moosedb::{Moose, MooseDb};
use crate::render::{moose_irc, moose_png, moose_term};
use crate::templates::gallery;
use core::fmt;
use lazy_static::lazy_static;
use rand::Rng;
use rouille::percent_encoding::percent_decode;
use rouille::{
    percent_encoding::{percent_encode, NON_ALPHANUMERIC},
    router, Request, Response,
};
use std::borrow::Cow;
use std::str::FromStr;
use std::sync::{Arc, RwLock, RwLockReadGuard};

const RANDOM: &str = "random";
const LATEST: &str = "latest";

fn get_query_params(q: &str) -> impl Iterator<Item = (&str, &str)> {
    q.split_terminator('&')
        .map(|kv| kv.split_once('=').unwrap_or((kv, "")))
}

fn get_query_param_value<'m>(qstring: &'m str, param: &str) -> Cow<'m, str> {
    let ret = get_query_params(qstring)
        .find(|(key, _)| *key == param)
        .map(|(_, val)| val)
        .unwrap_or("");
    percent_decode(ret.as_bytes()).decode_utf8_lossy()
}

fn error_resp(code: u16, res_type: &'static str, message: String) -> Response {
    let mut e = Response::from_data(res_type, message);
    e.status_code = code;
    e
}

fn json_err_res(code: u16, message: &str) -> Response {
    error_resp(
        code,
        "application/json",
        format!(
            r#"{{"status":"error","msg":{}}}"#,
            serde_json::to_string::<str>(message).unwrap(),
        ),
    )
}

fn moose_404(moose_name: &str) -> Response {
    json_err_res(404, &format!("no such moose: {}", moose_name))
}

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

const APP_CSS: &[u8] = include_bytes!("../public/moose2.css");
const APP_ICON: &[u8] = include_bytes!("../public/favicon.ico");
const APP_JS: &[u8] = include_bytes!("../public/moose2.js");

lazy_static! {
    static ref APP_CSS_CRC32: u32 = crc32fast::hash(APP_CSS);
    static ref APP_CSS_CRC32_STR: String = APP_CSS_CRC32.to_string();
    static ref APP_ICON_CRC32: u32 = crc32fast::hash(APP_ICON);
    static ref APP_ICON_CRC32_STR: String = APP_ICON_CRC32.to_string();
    static ref APP_JS_CRC32: u32 = crc32fast::hash(APP_JS);
    static ref APP_JS_CRC32_STR: String = APP_JS_CRC32.to_string();
}

enum TypeRoutes {
    Irc,
    Term,
    Png,
    Moose,
}

impl FromStr for TypeRoutes {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "irc" => Ok(TypeRoutes::Irc),
            "term" => Ok(TypeRoutes::Term),
            "img" => Ok(TypeRoutes::Png),
            "moose" => Ok(TypeRoutes::Moose),
            _ => Err(()),
        }
    }
}

impl fmt::Display for TypeRoutes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TypeRoutes::Irc => f.write_str("irc"),
            TypeRoutes::Term => f.write_str("term"),
            TypeRoutes::Png => f.write_str("img"),
            TypeRoutes::Moose => f.write_str("moose"),
        }
    }
}

impl TypeRoutes {
    fn content_type(&self) -> &'static str {
        match self {
            TypeRoutes::Irc => "text/irc-art",
            TypeRoutes::Term => "text/ansi-truecolor",
            TypeRoutes::Png => "image/png",
            TypeRoutes::Moose => "application/json",
        }
    }
}

pub fn handler(db: Arc<RwLock<MooseDb>>, req: &Request) -> Response {
    // static paths and redirects
    if req.method() == "GET" {
        match req.url().as_str() {
            "/public/moose2.css" => {
                return Response::from_data("text/css", APP_CSS).with_etag(req, &*APP_CSS_CRC32_STR)
            }
            "/public/moose2.js" => {
                return Response::from_data("application/javascript", APP_JS)
                    .with_etag(req, &*APP_JS_CRC32_STR)
            }
            "/favicon.ico" => {
                return Response::from_data("image/x-icon", APP_ICON)
                    .with_etag(req, &*APP_ICON_CRC32_STR);
            }
            "/" | "/gallery" | "/gallery/" => return Response::redirect_303("/gallery/0"),
            "/gallery/random" => {
                let max_page = { db.read().unwrap().page_count() };
                let rand_idx = rand::thread_rng().gen_range(0..max_page);
                return Response::redirect_303(format!("/gallery/{}", rand_idx));
            }
            "/gallery/nojs-search" | "/search" => {
                let query = get_query_param_value(req.raw_query_string(), "q");

                if query.len() > 50 {
                    return json_err_res(400, "query is too large (max 50)");
                } else if query.is_empty() {
                    return json_err_res(400, "query is empty");
                } else {
                    let unlocked = db.read().unwrap();
                    let meese = unlocked.find_page_with_link(&query);
                    let (content_type, body): (_, Vec<u8>) = if req.url() == "/search" {
                        ("application/json", meese.into())
                    } else {
                        (
                            "text/html; charset=utf-8",
                            gallery::nojs_search("Search Results", meese)
                                .into_string()
                                .into(),
                        )
                    };
                    let crc = crc32fast::hash(&body).to_string();
                    return Response::from_data(content_type, body).with_etag(req, crc);
                }
            }
            "/page" => {
                let page_count = { db.read().unwrap().page_count() };
                return Response::from_data("application/json", format!("{}", page_count));
            }
            _ => (),
        }
    }

    router!(req,
        (GET) (/{moose_type: TypeRoutes}/{moose_name: String}) => {
            let db_locked = db.read().unwrap();
            match simple_get(&db_locked, &moose_name) {
                Ok(Some(moose)) => {
                    let body = match moose_type {
                        TypeRoutes::Irc => moose_irc(moose),
                        TypeRoutes::Term => moose_term(moose),
                        TypeRoutes::Png => moose_png(moose).unwrap(),
                        TypeRoutes::Moose => moose.into(),
                    };
                    let crc = crc32fast::hash(&body).to_string();
                    Response::from_data(moose_type.content_type(), body).with_public_cache(3600).with_etag(req, crc)
                },
                Ok(None) => {
                    moose_404(&moose_name)
                },
                Err(redir) => Response::redirect_303(format!("/{}/{}", moose_type, redir)),
            }
        },
        (GET) (/gallery/{pid: usize}) => {
            let db_locked = db.read().unwrap();
            let meese = db_locked.get_page(pid);
            let html = gallery::gallery(&format!("Page {}", pid), pid, db_locked.page_count(), meese).into_string();
            let html_crc = crc32fast::hash(html.as_bytes()).to_string();
            Response::from_data("text/html", html).with_etag(req, html_crc)
        },
        (GET) (/page/{pid: usize}) => {
            let db_locked = db.read().unwrap();
            let meese: Vec<u8> = db_locked.get_page(pid).into();
            let crc = crc32fast::hash(&meese).to_string();
            Response::from_data("application/json", meese).with_etag(req, crc)
        },
        _ => json_err_res(404, "path not routable"),
    )
}
