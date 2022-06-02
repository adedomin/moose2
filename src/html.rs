use crate::moosedb::MoosePage;
use askama::Template;

#[derive(Template)]
#[template(path = "gallery.html")]
pub struct GalleryTemplate<'m> {
    title: &'m str,
    page: usize,
    page_count: usize,
    page_range: Vec<usize>,
    meese: Vec<&'m str>,
}

pub fn gallery_page(moose_page: MoosePage, page: usize, page_count: usize) -> String {
    let mut meese = Vec::with_capacity(12);
    for moose in moose_page.0 {
        meese.push(moose.name.as_str());
    }
    let title = format!("Page {}", page);
    let page_start_range = if let Some(start) = page.checked_sub(5) {
        start
    } else {
        0
    };

    let page_start_range = if page_start_range.abs_diff(page_count) < 10 {
        if let Some(start) =
            page_start_range.checked_sub(10 - page_start_range.abs_diff(page_count))
        {
            start
        } else {
            0
        }
    } else {
        page_start_range
    };

    let page_range = (page_start_range..page_count)
        .take(10)
        .collect::<Vec<usize>>();

    (GalleryTemplate {
        title: &title,
        page,
        page_count,
        page_range,
        meese,
    })
    .render()
    .unwrap()
}
