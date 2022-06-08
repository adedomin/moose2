use crate::moosedb::{Moose, MooseDb};
use crate::render::{moose_png, IrcArt};
use crate::templates::gallery;
use astra::{Body, Request, Response, ResponseBuilder};
use core::fmt;
use lazy_static::lazy_static;
use matchit::Router;
use percent_encoding::{percent_decode, percent_encode, NON_ALPHANUMERIC};
use rand::Rng;
use std::borrow::Cow;
use std::sync::{Arc, RwLock, RwLockReadGuard};

const RANDOM: &str = "random";
const LATEST: &str = "latest";

fn get_query_params(q: &str) -> impl Iterator<Item = (&str, &str)> {
    q.split_terminator('&')
        .map(|kv| kv.split_once('=').unwrap_or((kv, "")))
}

fn get_query_param_value<'m>(req: &'m Request, param: &str) -> Cow<'m, str> {
    if let Some(qstring) = req.uri().query() {
        let ret = get_query_params(qstring)
            .find(|(key, _)| *key == param)
            .map(|(_, val)| val)
            .unwrap_or("");
        percent_decode(ret.as_bytes()).decode_utf8_lossy()
    } else {
        std::borrow::Cow::Borrowed("")
    }
}

fn error_resp(code: u16, res_type: &'static str, message: String) -> Response {
    ResponseBuilder::new()
        .status(code)
        .header("Content-Type", res_type)
        .header("Content-Length", message.as_bytes().len())
        .body(Body::new(message))
        .unwrap()
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

fn generate_etag(body: &[u8]) -> String {
    crc32fast::hash(body).to_string()
}

fn is_etag_match(req: &Request, calc: &str) -> bool {
    if let Some(etag) = req.headers().get("If-None-Match") {
        calc == etag
    } else {
        false
    }
}

enum Routes {
    Moose,
    Irc,
    Img,
    Gallery,
    Page,
}

impl Routes {
    fn content_type(&self) -> &'static str {
        match self {
            Routes::Moose | Routes::Page => "application/json",
            Routes::Irc => "text/irc-art",
            Routes::Img => "image/png",
            Routes::Gallery => "text/html",
        }
    }
}

impl fmt::Display for Routes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Routes::Moose => f.write_str("/moose"),
            Routes::Irc => f.write_str("/irc"),
            Routes::Img => f.write_str("/img"),
            Routes::Gallery => f.write_str("/gallery"),
            Routes::Page => f.write_str("/page"),
        }
    }
}

fn init_router() -> Router<Routes> {
    let mut router = matchit::Router::new();
    router.insert("/moose/:id", Routes::Moose).unwrap();
    router.insert("/irc/:id", Routes::Irc).unwrap();
    router.insert("/img/:id", Routes::Img).unwrap();
    router.insert("/gallery/:id", Routes::Gallery).unwrap();
    router.insert("/page/:id", Routes::Page).unwrap();
    router
}

lazy_static! {
    static ref APP_CSS_CRC32: String = generate_etag(APP_CSS);
    static ref APP_ICON_CRC32: String = generate_etag(APP_ICON);
    static ref APP_JS_CRC32: String = generate_etag(APP_JS);
}

lazy_static! {
    static ref APP_ROUTER: Router<Routes> = init_router();
}

macro_rules! static_resp {
    ($req:expr, $content_type:expr, $body:expr, $etag:expr) => {{
        let b = astra::ResponseBuilder::new().header("Content-Type", $content_type);
        if is_etag_match($req, &*$etag) {
            b.status(304).header("etag", &*$etag).body(Body::empty())
        } else {
            b.status(200)
                .header("Content-Length", $body.len())
                .header("etag", &*$etag)
                .body(Body::new($body))
        }
        .unwrap()
    }};
}

