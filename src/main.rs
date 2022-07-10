// use moosedb::MooseDb;
use actix_web::{App, HttpServer};
use moosedb::MooseDb;
use signal_hook::{consts::SIGHUP, iterator::Signals};
use std::{io, sync::RwLock, thread};

pub mod config;
pub mod moosedb;
pub mod render;
pub mod shared_data;
pub mod templates;
pub mod web_handlers;

fn main() -> io::Result<()> {
    let args = <config::Args as clap::Parser>::parse();
    if let config::SubArg::Import { file, output } = args.command {
        moosedb::moose_bulk_transform(file, output);
        return Ok(());
    }

    let moosedb = actix_web::web::Data::new(RwLock::new(MooseDb::open().unwrap()));

    let moosedb_clone = moosedb.clone();
    let mut signal_handler = Signals::new(&[SIGHUP]).unwrap();
    thread::spawn(move || {
        let signals = signal_handler.forever();
        for signal in signals {
            match signal {
                SIGHUP => {
                    println!("Reloading MooseDb......");
                    let mut locked = moosedb_clone.write().unwrap();
                    *locked = MooseDb::open().unwrap();
                    println!("Reloaded MooseDb.");
                }
                _ => unreachable!(),
            }
        }
    });

    let listen_addr = args.get_bind_addr();
    println!("Attempting to listen on: http://{}/", listen_addr);
    actix_web::rt::System::new().block_on({
        let builder = HttpServer::new(move || {
            App::new()
                .app_data(moosedb.clone())
                .service(web_handlers::static_files::static_file)
                .service(web_handlers::static_files::favicon)
                .service(web_handlers::static_files::const_js_modules)
                .service(web_handlers::api::get_moose)
                .service(web_handlers::api::get_moose_img)
                .service(web_handlers::api::get_moose_irc)
                .service(web_handlers::api::get_moose_term)
                .service(web_handlers::api::get_page_count)
                .service(web_handlers::api::get_page)
                .service(web_handlers::api::get_search_res)
                .service(web_handlers::display::gallery_redir)
                .service(web_handlers::display::gallery_random_redir)
                .service(web_handlers::display::gallery_page)
        });
        if !listen_addr.starts_with("unix:") {
            builder.bind(listen_addr)?.run()
        } else {
            builder.bind_uds(&listen_addr[5..])?.run()
        }
    })
}
