use crate::{
    model::{moose::Moose, other::MooseSearch},
    templates::{ebanner, header, moose_card, moose_card_template, pager, search_bar},
};
use maud::{html, Markup, DOCTYPE};

pub fn gallery(page_title: &str, page: usize, page_count: usize, meese: Vec<Moose>) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            (header(page_title))
            body {
                // we duplicate this top and bottom, might as well reuse it?
                @let p = pager(page, page_count);
                (p)
                (search_bar())
                (ebanner(meese.is_empty()))
                #moose-cards .cards {
                    @for moose in meese {
                        noscript {
                            (moose_card(&moose.name, ""))
                        }
                    }
                }
                (p)
                (moose_card_template())
            }
        }
    }
}

pub fn nojs_search(page_title: &str, meese: Vec<MooseSearch>) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            (header(page_title))
            body {
                (search_bar())
                (ebanner(meese.is_empty()))
                #moose-cards .cards {
                    @for MooseSearch { moose, page } in meese {
                        (moose_card(&moose.name, &format!("/gallery/{}", page)))
                    }
                }
                (moose_card_template())
            }
        }
    }
}
