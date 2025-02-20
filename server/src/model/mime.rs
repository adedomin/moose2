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

pub const MIME: [(&str, &str); 5] = [
    ("css", "text/css; charset=utf-8"),
    ("html", "text/html; charset=utf-8"),
    ("js", "application/javascript; charset=utf-8"),
    ("wasm", "application/wasm"),
    ("ico", "image/x-icon"),
];

pub fn get_mime(ext: &str) -> &'static str {
    match MIME.iter().find(|(e, _)| ext == *e) {
        Some((_, t)) => t,
        None => "application/octet-string",
    }
}
