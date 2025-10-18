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

use crate::templates::{header, log_inout_form, navbar};
use maud::{DOCTYPE, Markup, html};

pub fn page_range(page: usize, page_count: usize) -> std::iter::Take<std::ops::Range<usize>> {
    let page_start = page.saturating_sub(5);

    let page_diff = page_start.abs_diff(page_count);
    let page_start = if page_diff < 10 {
        page_start.saturating_sub(10 - page_diff)
    } else {
        page_start
    };

    (page_start..page_count).take(10)
}

fn pager(page: usize, page_count: usize) -> Markup {
    html! {
        .nav-block {
            a .arrow-left         .disable[page == 0] href={"/gallery/" (&(page.saturating_sub(1)))} { "Prev" }
            a .paddle.paddle-edge .disable[page == 0] .paddle-edge href={"/gallery/0"} {
                span.full { "Oldest" br; "Page" }
                span.short { "Old" }
            }

            @for pnum in page_range(page, page_count) {
                a .paddle .selected[pnum == page] href={"/gallery/" (pnum)} { (pnum) }
            }

            a .paddle.paddle-edge .disable[page+1 >= page_count] href={"/gallery/" (&(page_count.saturating_sub(1)))} {
                span.full { "Newest" br; "Page" }
                span.short { "New" }
            }
            a .arrow-right        .disable[page+1 >= page_count] href={"/gallery/" (&(page       + 1))} { "Next" }
        }
    }
}

pub fn gallery(
    page_title: &str,
    page: usize,
    page_count: usize,
    username: Option<String>,
) -> Markup {
    let is_login = username.is_some();
    html! {
        (DOCTYPE)
        html lang="en" {
            (header(page_title, "/public/gallery/moose2.css"))
            body {
                (navbar(true, username))
                // we duplicate this top and bottom, might as well reuse it?
                @let pager_widget = pager(page, page_count);
                (pager_widget)
                form #search-form method="get" {
                    .full-width.btn-grp {
                        input      #search-field name="q"    type="text"   placeholder="Search Moose";
                        input                    name="nojs" type="hidden" value="true";
                        input .btn #submit                   type="submit" value="Search";
                    }
                }
                h1 #hidden-banner-error .center-banner .hidden { "No Moose!" }
                #moose-cards .cards {}
                (pager_widget)
                template #moose-card-template {
                    .card.center-me {
                        a .nil {
                            img .img;
                        }
                        br;
                        a .black-link {}
                    }
                }
                script src="/public/gallery/moose2.js" type="module" {}
                (log_inout_form(format!("/gallery/{page}").as_str(), is_login))
            }
        }
    }
}
