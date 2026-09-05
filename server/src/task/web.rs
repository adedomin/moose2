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

use std::{net::SocketAddr, sync::Arc};

use axum::{Router, extract::DefaultBodyLimit};
#[cfg(unix)]
use tokio::net::UnixListener;
use tokio::{net::TcpListener, task::JoinHandle};
use tokio_util::sync::CancellationToken;
use tower::ServiceBuilder;
use tower_cookies::CookieManagerLayer;

use crate::{
    config::RunConfig,
    db::sqlite3_impl::Pool,
    middleware::{csrf::HeaderCsrf, etag::EtagLayer},
    model::app_data::AppData,
    web_handlers::{api, auth, display, static_files},
};

pub fn web_task(
    rc: RunConfig,
    db: Pool,
    stop_token: CancellationToken,
) -> JoinHandle<Result<(), std::io::Error>> {
    let listen_addr = rc.get_bind_addr();
    log::info!("Attempting to listen on: http://{listen_addr}/");
    let moose_dump = rc.get_moose_dump();
    let app_data = Arc::new(AppData {
        db,
        cookie_key: rc.cookie_key.into(),
        invite_hash: rc.invite_hash,
    });

    let app = Router::new()
        .merge(api::routes(rc.ratelim))
        .merge(api::dump_route(moose_dump))
        .merge(auth::routes())
        .merge(display::routes())
        .merge(static_files::routes())
        .layer(
            ServiceBuilder::new()
                // 16 KiB. Default 2 MiB is too large.
                .layer(DefaultBodyLimit::max(16 * 1024))
                .layer(HeaderCsrf)
                .layer(CookieManagerLayer::new())
                .layer(EtagLayer),
        )
        .with_state(app_data);

    tokio::spawn(async move {
        let stop_clone = stop_token.clone();
        // if the web server returns earlier than expected, make sure we cancel.
        let _dropped = stop_token.drop_guard();
        let shutdown_h = async move {
            _ = stop_clone.cancelled().await;
            log::warn!("Web Task is shutting down.")
        };
        #[cfg(unix)]
        if let Some(path) = listen_addr.strip_prefix("unix:") {
            let uds = UnixListener::bind(path).unwrap();
            axum::serve(uds, app)
                .with_graceful_shutdown(shutdown_h)
                .await
        } else {
            let inet = TcpListener::bind(listen_addr).await.unwrap();
            axum::serve(
                inet,
                app.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .with_graceful_shutdown(shutdown_h)
            .await
        }
        #[cfg(not(unix))]
        {
            let inet = TcpListener::bind(listen_addr).await.unwrap();
            axum::serve(
                inet,
                app.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .with_graceful_shutdown(shutdown_h)
            .await
        }
    })
}
