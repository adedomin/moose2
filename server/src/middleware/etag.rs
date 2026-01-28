use std::{
    pin::Pin,
    task::{Context, Poll},
};

use axum::{
    extract::Request,
    response::{IntoResponse, Response},
};
use base64::{Engine, prelude::BASE64_STANDARD_NO_PAD};
use http::{
    HeaderValue, StatusCode,
    header::{ETAG, IF_NONE_MATCH},
};
use pin_project_lite::pin_project;
use sha2::Digest;
use tower::{Layer, Service};

/// Hash a response body into a strong MD5-based ETag.
pub fn etag<T: AsRef<[u8]>>(body: T) -> String {
    let sum = sha2::Sha256::digest(body);
    let sum = BASE64_STANDARD_NO_PAD.encode(sum.as_slice());
    format!("\"{sum}\"")
}

fn etag_match(etag: &Option<HeaderValue>, res: Response) -> Response {
    if let Some(e) = res.headers().get(ETAG)
        && let Some(e2) = etag
        && e == e2
    {
        let (mut parts, _) = res.into_parts();
        parts.status = StatusCode::NOT_MODIFIED;
        return parts.into_response();
    }
    res
}

pin_project! {
    pub struct EtagFut<I> {
        etag: Option<HeaderValue>,
        #[pin]
        inner: I
    }
}

impl<I, E> Future for EtagFut<I>
where
    I: Future<Output = Result<Response, E>>,
{
    type Output = Result<Response, E>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        match this.inner.poll(cx) {
            Poll::Ready(Ok(res)) => Poll::Ready(Ok(etag_match(this.etag, res))),
            other => other,
        }
    }
}

/// A Tower Layer that 304s responses and removes their body if etags match.
#[derive(Clone)]
pub struct EtagLayer;

/// A Tower Service that 304s responses and removes their body if etags match.
#[derive(Clone)]
pub struct EtagService<S> {
    inner: S,
}

impl<S> Layer<S> for EtagLayer {
    type Service = EtagService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        Self::Service { inner }
    }
}

impl<S> Service<Request> for EtagService<S>
where
    S: Service<Request, Response = Response> + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = EtagFut<S::Future>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request) -> Self::Future {
        let etag = req.headers().get(IF_NONE_MATCH).map(|v| v.to_owned());
        let inner = self.inner.call(req);
        EtagFut { etag, inner }
    }
}
