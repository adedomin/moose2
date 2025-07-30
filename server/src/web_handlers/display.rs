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
    db::MooseDB,
    middleware::etag::etag,
    model::{
        author::Author,
        pages::{MooseSearch, MooseSearchPage},
        queries::SearchQuery,
    },
    templates::gallery,
    web_handlers::ApiError,
};
use axum::{
    Router,
    extract::{Path, Query, State},
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
            ApiError::new(e.to_string()).into_response()
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
            ApiError::new(e.to_string()).into_response()
        }
    }
}

async fn nojs_gallery_search(
    db: MooseWebData,
    page_num: usize,
    query: &str,
    search_page: usize,
    nojs: bool,
    username: Option<String>,
) -> String {
    let db = &db.db;
    let meese = db
        .search_moose(query, search_page)
        .await
        .unwrap_or_else(|err| {
            log::error!("DB: {err}");
            MooseSearchPage::default()
        });

    gallery::gallery(
        &format!("Search: {query}"),
        page_num,
        db.get_page_count().await.unwrap_or(page_num),
        Some(meese.result),
        true,
        nojs,
        username,
    )
    .into_string()
}

async fn normal_gallery_page(
    db: MooseWebData,
    page_num: usize,
    nojs: bool,
    username: Option<String>,
) -> String {
    let db = &db.db;
    let meese = if nojs {
        let meese = db.get_moose_page(page_num).await.unwrap_or_else(|err| {
            log::error!("DB: {err}");
            vec![]
        });
        Some(
            meese
                .into_iter()
                .map(|moose| MooseSearch {
                    page: page_num,
                    moose,
                })
                .collect::<Vec<MooseSearch>>(),
        )
    } else {
        None
    };
    gallery::gallery(
        &format!("Page {page_num}"),
        page_num,
        db.get_page_count().await.unwrap_or(page_num),
        meese,
        false,
        nojs,
        username,
    )
    .into_string()
}

async fn gallery_page(
    State(db): State<MooseWebData>,
    Path(page_num): Path<usize>,
    username: Author,
    Query(query): Query<SearchQuery>,
) -> Response {
    let SearchQuery { query, page, nojs } = query;
    let username = match username {
        Author::Anonymous => None,
        Author::Oauth2(s) => Some(s),
    };

    let body = if !query.is_empty() && nojs {
        nojs_gallery_search(db, page_num, &query, page, nojs, username).await
    } else {
        normal_gallery_page(db, page_num, nojs, username).await
    };

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
