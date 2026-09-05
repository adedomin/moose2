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

use maud::{Markup, html};

use crate::model::author::Author;

pub mod gallery;
pub mod index;
pub mod login;

// TODO: add cache-busting query string to all static resources.
pub fn header(page_title: &str, css: &'static str) -> Markup {
    html! {
        head {
            meta charset="utf-8";
            meta name="description" content="Draw and Share Moose with your IRC friends.";
            meta name="viewport" content="width=device-width, initial-scale=1, shrink-to-fit=no";
            link rel="stylesheet" href="/public/root/common.css";
            link rel="stylesheet" href=(css);
            title { "Moose2 - " (page_title) }
        }
    }
}

// TODO: add cache-busting query string to all static resources.
pub fn script(script_src: &str) -> Markup {
    html! {
        script src=(script_src) type="module" {}
    }
}

enum NavPage {
    Index,
    Gallery,
    Login,
}

impl NavPage {
    fn selected(&self, idx: usize) -> bool {
        matches!((self, idx), (NavPage::Index, 0) | (NavPage::Gallery, 1))
    }
    fn onclick(&self, idx: usize) -> Option<&'static str> {
        self.selected(idx).then_some("return false")
    }
}

impl From<&str> for NavPage {
    fn from(url: &str) -> Self {
        match url {
            "/" => Self::Index,
            "/login" => Self::Login,
            _ => Self::Gallery,
        }
    }
}

pub fn navbar(redir_to: &str, auth: Author) -> Markup {
    let authlevel = auth.auth_level();
    let display = auth.displayable().unwrap_or_else(|| "Login".to_owned());
    let action_url = if authlevel > 0 { "/logout" } else { "/login" };
    let page = NavPage::from(redir_to);

    html! {
        .nav {
            .btn-grp {
                a.btn.selected[page.selected(0)] href="/"        onclick=[page.onclick(0)] { "Moose2" }
                a.btn.selected[page.selected(1)] href="/gallery" onclick=[page.onclick(1)] { "Gallery" }
            }
            @if !matches!(page, NavPage::Login)  {
                .btn-grp.float-right {
                    input.btn type="submit" form="log-inout-form" id="login" data-authlevel=(authlevel) value=(display);
                }
                form #log-inout-form action=(action_url) method="post" style="display: none;" {
                    input #lio-redir name="redirect" type="hidden" value=(redir_to);
                }
            }
        }
    }
}
