use crate::moosedb::{Moose, MooseDb};
use crate::render::{moose_irc, moose_png, moose_term};
use crate::templates::gallery;
use lazy_static::lazy_static;
use rand::Rng;
use rouille::percent_encoding::percent_decode;
use rouille::{
    percent_encoding::{percent_encode, NON_ALPHANUMERIC},
    router, Request, Response,
};
use std::borrow::Cow;
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
            "/" | "/gallery" => return Response::redirect_303("/gallery/0"),
            "/gallery/random" => {
                let max_page = { db.read().unwrap().page_count() };
                let rand_idx = rand::thread_rng().gen_range(0..max_page);
                return Response::redirect_303(format!("/gallery/{}", rand_idx));
            }
            "/gallery/nojs-search" => {
                let query = get_query_param_value(req.raw_query_string(), "q");
                let unlocked = db.read().unwrap();
                let meese = unlocked.find_page_with_link(&query);
                let html = gallery::nojs_search("Search Results", meese).into_string();
                let html_crc = crc32fast::hash(html.as_bytes()).to_string();
                return Response::from_data("text/html", html).with_etag(req, html_crc);
            }
            "/page" => {
                let page_count = { db.read().unwrap().page_count() };
                return Response::from_data("application/json", format!("{}", page_count));
            }
            "/search" => {
                let query = get_query_param_value(req.raw_query_string(), "q");

                if query.len() > 50 {
                    return json_err_res(400, "query is too large (max 50)");
                } else if query.is_empty() {
                    return json_err_res(400, "query is empty");
                } else {
                    let unlocked = db.read().unwrap();
                    let meese = unlocked.find_page_with_link_bin(&query);
                    let meese_crc = crc32fast::hash(&meese).to_string();
                    return Response::from_data("application/json", meese)
                        .with_etag(req, meese_crc);
                }
            }
            _ => (),
        }
    }

    router!(req,
        (GET) (/moose/{moose_name: String}) => {
            let db_locked = db.read().unwrap();
            match simple_get(&db_locked, &moose_name) {
                Ok(Some(moose)) => Response::from_data("application/json", moose).with_public_cache(3600),
                Ok(None) => {
                    moose_404(&moose_name)
                },
                Err(redir) => Response::redirect_303(format!("/moose/{}", redir)),
            }
        },
        (GET) (/irc/{moose_name: String}) => {
            let db_locked = db.read().unwrap();
            match simple_get(&db_locked, &moose_name) {
                Ok(Some(moose)) => Response::from_data("text/irc-art", moose_irc(moose)).with_public_cache(3600),
                Ok(None) => {
                    moose_404(&moose_name)
                },
                Err(redir) => Response::redirect_303(format!("/irc/{}", redir)),
            }
        },
        (GET) (/term/{moose_name: String}) => {
            let db_locked = db.read().unwrap();
            match simple_get(&db_locked, &moose_name) {
                Ok(Some(moose)) => Response::from_data("text/ansi-truecolor", moose_term(moose)).with_public_cache(3600),
                Ok(None) => {
                    moose_404(&moose_name)
                },
                Err(redir) => Response::redirect_303(format!("/irc/{}", redir)),
            }
        },
        (GET) (/img/{moose_name: String}) => {
            let db_locked = db.read().unwrap();
            match simple_get(&db_locked, &moose_name) {
                Ok(Some(moose)) => {
                    match moose_png(moose) {
                        Ok(png) => Response::from_data("image/png", png).with_public_cache(3600),
                        Err(e) => {
                            // bad error...
                            json_err_res(500, &e.to_string())
                        }
                    }
                },
                Ok(None) => {
                    moose_404(&moose_name)
                },
                Err(redir) => Response::redirect_303(format!("/img/{}", redir)),
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
            let meese = db_locked.get_page(pid);
            Response::from_data("application/json", meese)
        },
        _ => Response::empty_404(),
    )
}
