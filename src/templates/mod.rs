use maud::{html, Markup};
use rouille::percent_encoding::{percent_encode, NON_ALPHANUMERIC};

pub mod gallery;

pub fn header(page_title: &str) -> Markup {
    html! {
        head {
            meta charset="utf-8";
            meta name="description" content="Draw and Share Moose with your IRC friends.";
            link rel="stylesheet" href="/public/moose2.css";
            title { "Moose2 - " (page_title) }
        }
    }
}

pub fn ebanner(is_empty: bool) -> Markup {
    html!(h1 #hidden-banner-error .center-banner .hidden[!is_empty] { "No Moose!" })
}

pub fn pager(page: usize, page_count: usize) -> Markup {
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

    html! {
        .nav-block {
            @if page != 0 {
                 a .arrow-left href={"/gallery/" (&(page - 1))} { "Prev" }
                 a .paddle     href="/gallery/0"             { "Oldest" br; "Page" }
            }

            @for pnum in page_range {
                a .paddle .selected[pnum == page] href={"/gallery/" (pnum)} { (pnum) }
            }

           @if page+1 < page_count {
               a .paddle      href={"/gallery/" (&(page_count - 1))} { "Newest" br; "Page"}
               a .arrow-right href={"/gallery/" (&(page       + 1))} { "Next" }
           }
        }
    }
}

/// the moose HTML card to display, only need the name.
pub fn moose_card(moose: &str, href_pre: &str) -> Markup {
    let moose_enc = percent_encode(moose.as_bytes(), NON_ALPHANUMERIC);
    html! {
       #{"-m-" (moose_enc)} .card {
            a href={"/img/" (moose_enc)} {
                img .img src={"/img/" (moose_enc)};
            }
            br;
            a .black-link href={(href_pre) "#-m-" (moose_enc)} { (moose) }
        }
    }
}

pub fn moose_card_template() -> Markup {
    html! {
        template #moose-card-template {
            .card {
                a .nil {
                    img .img;
                }
                br;
                a .black-link {}
            }
        }
    }
}

pub fn search_bar() -> Markup {
    html! {
        form #search-form method="get" action="/gallery/nojs-search" {
            .full-width {
                input #search-field name="q" type="text"   placeholder="Search Moose";
                input #submit                type="submit" value="Search";
            }
        }
        script defer src="/public/moose2.js" {}
    }
}
