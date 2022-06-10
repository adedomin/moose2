// use moosedb::MooseDb;
use moosedb::MooseDb;
use signal_hook::{consts::SIGHUP};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, RwLock,
    },
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
    let reload_signal = Arc::new(AtomicBool::new(false));
    let _ = signal_hook::flag::register(SIGHUP, reload_signal.clone())
        .expect("expected to register SIGHUP handler");
    let listen_addr = args.get_bind_addr();

    println!("Attempting to listen on: http://{}/", listen_addr);
    rouille::start_server(listen_addr, move |req| {
        if let Ok(true) =
            reload_signal.compare_exchange(true, false, Ordering::Acquire, Ordering::Relaxed)
        {
            println!("Reloading Moose database...");
            let mut db = db.write().unwrap();
            *db = MooseDb::open().unwrap();
            println!("Reloaded");
        }
        web::handler(db.clone(), req)
    });
}
