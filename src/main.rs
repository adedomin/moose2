// use moosedb::MooseDb;
use crate::{
    config::{get_config, Args, GitHubOauth2},
    db::moose_bulk_import,
    model::moose::moose_bulk_transform,
};
use actix_web::{App, HttpServer};
use db::Pool;
use oauth2::{basic::BasicClient, AuthUrl, ClientId, ClientSecret, TokenUrl};
use std::io;

pub mod config;
pub mod db;
pub mod model;
pub mod render;
pub mod shared_data;
pub mod templates;
pub mod web_handlers;
fn main() -> io::Result<()> {
    match config::parse() {
        Args::Run => (),
        Args::Import(file) => {
            moose_bulk_import(file);
            return Ok(());
        }
        Args::Convert(path1, path2) => {
            moose_bulk_transform(path1, path2);
            return Ok(());
        }
    }

    let listen_addr = get_config().get_bind_addr();
    println!("Attempting to listen on: http://{}/", listen_addr);
    actix_web::rt::System::new().block_on({
        let builder = HttpServer::new(move || {
            App::new()
                .app_data(actix_web::web::Data::new(db::open_db()))
                .service(web_handlers::oauth2_gh::login)
                .service(web_handlers::oauth2_gh::auth)
                .service(web_handlers::static_files::static_file)
                .service(web_handlers::static_files::favicon)
                .service(web_handlers::static_files::const_js_modules)
                .service(web_handlers::static_files::db_dump)
                .service(web_handlers::api::get_moose)
                .service(web_handlers::api::get_moose_img)
                .service(web_handlers::api::get_moose_irc)
                .service(web_handlers::api::get_moose_term)
                .service(web_handlers::api::get_page_count)
                .service(web_handlers::api::get_page)
                .service(web_handlers::api::get_page_nav_range)
                .service(web_handlers::api::get_search_res)
                .service(web_handlers::display::gallery_redir)
                .service(web_handlers::display::gallery_random_redir)
                .service(web_handlers::display::nojs_gallery_search)
                .service(web_handlers::display::gallery_page)
        });
        if !listen_addr.starts_with("unix:") {
            builder.bind(listen_addr)?.run()
        } else {
            builder.bind_uds(&listen_addr[5..])?.run()
        }
    })
}
