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

use std::sync::Arc;

use http::{HeaderName, HeaderValue, header::CONTENT_TYPE};
use tower_cookies::{Cookies, Key};

use crate::model::{app_data::AppData, author::Author};

pub mod api;
pub mod display;
pub mod oauth2_gh;
pub mod static_files;

pub type MooseWebData = Arc<AppData>;

pub const JSON_TYPE: (HeaderName, HeaderValue) = (
    CONTENT_TYPE,
    HeaderValue::from_static("application/json; charset=utf-8"),
);
pub const HTML_TYPE: (HeaderName, HeaderValue) = (
    CONTENT_TYPE,
    HeaderValue::from_static("text/html; charset=utf-8"),
);
pub const HOUR_CACHE: HeaderValue = HeaderValue::from_static("max-age=3600, stale-if-error=3600");
pub const LOGIN_COOKIE: &str = "login";
pub const CSRF_COOKIE: &str = "csrf";
pub const REDIR_COOKIE: &str = "redirect";

pub fn get_login(c: &Cookies, k: &Key) -> Option<Author> {
    c.private(k)
        .get(LOGIN_COOKIE)
        .and_then(|c| serde_json::from_str::<Author>(c.value()).ok())
}
