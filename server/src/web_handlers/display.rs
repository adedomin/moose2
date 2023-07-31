use super::{MooseWebData, SearchQuery};
use crate::{db::MooseDB, model::other::MooseSearchPage, templates::gallery};
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

#[get("/gallery/nojs-search")]
pub async fn nojs_gallery_search(db: MooseWebData, query: web::Query<SearchQuery>) -> HttpResponse {
    let db = &db.db;
    let SearchQuery { query, page } = query.into_inner();
    let meese = db.search_moose(&query, page).await.unwrap_or_else(|err| {
        eprintln!("{}", err);
        MooseSearchPage::default()
    });
    let html = gallery::nojs_search(&query, meese.result).into_string();
    HttpResponse::Ok()
        .insert_header(("Content-Type", "text/html"))
        .body(html)
}

#[get("/gallery/{page_id}")]
pub async fn gallery_page(db: MooseWebData, page_id: web::Path<usize>) -> HttpResponse {
    let db = &db.db;
    let page_num = page_id.into_inner();
    let meese = db.get_moose_page(page_num).await.unwrap_or_else(|err| {
        eprintln!("{}", err);
        vec![]
    });
    let html = gallery::gallery(
        &format!("Page {}", page_num),
        page_num,
        db.get_page_count().await.unwrap_or(page_num),
        meese,
    )
    .into_string();
    HttpResponse::Ok()
        .insert_header(("Content-Type", "text/html"))
        .body(html)
}
