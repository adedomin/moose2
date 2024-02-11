use actix_session::{storage::CookieSessionStore, SessionMiddleware};
use actix_web::{cookie, App, HttpServer};
use oauth2::{basic::BasicClient, AuthUrl, ClientId, ClientSecret, TokenUrl};
use tokio::{sync::broadcast::Receiver, task::JoinHandle};

use crate::{
    config::{GitHubOauth2, RunConfig},
    db::Pool,
    model::app_data::AppData,
    web_handlers,
};

pub fn web_task(
    rc: RunConfig,
    db: Pool,
    mut shutdown_signal: Receiver<bool>,
) -> JoinHandle<Result<(), std::io::Error>> {
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
                TokenUrl::new("https://github.com/login/oauth/access_token".to_string()).unwrap(),
            );
            Some(BasicClient::new(client_id, secret, auth_url, token_url))
        }
        None => None,
    };
    let moose_dump = rc.get_moose_dump();
    let app_data = actix_web::web::Data::new(AppData {
        oauth2_client,
        db,
        moose_dump,
    });
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
            .service(web_handlers::static_files::static_root_file)
            .service(web_handlers::static_files::gridpaint_modules)
            .service(web_handlers::static_files::gridpaint_lib_modules)
            .service(web_handlers::api::get_moose)
            .service(web_handlers::api::get_moose_img)
            .service(web_handlers::api::get_moose_irc)
            .service(web_handlers::api::get_moose_term)
            .service(web_handlers::api::get_page_count)
            .service(web_handlers::api::get_page)
            .service(web_handlers::api::get_page_nav_range)
            .service(web_handlers::api::get_search_page)
            .service(web_handlers::api::put_new_moose)
            .service(web_handlers::api::get_dump)
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
    tokio::spawn(async move {
        shutdown_signal.recv().await.unwrap();
        web_handle.stop(true).await;
    });
    tokio::spawn(async move {
        let e = web_svr.await;
        println!("WARN: [WEB] Task has shut down: {:?}", e);
        e
    })
}
