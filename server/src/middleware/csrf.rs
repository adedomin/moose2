use axum::{extract::Request, middleware::Next, response::IntoResponse};
use http::{
    HeaderMap, StatusCode,
    header::{HOST, ORIGIN},
};

use crate::web_handlers::ApiError;

const SEC_FETCH_SITE: &str = "Sec-Fetch-Site";
const SEC_FETCH_SITE_ALLOWED: &str = "same-origin";

/// Checks if Origin's schemaless value matches the Host header.
/// Any of the headers being missing is an automatic pass because it's assumed it is a weird custom client,
/// such as a phone app or a curl user.
fn origin_check(headers: &HeaderMap) -> Option<bool> {
    let origin = headers.get(ORIGIN).and_then(|v| v.to_str().ok())?;
    // Origin: <scheme>://<host>:<port>
    let (_, origin) = origin.split_once("://")?;
    // Host: <host>:<port>
    let host = headers.get(HOST).and_then(|v| v.to_str().ok())?;
    Some(origin == host)
}

/// Check if Sec-Fetch-Site is set and reject all non same-origin requests.
/// Any of the headers being missing is an automatic pass because it's assumed it is a weird custom client,
/// such as a phone app or a curl user.
fn sec_fetch_site_check(headers: &HeaderMap) -> Option<bool> {
    headers
        .get(SEC_FETCH_SITE)
        .and_then(|v| v.to_str().ok())
        .map(|v| v == SEC_FETCH_SITE_ALLOWED)
}

pub async fn header_csrf(req: Request, next: Next) -> Result<impl IntoResponse, ApiError> {
    if !req.method().is_safe() {
        let headers = req.headers();
        if !sec_fetch_site_check(headers).unwrap_or_else(|| origin_check(headers).unwrap_or(true)) {
            return Err(ApiError::new_with_status(
                StatusCode::FORBIDDEN,
                "CSRF failure.",
            ));
        }
    }
    Ok(next.run(req).await)
}
