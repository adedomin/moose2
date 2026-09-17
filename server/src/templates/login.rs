/* Copyright (C) 2026  Anthony DeDominic
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

use maud::{DOCTYPE, Markup, html};

use crate::{
    model::{
        author::{Author, IRC_MAX_BYTE_LEN},
        secret::PASS_MAX_LEN,
    },
    templates::{header, navbar},
};

pub fn login_choice(user: Option<&str>, err_msg: Option<&'static str>) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            (header("Login", "/public/root/login.css"))
            body {
                .divider {
                    (navbar("/login", Author::Anonymous))
                    .center-me {
                        form method="post" action="/login/submit" {
                            fieldset {
                                input #user .block.full-width name="user" type="text" maxlength=(IRC_MAX_BYTE_LEN) placeholder="Username or Alias" value=(user.unwrap_or(""));
                                input #pass .block.full-width name="pass" type="password" maxlength=(PASS_MAX_LEN) placeholder="(Optional) Password";
                                input #change type="checkbox";
                                label for="change" { " Change Password" }
                                input #newpass .block.full-width name="newpass" type="password" maxlength="64" placeholder="New Password";
                                input .block.btn.full-width #submit type="submit" value="Submit";
                            }
                            @if let Some(err_msg) = err_msg {
                                p.err-text { (err_msg) }
                            } @else {
                                // empty space, prevent reflow
                                p { "\u{A0}" }
                            }
                        }
                    }
                }
            }
        }
    }
}
