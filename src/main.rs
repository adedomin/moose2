// use moosedb::MooseDb;
use crate::{
    config::{get_config, Args, GitHubOauth2},
    db::moose_bulk_import,
    model::moose::moose_bulk_transform,
};
use actix_session::{storage::CookieSessionStore, SessionMiddleware};
use actix_web::{cookie, App, HttpServer};
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

pub struct AppData {
    pub oauth2_client: Option<BasicClient>,
    pub db: Pool,
}

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
            let oauth2_client = match &config::get_config().github_oauth2 {
                Some(GitHubOauth2 { id, secret }) => {
                    let client_id = ClientId::new(id.to_string());
                    let secret = Some(ClientSecret::new(secret.to_string()));
                    let auth_url =
                        AuthUrl::new("https://github.com/login/oauth/authorize".to_string())
                            .unwrap();
                    let token_url = Some(
                        TokenUrl::new("https://github.com/login/oauth/access_token".to_string())
                            .unwrap(),
                    );
                    Some(BasicClient::new(client_id, secret, auth_url, token_url))
                }
                None => None,
            };
            let app_data = actix_web::web::Data::new(AppData {
                oauth2_client,
                db: db::open_db(),
            });
            let cookie_session = SessionMiddleware::builder(
                CookieSessionStore::default(),
                cookie::Key::from(&get_config().cookie_key.0),
            )
            .cookie_secure(false)
            .build();
            App::new()
                .wrap(cookie_session)
                .app_data(app_data)
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
                .service(web_handlers::api::put_new_moose)
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
