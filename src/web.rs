use crate::html;
use crate::moosedb::{Moose, MooseDb};
use crate::render::{moose_png, IrcArt};
use lazy_static::lazy_static;
use rand::Rng;
use rouille::{
    percent_encoding::{percent_encode, NON_ALPHANUMERIC},
    router, Request, Response,
};
use std::sync::{Arc, RwLock, RwLockReadGuard};

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

const APP_CSS: &[u8] = include_bytes!("public/moose2.css");
const APP_ICON: &[u8] = include_bytes!("public/favicon.ico");

lazy_static! {
    pub static ref APP_CSS_CRC32: u32 = crc32fast::hash(APP_CSS);
    pub static ref APP_CSS_CRC32_STR: String = APP_CSS_CRC32.to_string();
    pub static ref APP_ICON_CRC32: u32 = crc32fast::hash(APP_ICON);
    pub static ref APP_ICON_CRC32_STR: String = APP_ICON_CRC32.to_string();
}

pub fn handler(db: Arc<RwLock<MooseDb>>, req: &Request) -> Response {
    // static paths and redirects
    if req.method() == "GET" {
        match req.url().as_str() {
            "/public/moose2.css" => {
                return Response::from_data("text/css", APP_CSS).with_etag(req, &*APP_CSS_CRC32_STR)
            }
            "favicon.ico" => {
                return Response::from_data("image/x-icon", APP_ICON)
                    .with_etag(req, &*APP_ICON_CRC32_STR)
            }
            "/" | "/gallery" => return Response::redirect_303("/gallery/0"),
            "/gallery/random" => {
                let max_page = { db.read().unwrap().page_count() };
                let rand_idx = rand::thread_rng().gen_range(0..max_page);
                return Response::redirect_303(format!("/gallery/{}", rand_idx));
            }
            "/page" => {
                let page_count = { db.read().unwrap().page_count() };
                return Response::from_data("application/json", format!("{}", page_count));
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
                    let mut e = Response::text(format!("no such moose: {}", moose_name));
                    e.status_code = 404u16;
                    e
                },
                Err(redir) => Response::redirect_303(format!("/moose/{}", redir)),
            }
        },
        (GET) (/irc/{moose_name: String}) => {
            let db_locked = db.read().unwrap();
            match simple_get(&db_locked, &moose_name) {
                Ok(Some(moose)) => Response::from_data("text/irc-art", IrcArt::from(moose)).with_public_cache(3600),
                Ok(None) => {
                    let mut e = Response::text(format!("no such moose: {}", moose_name));
                    e.status_code = 404u16;
                    e
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
                            let mut e = Response::text(e.to_string());
                            e.status_code = 500;
                            e
                        }
                    }
                },
                Ok(None) => {
                    let mut e = Response::text(format!("no such moose: {}", moose_name));
                    e.status_code = 404u16;
                    e
                },
                Err(redir) => Response::redirect_303(format!("/img/{}", redir)),
            }
        },
        (GET) (/gallery/{pid: usize}) => {
            let db_locked = db.read().unwrap();
            let meese = db_locked.get_page(pid);
            let html = html::gallery_page(meese, pid, db_locked.page_count());
            let html_crc = crc32fast::hash(html.as_bytes()).to_string();
            Response::from_data("text/html", html).with_etag(req, html_crc)
        },

        (GET) (/page/{pid: usize}) => {
            let db_locked = db.read().unwrap();
            let meese = db_locked.get_page(pid);
            Response::from_data("application/json", meese)
        },
        (GET) (/search/{query: String}) => {
            if query.len() > 50 {
                let mut e = Response::from_data("application/json", r#"{"status":"error","msg":"query length too long"}"#.to_owned());
                e.status_code = 400u16;
                e
            } else if query.is_empty() {
                let mut e = Response::from_data("application/json", r#"{"status":"error","msg":"query is empty"}"#.to_owned());
                e.status_code = 400u16;
                e
            } else {
                let unlocked = db.read().unwrap();
                let meese = unlocked.find_page_bin(&query);
                Response::from_data("application/json", meese)
            }
        },
        _ => Response::empty_404(),
    )
}
