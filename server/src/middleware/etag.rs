use axum::{extract::Request, middleware::Next, response::IntoResponse};
use base64::{Engine, prelude::BASE64_STANDARD_NO_PAD};
use http::{
    StatusCode,
    header::{ETAG, IF_NONE_MATCH},
};
use sha2::Digest;

/// Hash a response body into a strong MD5-based ETag.
pub fn etag<T: AsRef<[u8]>>(body: T) -> String {
    let sum = sha2::Sha256::digest(body);
    let sum = BASE64_STANDARD_NO_PAD.encode(sum.as_slice());
    format!("\"{sum}\"")
}

pub async fn etag_match(req: Request, next: Next) -> impl IntoResponse {
    let etag = req.headers().get(IF_NONE_MATCH).map(|v| v.to_owned());
    let res = next.run(req).await;
    if let Some(e) = res.headers().get(ETAG) {
        if let Some(e2) = etag {
            if e == e2 {
                let (mut parts, _) = res.into_parts();
                parts.status = StatusCode::NOT_MODIFIED;
                return parts.into_response();
            }
        }
    }
    res
}
