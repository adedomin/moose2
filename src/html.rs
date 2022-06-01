use crate::moosedb::MoosePage;
use askama::Template;

#[derive(Template)]
#[template(path = "gallery.html")]
pub struct GalleryTemplate<'m> {
    title: &'m str,
    page: usize,
    page_max: usize,
    meese: Vec<&'m str>,
}

pub fn gallery_page(moose_page: MoosePage, page: usize, page_max: usize) -> String {
    let mut meese = Vec::with_capacity(12);
    for moose in moose_page.0 {
        meese.push(moose.name.as_str());
    }
    let title = format!("Page {}", page);
    (GalleryTemplate {
        title: &title,
        page,
        page_max,
        meese,
    })
    .render()
    .unwrap()
}
