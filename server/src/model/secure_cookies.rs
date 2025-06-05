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
