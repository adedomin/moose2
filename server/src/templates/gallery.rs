/* Copyright (C) 2024  Anthony DeDominic
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use crate::{
    model::pages::MooseSearch,
    templates::{
        ebanner, header, log_inout_form, moose_card, moose_card_template, navbar, pager, search_bar,
    },
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
    let njs = if nojs { "?nojs=true" } else { "" };
    let is_login = username.is_some();
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
