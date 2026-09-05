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
    model::author::Author,
    templates::{header, navbar, script},
};

fn modal() -> Markup {
    html! {
       #modal-backdrop.close.center-me {
           #modal.close {
               p #modal-title { "header" }
               button #modal-close { "×" }
               p #modal-content { "None" }
           }
       }
    }
}

fn painter_cntl() -> Markup {
    html! {
       .push {
           .btn-grp.full-width {
               input #name type="text" placeholder="Moose Name";
               button #save.btn { "Save" }
           }
       }
       #btn-tools.push {
           #painter-tools.btn-grp {
               button #pencil.btn.selected { "Pencil" }
               button #line  .btn          { "Line" }
               button #bucket.btn          { "Bucket" }
           }
           #painter-actions.btn-grp {
               button #undo.btn { "Undo" }
               button #redo.btn { "Redo" }
           }
           .btn-grp {
               button #hd   .btn                { "HD" }
               button #grid .btn.selected       { "Grid" }
               button #clear.btn.is-destructive { "Clear" }
           }
       }
       .push {
           #painter-palette {
               button.palette-btn style="opacity: 0" { "\u{A0}" }
           }
           #painter-palette-sub {
               button.palette-btn style="opacity: 0" { "\u{A0}" }
           }
       }
    }
}

pub fn index(username: Author) -> Markup {
    html! {
       (DOCTYPE)
       html lang="en" {
           (header("Client", "/public/root/index.css"))
           body {
               (modal())
               .divider {
                   (navbar("/", username))
                   .center-me {
                       #painter-frame {
                           #painter-area {
                               #painter-placeholder {}
                           }
                           #painter-controls {
                               (painter_cntl())
                           }
                       }
                   }
               }
               (script("/public/root/index.js"))
               (script("/public/root/loggedin.js"))
           }
       }
    }
}
