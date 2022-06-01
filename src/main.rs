// use moosedb::MooseDb;
use moosedb::MooseDb;
use std::sync::{Arc, RwLock};

pub mod config;
pub mod html;
pub mod moosedb;
pub mod render;
pub mod web;

fn main() {
    let args = <config::Args as clap::Parser>::parse();
    if let config::SubArg::Import { file, output } = args.command {
        moosedb::moose_bulk_transform(file, output);
        return;
    }

    let db = Arc::new(RwLock::new(MooseDb::open().unwrap()));
    let listen_addr = args.get_bind_addr();
    println!("Attempting to listen on: http://{}/", listen_addr);
    rouille::start_server(listen_addr, move |req| web::handler(db.clone(), req));
}
