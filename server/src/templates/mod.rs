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

use maud::{html, Markup};
use percent_encoding::{percent_encode, NON_ALPHANUMERIC};

use crate::model::{moose::Moose, PIX_FMT_HEIGHT, PIX_FMT_WIDTH};

pub mod gallery;

pub fn header(page_title: &str) -> Markup {
    html! {
        head {
            meta charset="utf-8";
            meta name="description" content="Draw and Share Moose with your IRC friends.";
            meta name="viewport" content="width=device-width, initial-scale=1, shrink-to-fit=no";
            link rel="stylesheet" href="/public/gallery/moose2.css";
            title { "Moose2 - " (page_title) }
        }
    }
}

pub fn navbar(username: Option<String>) -> Markup {
    let is_login = username.is_some();
    html! {
        .nav-actual {
            .btn-grp {
                a.btn href="/" { "Moose2" }
                a.btn.selected href="/gallery" onclick="return false" { "Gallery" }
            }
            .btn-grp.float-right {
                input.btn type="submit" form="log-inout-form" id="login" data-login=(is_login) value=(username.unwrap_or("Login".to_owned()));
            }
        }
    }
}

pub fn log_inout_form(redir_to: &str, is_login: bool) -> Markup {
    let action_url = if is_login { "/logout" } else { "/login" };
    html! {
        form #log-inout-form action=(action_url) method="post" style="display: none;" {
            input #lio-redir name="redirect" type="hidden" value=(redir_to);
        }
    }
}

pub fn ebanner(is_empty: bool) -> Markup {
    html!(h1 #hidden-banner-error .center-banner .hidden[!is_empty] { "No Moose!" })
}

pub fn page_range(page: usize, page_count: usize) -> std::iter::Take<std::ops::Range<usize>> {
    let page_start_range = page.saturating_sub(5);

    let page_start_range = if page_start_range.abs_diff(page_count) < 10 {
        page_start_range.saturating_sub(10 - page_start_range.abs_diff(page_count))
    } else {
        page_start_range
    };

    (page_start_range..page_count).take(10)
}

const NOJS_STR: &str = "?nojs=true";

pub fn pager(page: usize, page_count: usize, disabled: bool, nojs: bool) -> Markup {
    let njs = if nojs { NOJS_STR } else { "" };
    html! {
        .nav-block .disable[disabled] {
            a .arrow-left         .disable[page == 0] href={"/gallery/" (&(page.saturating_sub(1))) (njs)} { "Prev" }
            a .paddle.paddle-edge .disable[page == 0] .paddle-edge href={"/gallery/0" (njs)} {
                span.full { "Oldest" br; "Page" }
                span.short { "Old" }
            }

            @for pnum in page_range(page, page_count) {
                a .paddle .selected[pnum == page] href={"/gallery/" (pnum) (njs)} { (pnum) }
            }

            a .paddle.paddle-edge .disable[page+1 >= page_count] href={"/gallery/" (&(page_count.saturating_sub(1))) (njs)} {
                span.full { "Newest" br; "Page" }
                span.short { "New" }
            }
            a .arrow-right        .disable[page+1 >= page_count] href={"/gallery/" (&(page       + 1)) (njs)} { "Next" }
        }
    }
}

/// the moose HTML card to display, only need the name.
pub fn moose_card(moose: &Moose, href_pre: &str) -> Markup {
    let moose_name = &moose.name;
    let (pix_w, pix_h, _) = moose.dimensions.width_height();
    let pix_w = pix_w * PIX_FMT_WIDTH;
    let pix_h = pix_h * PIX_FMT_HEIGHT;
    let moose_enc = percent_encode(moose_name.as_bytes(), NON_ALPHANUMERIC);
    html! {
       #{"-m-" (moose_enc)} .card {
            a href={"/img/" (moose_enc)} {
                img .img width=(pix_w) height=(pix_h) src={"/img/" (moose_enc)};
            }
            br;
            a .black-link href={(href_pre) "#-m-" (moose_enc)} { (moose_name) }
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
        form #search-form method="get" {
            .full-width {
                input #search-field name="q"    type="text"   placeholder="Search Moose";
                input               name="nojs" type="hidden" value="true";
                input #submit                   type="submit" value="Search";
            }
        }
        // script type="module" src="/public/gallery/moose2.js" {}
    }
}
