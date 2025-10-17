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

pub mod gallery;
// pub mod login;

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

pub fn navbar(username: Option<String>) -> Markup {
    let is_login = username.is_some();
    html! {
        .nav {
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
