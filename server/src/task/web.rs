/* Copyright (C) 2024  Anthony DeDominic
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use actix_session::{SessionMiddleware, storage::CookieSessionStore};
use actix_web::{App, HttpServer, cookie};
use oauth2::{AuthUrl, ClientId, ClientSecret, RedirectUrl, TokenUrl, basic::BasicClient};
use tokio::{sync::broadcast::Receiver, task::JoinHandle};

use crate::{
    config::{GitHubOauth2, RunConfig},
    db::sqlite3_impl::Pool,
    model::app_data::{AppData, Oa},
    web_handlers,
};

pub fn web_task(
    rc: RunConfig,
    db: Pool,
    mut shutdown_signal: Receiver<()>,
) -> JoinHandle<Result<(), std::io::Error>> {
    let listen_addr = rc.get_bind_addr();
    println!(
        "INFO: [WEB] Attempting to listen on: http://{}/",
        listen_addr
    );
    let oauth2_client = match &rc.github_oauth2 {
        Some(GitHubOauth2 {
            id,
            secret,
            redirect,
        }) => {
            let client_id = ClientId::new(id.to_string());
            let secret = ClientSecret::new(secret.to_string());
            let auth_url =
                AuthUrl::new("https://github.com/login/oauth/authorize".to_string()).unwrap();
            let token_url =
                TokenUrl::new("https://github.com/login/oauth/access_token".to_string()).unwrap();
            let oa = BasicClient::new(client_id)
                .set_client_secret(secret)
                .set_auth_uri(auth_url)
                .set_token_uri(token_url);
            let oa = if let Some(redir) = redirect {
                let redir = RedirectUrl::new(redir.clone()).unwrap();
                oa.set_redirect_uri(redir)
            } else {
                oa
            };
            Some(Oa {
                oa,
                web: {
                    reqwest::Client::builder()
                        .user_agent(concat!(
                            env!("CARGO_PKG_NAME"),
                            "/",
                            env!("CARGO_PKG_VERSION")
                        ))
                        .redirect(reqwest::redirect::Policy::none())
                        .build()
                        .unwrap()
                },
            })
        }
        None => None,
    };
    let moose_dump = rc.get_moose_dump();
    let app_data = actix_web::web::Data::new(AppData {
        db,
        moose_dump,
        oauth2_client,
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
            .service(web_handlers::oauth2_gh::login_post)
            .service(web_handlers::oauth2_gh::auth)
            .service(web_handlers::oauth2_gh::logged_in)
            .service(web_handlers::oauth2_gh::logout)
            .service(web_handlers::static_files::static_gallery_file)
            .service(web_handlers::static_files::favicon)
            .service(web_handlers::static_files::const_js_modules)
            .service(web_handlers::static_files::index_page)
            .service(web_handlers::static_files::err_js_script)
            .service(web_handlers::static_files::static_root_file)
            .service(web_handlers::static_files::gridpaint_modules)
            .service(web_handlers::static_files::gridpaint_lib_modules)
            .service(web_handlers::api::resolve_moose)
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
