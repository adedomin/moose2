use crate::{
    moosedb::{Moose, MoosePage},
    templates::{ebanner, header, moose_card, pager, search_bar, moose_card_template},
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
                (search_bar())
                (ebanner(meese.0.is_empty()))
                #moose-cards .cards {
                    @for moose in meese.0 {
                        (moose_card(&moose.name, ""))
                    }
                }
                (p)
                (moose_card_template())
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
                (ebanner(meese.is_empty()))
                #moose-cards .cards {
                    @for (page, moose) in meese {
                        (moose_card(&moose.name, &format!("/gallery/{}", page)))
                    }
                }
            }
        }
    }
}
