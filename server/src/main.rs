// use moosedb::MooseDb;
use crate::{
    config::{GitHubOauth2, SubCommand},
    db::moose_bulk_import,
    model::moose::moose_bulk_transform,
    task::dump_moose,
};
use actix_session::{storage::CookieSessionStore, SessionMiddleware};
use actix_web::{cookie, App, HttpServer};
use db::Pool;
use oauth2::{basic::BasicClient, AuthUrl, ClientId, ClientSecret, TokenUrl};
use tokio::{
    signal::unix::SignalKind,
    sync::broadcast::{self, error::SendError},
    task::JoinHandle,
};

pub mod config;
pub mod db;
pub mod model;
pub mod render;
pub mod shared_data;
pub mod task;
pub mod templates;
pub mod web_handlers;

pub struct AppData {
    pub oauth2_client: Option<BasicClient>,
    pub db: Pool,
}

fn main() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .enable_io()
        .build()
        .unwrap();
    rt.block_on(async {
        let (subcmd, rc) = config::parse_args();
        if let Some(sub) = subcmd {
            match sub {
                SubCommand::Import { input } => {
                    moose_bulk_import(input, &rc).await;
                    return;
                }
                SubCommand::Convert { input, output } => {
                    moose_bulk_transform(input, output);
                    return;
                }
            }
        }

        println!(
            "INFO: [MAIN] Connecting to database: {:?}",
            rc.get_moose_path()
        );
        let db = db::open_db(&rc).await;

        let moose_dump_file = rc.get_moose_dump();
        println!(
            "INFO: [DUMP] Setting up Auto-dumps of database to: {:?}",
            moose_dump_file
        );
        let dbx = db.clone();
        let (stopchan_tx, rx1) = broadcast::channel(1);
        let dump_task = tokio::spawn(async move {
            let e = dump_moose(moose_dump_file, dbx, rx1).await;
            println!("WARN: [DUMP] Task has shut down: {:?}", e);
            e
        });

        let listen_addr = rc.get_bind_addr();
        println!(
            "INFO: [WEB] Attempting to listen on: http://{}/",
            listen_addr
        );
        let oauth2_client = match &rc.github_oauth2 {
            Some(GitHubOauth2 { id, secret }) => {
                let client_id = ClientId::new(id.to_string());
                let secret = Some(ClientSecret::new(secret.to_string()));
                let auth_url =
                    AuthUrl::new("https://github.com/login/oauth/authorize".to_string()).unwrap();
                let token_url = Some(
                    TokenUrl::new("https://github.com/login/oauth/access_token".to_string())
                        .unwrap(),
                );
                Some(BasicClient::new(client_id, secret, auth_url, token_url))
            }
            None => None,
        };
        let app_data = actix_web::web::Data::new(AppData { oauth2_client, db });
        let builder = HttpServer::new(move || {
            let cookie_session = SessionMiddleware::builder(
                CookieSessionStore::default(),
                cookie::Key::from(&rc.cookie_key.0),
            )
            .cookie_secure(false)
            .build();
            App::new()
                .wrap(cookie_session)
                .app_data(app_data.clone())
                .service(web_handlers::oauth2_gh::login)
                .service(web_handlers::oauth2_gh::auth)
                .service(web_handlers::static_files::static_gallery_file)
                .service(web_handlers::static_files::favicon)
                .service(web_handlers::static_files::const_js_modules)
                .service(web_handlers::static_files::index_page)
                .service(web_handlers::static_files::err_js_script)
                .service(web_handlers::api::get_moose)
                .service(web_handlers::api::get_moose_img)
                .service(web_handlers::api::get_moose_irc)
                .service(web_handlers::api::get_moose_term)
                .service(web_handlers::api::get_page_count)
                .service(web_handlers::api::get_page)
                .service(web_handlers::api::get_page_nav_range)
                .service(web_handlers::api::get_search_page)
                .service(web_handlers::api::put_new_moose)
                .service(web_handlers::display::gallery_redir)
                .service(web_handlers::display::gallery_random_redir)
                .service(web_handlers::display::gallery_latest_redir)
                .service(web_handlers::display::gallery_page)
        });
        let web_svr = if !listen_addr.starts_with("unix:") {
            builder.bind(listen_addr).unwrap()
        } else {
            builder.bind_uds(&listen_addr[5..]).unwrap()
        };
        let web_svr = web_svr.disable_signals().shutdown_timeout(10).run();
        let web_handle = web_svr.handle();
        let web_task = tokio::spawn(async {
            let e = web_svr.await;
            println!("WARN: [WEB] Task has shut down: {:?}", e);
            e
        });

        let shutdown_task: JoinHandle<Result<(), SendError<_>>> = tokio::spawn(async move {
            let mut sigterm = tokio::signal::unix::signal(SignalKind::terminate()).unwrap();
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {
                    println!("WARN: [SHUTDOWN] SIGINT: shutting down.");
                    web_handle.stop(true).await;
                    stopchan_tx.send(true)?;
                }
                _ = sigterm.recv() => {
                    println!("WARN: [SHUTDOWN] SIGTERM: shutting down.");
                    web_handle.stop(true).await;
                    stopchan_tx.send(true)?;
                }
            }
            Ok(())
        });

        let _ = tokio::try_join!(shutdown_task, web_task, dump_task)
            .expect("All tasks to start/shutdown successfully.");
    });
}
