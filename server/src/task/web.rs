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

use std::sync::Arc;

use axum::{Router, middleware};
use oauth2::{AuthUrl, ClientId, ClientSecret, RedirectUrl, TokenUrl, basic::BasicClient};
use tokio::{
    net::{TcpListener, UnixListener},
    sync::broadcast::Receiver,
    task::JoinHandle,
};
use tower::ServiceBuilder;
use tower_cookies::{CookieManagerLayer, Key};

use crate::{
    config::{GitHubOauth2, RunConfig},
    db::sqlite3_impl::Pool,
    middleware::etag::etag_match,
    model::app_data::{AppData, Oa},
    web_handlers::{api, display, oauth2_gh, static_files},
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
    let app_data = Arc::new(AppData {
        db,
        cookie_key: Key::from(&rc.cookie_key.0),
        oauth2_client,
    });
    let moose_dump = rc.get_moose_dump();

    let app = Router::new()
        .merge(api::routes())
        .merge(api::dump_route(moose_dump))
        .merge(oauth2_gh::routes())
        .merge(display::routes())
        .merge(static_files::routes())
        .layer(
            ServiceBuilder::new()
                .layer(CookieManagerLayer::new())
                .layer(middleware::from_fn(etag_match)),
        )
        .with_state(app_data);

    tokio::spawn(async move {
        let shutdown_h = async move {
            shutdown_signal.recv().await.unwrap();
        };
        if let Some(path) = listen_addr.strip_prefix("unix:") {
            let uds = UnixListener::bind(path).unwrap();
            axum::serve(uds, app)
                .with_graceful_shutdown(shutdown_h)
                .await
        } else {
            let inet = TcpListener::bind(listen_addr).await.unwrap();
            axum::serve(inet, app)
                .with_graceful_shutdown(shutdown_h)
                .await
        }
    })
}
