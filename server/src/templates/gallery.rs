use crate::{
    model::pages::MooseSearch,
    templates::{ebanner, header, moose_card, moose_card_template, navbar, pager, search_bar},
};
use maud::{html, Markup, DOCTYPE};

pub fn gallery(
    page_title: &str,
    page: usize,
    page_count: usize,
    meese: Option<Vec<MooseSearch>>,
    search: bool,
    nojs: bool,
    username: Option<String>,
) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            (header(page_title))
            body {
                (navbar(username))
                // we duplicate this top and bottom, might as well reuse it?
                @let pager_widget = pager(page, page_count, search, nojs);
                (pager_widget)
                (search_bar())
                (ebanner(meese.as_ref().map(|meese| meese.is_empty()).unwrap_or(false)))
                #moose-cards .cards {
                    @if let Some(meese) = meese {
                        @let njs = if nojs { "?nojs=true" } else { "" };
                        @for MooseSearch { moose, page } in meese {
                            (moose_card(&moose, format!("/gallery/{page}{njs}").as_str()))
                        }
                    }
                }
                (pager_widget)
                (moose_card_template())
                @if !nojs {
                    script src="/public/global-modules/err.js" {}
                    script src="/public/gallery/moose2.js" type="module" {}
                }
            }
        }
    }
}
