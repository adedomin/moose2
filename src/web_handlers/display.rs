use super::SearchQuery;
use crate::{moosedb::MooseDb, templates::gallery};
use actix_web::{
    get,
    http::{header::LOCATION, StatusCode},
    web, HttpResponse,
};
use rand::Rng;
use std::sync::RwLock;

#[get("/gallery")]
pub async fn gallery_redir() -> HttpResponse {
    HttpResponse::Ok()
        .insert_header((LOCATION, "/gallery/0"))
        .status(StatusCode::SEE_OTHER)
        .body(())
}

#[get("/gallery/random")]
pub async fn gallery_random_redir(db: web::Data<RwLock<MooseDb>>) -> HttpResponse {
    let page_count = { db.read().unwrap().page_count() };
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

#[get("/gallery/nojs-search")]
pub async fn nojs_gallery_search(
    db: web::Data<RwLock<MooseDb>>,
    query: web::Query<SearchQuery>,
) -> HttpResponse {
    let db = db.read().unwrap();
    let meese = db.find_page_with_link(&query.query);
    let html = gallery::nojs_search(&query.query, meese).into_string();
    HttpResponse::Ok()
        .insert_header(("Content-Type", "text/html"))
        .body(html)
}

#[get("/gallery/{page_id}")]
pub async fn gallery_page(
    db: web::Data<RwLock<MooseDb>>,
    page_id: web::Path<usize>,
) -> HttpResponse {
    let db_locked = db.read().unwrap();
    let pid = page_id.into_inner();
    let meese = db_locked.get_page(pid);
    let html = gallery::gallery(&format!("Page {}", pid), pid, db_locked.page_count(), meese)
        .into_string();
    HttpResponse::Ok()
        .insert_header(("Content-Type", "text/html"))
        .body(html)
}
