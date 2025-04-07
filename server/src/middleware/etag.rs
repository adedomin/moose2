use axum::{extract::Request, middleware::Next, response::IntoResponse};
use base64::{Engine, prelude::BASE64_STANDARD_NO_PAD};
use http::{
    HeaderValue, StatusCode,
    header::{ETAG, IF_NONE_MATCH},
};

pub fn md5_etag<T: AsRef<[u8]>>(body: T) -> HeaderValue {
    let sum = md5::compute(body);
    let sum = BASE64_STANDARD_NO_PAD.encode(sum.as_slice());
    HeaderValue::try_from(format!("\"{sum}\"")).unwrap()
}

pub fn crc32_etag(body: &[u8]) -> HeaderValue {
    let sum = crc32fast::hash(body);
    HeaderValue::try_from(format!("\"{sum:08x}\"")).unwrap()
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
