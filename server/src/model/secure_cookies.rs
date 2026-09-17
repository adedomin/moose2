/* Copyright (C) 2025  Anthony DeDominic
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

use axum::extract::FromRequestParts;
use http::{StatusCode, request::Parts};
use tower_cookies::{Cookies, Key, PrivateCookies};

/// Wrapper around tower-cookies extension that makes sure the cookie is only used with an encryption key.
pub struct SecureCookies {
    inner: Cookies,
}

impl SecureCookies {
    /// get the inner `PrivateCookies` thus forcing the use of a private key.
    pub fn get_inner(self, key: &Key) -> PrivateCookies<'_> {
        self.inner.private(key)
    }
}

impl<S> FromRequestParts<S> for SecureCookies
where
    S: Send + Sync,
{
    type Rejection = (http::StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let Some(cookies) = parts.extensions.get::<Cookies>().cloned() else {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Can't extract cookies. Is tower-cookies applied!?",
            ));
        };
        Ok(SecureCookies { inner: cookies })
    }
}
