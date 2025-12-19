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

use super::{HTML_TYPE, MooseWebData};
use crate::{
    db::MooseDB, middleware::etag::etag, model::author::Author, templates::gallery,
    web_handlers::ApiError,
};
use axum::{
    Router,
    extract::{Path, State},
    response::{IntoResponse, Redirect, Response},
    routing::get,
};
use http::{StatusCode, header::ETAG};
use rand::Rng;

async fn gallery_random_redir(State(db): State<MooseWebData>) -> Response {
    let db = &db.db;
    match db.get_page_count().await {
        Ok(page_count) => {
            if page_count == 0 {
                Redirect::to("/gallery/0").into_response()
            } else {
                let rand_idx = rand::thread_rng().r#gen_range(0..page_count);
                Redirect::to(&format!("/gallery/{rand_idx}")).into_response()
            }
        }
        Err(e) => {
            log::error!("DB: {e}");
            ApiError::new(e).into_response()
        }
    }
}

async fn gallery_latest_redir(State(db): State<MooseWebData>) -> Response {
    let db = &db.db;
    match db.get_page_count().await {
        Ok(page_count) => {
            Redirect::to(&format!("/gallery/{}", page_count.saturating_sub(1))).into_response()
        }
        Err(e) => {
            log::error!("DB: {e}");
            ApiError::new(e).into_response()
        }
    }
}

async fn gallery_page(
    State(db): State<MooseWebData>,
    Path(page): Path<usize>,
    username: Author,
) -> Response {
    let page_count = {
        let db = &db.db;
        db.get_page_count().await.unwrap_or(page)
    };
    let body = gallery::gallery(&format!("Page {page}"), page, page_count, username).into_string();

    Response::builder()
        .status(StatusCode::OK)
        .header(HTML_TYPE.0, HTML_TYPE.1)
        .header(ETAG, etag(&body))
        .body(body.into())
        .unwrap()
}

pub fn routes() -> Router<MooseWebData> {
    Router::new()
        .route("/gallery", get(Redirect::permanent("/gallery/0")))
        .route("/gallery/", get(Redirect::permanent("/gallery/0")))
        .route("/gallery/latest", get(gallery_latest_redir))
        .route("/gallery/random", get(gallery_random_redir))
        .route("/gallery/{page_id}", get(gallery_page))
}
