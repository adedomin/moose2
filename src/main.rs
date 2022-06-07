// use moosedb::MooseDb;
use moosedb::MooseDb;
use signal_hook::{consts::SIGHUP, iterator::Signals};
use std::{
    sync::{Arc, RwLock},
    thread,
};

pub mod config;
pub mod moosedb;
pub mod render;
pub mod templates;
pub mod web;

fn main() {
    let args = <config::Args as clap::Parser>::parse();
    if let config::SubArg::Import { file, output } = args.command {
        moosedb::moose_bulk_transform(file, output);
        return;
    }

    let db = Arc::new(RwLock::new(MooseDb::open().unwrap()));
    let db_reload = db.clone();
    let listen_addr = args.get_bind_addr();

    thread::spawn(move || {
        println!("Attempting to listen on: http://{}/", listen_addr);
        rouille::start_server(listen_addr, move |req| web::handler(db.clone(), req));
    });

    let mut signals = Signals::new([SIGHUP]).expect("expected to listen on sighup");
    for signal in signals.forever() {
        if let SIGHUP = signal {
            println!("Attempting to reload MooseDB...");
            {
                let mut dblock = db_reload.write().unwrap();
                *dblock = MooseDb::open().unwrap();
            }
            println!("reloaded MooseDB.");
        }
    }
}
