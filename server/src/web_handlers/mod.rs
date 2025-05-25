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

use std::{fmt::Display, sync::Arc};

use axum::response::{IntoResponse, Response};
use http::{HeaderName, HeaderValue, StatusCode, header::CONTENT_TYPE};
use serde::Serialize;
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

#[derive(Serialize, Debug)]
pub struct ApiError {
    #[serde(skip)]
    code: StatusCode,
    status: &'static str,
    msg: String,
}

const FALLBACK: &[u8] = br##"{ "status": "critical", "msg": "failed to serialize api message." }"##;

impl ApiError {
    pub fn new(msg: String) -> Self {
        ApiError {
            code: StatusCode::INTERNAL_SERVER_ERROR,
            status: "error",
            msg,
        }
    }

    pub fn new_ok(msg: String) -> Self {
        ApiError {
            code: StatusCode::OK,
            status: "ok",
            msg,
        }
    }

    pub fn new_with_status<T: Display>(code: StatusCode, msg: T) -> Self {
        ApiError {
            code,
            status: if code.is_success() { "ok" } else { "error" },
            msg: msg.to_string(),
        }
    }

    pub fn to_json(&self) -> Vec<u8> {
        match serde_json::to_vec(&self) {
            Ok(ok) => ok,
            Err(e) => {
                log::error!("Could not Serialize ApiError Struct: {self:?}, reason {e}");
                FALLBACK.to_owned()
            }
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        Response::builder()
            .status(self.code)
            .header(JSON_TYPE.0, JSON_TYPE.1)
            .body(self.to_json().into())
            .unwrap()
    }
}
