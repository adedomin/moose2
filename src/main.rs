// use moosedb::MooseDb;
use moosedb::MooseDb;
use rand::Rng;
use rouille::{router, Request, Response};
use std::sync::{Arc, RwLock};

pub mod config;
pub mod moosedb;

fn handler(db: Arc<RwLock<MooseDb>>, req: &Request) -> Response {
    router!(req,
        (GET) (/moose/random) => {
            let unlocked = db.read().unwrap();
            if unlocked.meese.is_empty() {
                Response::empty_404()
            } else {
                let rand_idx = rand::thread_rng().gen_range(0..unlocked.meese.len());
                let moose_name = unlocked.meese[rand_idx].name.clone();
                Response::redirect_303(format!("/moose/{}", moose_name))
            }
        },
        (GET) (/moose/latest) => {
            let unlocked = db.read().unwrap();
            if let Some(last_moose) =  unlocked.meese.last() {
                let moose_name = last_moose.name.clone();
                Response::redirect_303(format!("/moose/{}", moose_name))
            } else {
                Response::empty_404()
            }
        },
        (GET) (/moose/{moose: String}) => {
            let unlocked = db.read().unwrap();
            if let Some(moose) = unlocked.get_bin(&moose) {
                Response::from_data("application/json", moose)
            } else {
                Response::empty_404()
            }
        },
        (GET) (/page) => {
            let unlocked = db.read().unwrap();
            Response::from_data("application/json", format!("{}", unlocked.page_count()))
        },
        (GET) (/page/{pid: usize}) => {
            let unlocked = db.read().unwrap();
            let meese = unlocked.get_page_bin(pid);
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

fn main() {
    let args = <config::Args as clap::Parser>::parse();
    if let config::SubArg::Import { file, output } = args.command {
        moosedb::moose_bulk_transform(file, output);
        return;
    }

    let db = Arc::new(RwLock::new(MooseDb::open().unwrap()));
    let listen_addr = args.get_bind_addr();
    println!("Attempting to listen on: http://{}/", listen_addr);
    rouille::start_server(listen_addr, move |req| handler(db.clone(), req));
}
