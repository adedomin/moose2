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

use super::MooseWebData;
use crate::{
    db::MooseDB,
    model::{
        author::Author,
        pages::{MooseSearch, MooseSearchPage},
        queries::SearchQuery,
    },
    templates::gallery,
};
use actix_session::Session;
use actix_web::{
    get,
    http::{header::LOCATION, StatusCode},
    web, HttpResponse,
};
use rand::Rng;

#[get("/gallery")]
pub async fn gallery_redir() -> HttpResponse {
    HttpResponse::Ok()
        .insert_header((LOCATION, "/gallery/0"))
        .status(StatusCode::SEE_OTHER)
        .body(())
}

#[get("/gallery/random")]
pub async fn gallery_random_redir(db: MooseWebData) -> HttpResponse {
    let db = &db.db;
    match db.get_page_count().await {
        Ok(page_count) => {
            if page_count == 0 {
                HttpResponse::Ok()
                    .insert_header((LOCATION, "/gallery/0"))
                    .status(StatusCode::SEE_OTHER)
                    .body(())
            } else {
                let rand_idx = rand::thread_rng().gen_range(0..page_count);
                HttpResponse::Ok()
                    .insert_header((LOCATION, format!("/gallery/{}", rand_idx)))
                    .status(StatusCode::SEE_OTHER)
                    .body(())
            }
        }
        Err(e) => {
            eprintln!("{}", e);
            HttpResponse::Ok()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(e.to_string())
        }
    }
}

#[get("/gallery/latest")]
pub async fn gallery_latest_redir(db: MooseWebData) -> HttpResponse {
    let db = &db.db;
    match db.get_page_count().await {
        Ok(page_count) => HttpResponse::Ok()
            .insert_header((
                LOCATION,
                format!("/gallery/{}", page_count.saturating_sub(1)).as_str(),
            ))
            .status(StatusCode::SEE_OTHER)
            .body(()),
        Err(e) => {
            eprintln!("{}", e);
            HttpResponse::Ok()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(e.to_string())
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
) -> HttpResponse {
    let db = &db.db;
    let meese = db
        .search_moose(query, search_page)
        .await
        .unwrap_or_else(|err| {
            eprintln!("{}", err);
            MooseSearchPage::default()
        });

    let html = gallery::gallery(
        &format!("Search: {query}"),
        page_num,
        db.get_page_count().await.unwrap_or(page_num),
        Some(meese.result),
        true,
        nojs,
        username,
    )
    .into_string();
    HttpResponse::Ok()
        .insert_header(("Content-Type", "text/html"))
        .body(html)
}

async fn normal_gallery_page(
    db: MooseWebData,
    page_num: usize,
    nojs: bool,
    username: Option<String>,
) -> HttpResponse {
    let db = &db.db;
    let meese = if nojs {
        let meese = db.get_moose_page(page_num).await.unwrap_or_else(|err| {
            eprintln!("{}", err);
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
    let html = gallery::gallery(
        &format!("Page {}", page_num),
        page_num,
        db.get_page_count().await.unwrap_or(page_num),
        meese,
        false,
        nojs,
        username,
    )
    .into_string();
    HttpResponse::Ok()
        .insert_header(("Content-Type", "text/html"))
        .body(html)
}

#[get("/gallery/{page_id}")]
async fn gallery_page(
    db: MooseWebData,
    session: Session,
    page_id: web::Path<usize>,
    query: web::Query<SearchQuery>,
) -> HttpResponse {
    let page_num = page_id.into_inner();
    let SearchQuery { query, page, nojs } = query.into_inner();

    let username = session
        .get::<Author>("login")
        .unwrap_or_default()
        .and_then(|author| author.try_into().ok());

    if !query.is_empty() && nojs {
        nojs_gallery_search(db, page_num, &query, page, nojs, username).await
    } else {
        normal_gallery_page(db, page_num, nojs, username).await
    }
}