fn byte_resp<T>(req: &Request, content_type: &'static str, body: T) -> Response
where
    T: Into<Vec<u8>>,
{
    let body = body.into();
    let etag = generate_etag(&body);
    let res = ResponseBuilder::new()
        .header("Content-Type", content_type)
        .header("Content-Length", body.len())
        .header("etag", &etag);

    if is_etag_match(req, &etag) {
        res.status(304).body(Body::empty())
    } else {
        res.status(200).body(Body::new(body))
    }
    .unwrap()
}

fn cache_resp<T>(req: &Request, content_type: &'static str, body: T) -> Response
where
    T: Into<Vec<u8>>,
{
    let body = body.into();
    let etag = generate_etag(&body);
    let res = ResponseBuilder::new()
        .header("Content-Type", content_type)
        .header("Cache-Control", "public, max-age=3600")
        .header("etag", &etag);

    if is_etag_match(req, &etag) {
        res.status(304).body(Body::empty())
    } else {
        res.status(200)
            .header("Content-Length", body.len())
            .body(Body::new(body))
    }
    .unwrap()
}

fn redirect_303(path: &str) -> Response {
    ResponseBuilder::new()
        .status(303)
        .header("location", path)
        .body(Body::empty())
        .unwrap()
}

pub fn handler(db: Arc<RwLock<MooseDb>>, req: Request) -> Response {
    // static paths and redirects
    if req.method() == "GET" {
        match req.uri().path() {
            "/public/moose2.css" => {
                return static_resp!(&req, "text/css", APP_CSS, APP_CSS_CRC32);
            }
            "/public/moose2.js" => {
                return static_resp!(&req, "application/javascript", APP_JS, APP_JS_CRC32);
            }
            "/favicon.ico" => {
                return static_resp!(&req, "image/x-icon", APP_ICON, APP_ICON_CRC32);
            }
            "/" | "/gallery" => return redirect_303("/gallery/0"),
            "/gallery/random" => {
                let max_page = { db.read().unwrap().page_count() };
                let rand_idx = rand::thread_rng().gen_range(0..max_page);
                return redirect_303(&format!("/gallery/{}", rand_idx));
            }
            "/gallery/nojs-search" => {
                let query = get_query_param_value(&req, "q");
                let unlocked = db.read().unwrap();
                let meese = unlocked.find_page_with_link(&query);
                let html = gallery::nojs_search("Search Results", meese).into_string();
                return byte_resp(&req, "text/html", html);
            }

            "/page" => {
                let page_count = { db.read().unwrap().page_count() };
                let body = format!("{}", page_count);
                return ResponseBuilder::new()
                    .status(200)
                    .header("Content-Type", "application/json")
                    .header("Content-Length", body.as_bytes().len())
                    .body(Body::new(body))
                    .unwrap();
            }
            "/search" => {
                let query = get_query_param_value(&req, "q");

                if query.len() > 50 {
                    return json_err_res(400, "query is too large (max 50)");
                } else if query.is_empty() {
                    return json_err_res(400, "query is empty");
                } else {
                    let unlocked = db.read().unwrap();
                    let meese = unlocked.find_page_with_link_bin(&query);
                    return byte_resp(&req, "application/json", meese);
                }
            }
            _ => (),
        }
    }

    if let Ok(matched) = APP_ROUTER.at(req.uri().path()) {
        match matched.value {
            Routes::Moose | Routes::Irc | Routes::Img => {
                let db_locked = db.read().unwrap();
                serve_moose(&req, db_locked, matched.value, matched.params.get("id"))
            }
            Routes::Gallery | Routes::Page => {
                if let Some(pid) = matched.params.get("id") {
                    if let Ok(pid) = pid.parse() {
                        let db_locked = db.read().unwrap();
                        let meese = db_locked.get_page(pid);
                        let body: Vec<u8> = if let Routes::Gallery = matched.value {
                            gallery::gallery(
                                &format!("Page {}", pid),
                                pid,
                                db_locked.page_count(),
                                meese,
                            )
                            .into_string()
                            .into()
                        } else {
                            meese.into()
                        };
                        byte_resp(&req, matched.value.content_type(), body)
                    } else {
                        json_err_res(400, "ID is not a number")
                    }
                } else {
                    json_err_res(400, "Invalid ID")
                }
            }
        }
    } else {
        json_err_res(404, "no such path")
    }
}

fn serve_moose(
    req: &Request,
    db: RwLockReadGuard<MooseDb>,
    mtype: &Routes,
    id_enc: Option<&str>,
) -> Response {
    if let Some(id_enc) = id_enc {
        if let Ok(name) = percent_decode(id_enc.as_bytes()).decode_utf8() {
            match simple_get(&db, &name) {
                Ok(Some(moose)) => {
                    let body = match mtype {
                        Routes::Moose => moose.into(),
                        Routes::Irc => IrcArt::from(moose).into(),
                        Routes::Img => moose_png(moose).unwrap(),
                        _ => unreachable!(),
                    };
                    cache_resp(req, mtype.content_type(), body)
                }
                Ok(None) => moose_404(&name),
                Err(new_path) => redirect_303(&format!("{}/{}", mtype, new_path)),
            }
        } else {
            json_err_res(400, "invalid utf-8 moose name")
        }
    } else {
        json_err_res(404, "no moose given")
    }
}
