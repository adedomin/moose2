use crate::{
    moosedb::{Moose, MoosePage},
    templates::{header, moose_card, pager, search_bar},
};
use maud::{html, Markup, DOCTYPE};

pub fn gallery(page_title: &str, page: usize, page_count: usize, meese: MoosePage) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            (header(page_title))
            body {
                // we duplicate this top and bottom, might as well reuse it?
                @let p = pager(page, page_count);
                (p)
                .cards {
                    @for moose in meese.0 {
                        (moose_card(&moose.name, ""))
                    }
                }
                (p)
            }
        }
    }
}

pub fn nojs_search(page_title: &str, meese: Vec<(usize, &Moose)>) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            (header(page_title))
            body {
                (search_bar())
                .cards {
                    @for (page, moose) in meese {
                        (moose_card(&moose.name, &format!("/gallery/{}", page)))
                    }
                }
            }
        }
    }
}
