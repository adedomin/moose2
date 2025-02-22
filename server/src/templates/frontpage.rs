use maud::{DOCTYPE, Markup, html};

use crate::templates::{header, navbar};

pub fn frontpage(username: Option<String>) -> Markup {
    let is_login = username.is_some();
    html! {
        (DOCTYPE)
        html lang="en" {
            (header("Client", "/public/root/index.css"))
            body {
                (navbar(username))
                // we duplicate this top and bottom, might as well reuse it?
                @let pager_widget = pager(page, page_count, search, nojs);
                (pager_widget)
                (search_bar())
                (ebanner(meese.as_ref().map(|meese| meese.is_empty()).unwrap_or(false)))
                #moose-cards .cards {
                    @if let Some(meese) = meese {
                        @for MooseSearch { moose, page } in meese {
                            (moose_card(&moose, format!("/gallery/{page}{njs}").as_str()))
                        }
                    }
                }
                (pager_widget)
                @if !nojs {
                    (moose_card_template())
                    script src="/public/global-modules/err.js" {}
                    script src="/public/gallery/moose2.js" type="module" {}
                }
                (log_inout_form(format!("/gallery/{page}{njs}").as_str(), is_login))
            }
        }
    }
}
