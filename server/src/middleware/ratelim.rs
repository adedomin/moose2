use axum::{
    extract::{ConnectInfo, Request},
    response::{IntoResponse, Response},
};
use http::{StatusCode, header::RETRY_AFTER};
use pin_project::pin_project;
use std::{
    hash::{BuildHasher, RandomState},
    net::{IpAddr, SocketAddr},
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering::Relaxed},
    },
    task::{Context, Poll},
    time::{SystemTime, UNIX_EPOCH},
};
use tower::{Layer, Service};

use crate::{
    config::Ratelim,
    web_handlers::{ApiError, JSON_TYPE},
};

#[derive(Clone)]
struct BucketRateLimState {
    lim_secs: u64,
    trust_headers: bool,
    hasher: RandomState,
    buckets: Arc<Vec<AtomicU64>>,
}

#[derive(Clone)]
pub struct BucketRatelim {
    state: BucketRateLimState,
}

impl From<Ratelim> for BucketRatelim {
    fn from(rl: Ratelim) -> Self {
        Self {
            state: BucketRateLimState {
                lim_secs: rl.secs(),
                trust_headers: rl.trust_headers(),
                hasher: RandomState::new(),
                buckets: Arc::new((0..rl.bucket_size()).map(|_| AtomicU64::new(0)).collect()),
            },
        }
    }
}

#[derive(Clone)]
pub struct BucketRatelimMiddle<S> {
    inner: S,
    state: BucketRateLimState,
}

impl<S> Layer<S> for BucketRatelim {
    type Service = BucketRatelimMiddle<S>;

    fn layer(&self, inner: S) -> Self::Service {
        Self::Service {
            inner,
            state: self.state.clone(),
        }
    }
}

#[pin_project]
pub struct RlFuture<I> {
    #[pin]
    inner: RlFutType<I>,
}

#[pin_project(project = RlFutTypeProj)]
pub enum RlFutType<I> {
    Ok {
        #[pin]
        fut: I,
    },
    Exceeded {
        resp: Option<Response>,
    },
}

impl<I, E> Future for RlFuture<I>
where
    I: Future<Output = Result<Response, E>>,
{
    type Output = Result<Response, E>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.project().inner.project() {
            RlFutTypeProj::Ok { fut } => fut.poll(cx),
            RlFutTypeProj::Exceeded { resp } => Poll::Ready(Ok(resp
                .take()
                .expect("option used for take() out of projection."))),
        }
    }
}

fn check_lim(slot: &AtomicU64, lim_secs: u64) -> Result<(), u64> {
    // FIXME: unfortunately, wall-clock time is the only time we can get a convenient u64 from the stdlib.
    // please change this if there is a better monotonic source that can be repr as a u64.
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("To get the current time offset by UNIX Epoch.")
        .as_secs();
    slot.fetch_update(Relaxed, Relaxed, |time| {
        // if our clock went backards in time, this condition will almost certainly be true.
        if now.wrapping_sub(time) > lim_secs {
            Some(now)
        } else {
            None
        }
    })
    .map(|_| ())
    .map_err(|old| lim_secs - (now - old))
}

impl<S> Service<Request> for BucketRatelimMiddle<S>
where
    S: Service<Request, Response = Response> + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = RlFuture<S::Future>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request) -> Self::Future {
        let get_ip = || {
            request
                .extensions()
                .get::<ConnectInfo<SocketAddr>>()
                .map(|f| f.ip())
        };
        let ipu64 = if self.state.trust_headers {
            let ip = request
                .headers()
                .get("X-Real-IP")
                .and_then(|hv| hv.to_str().ok())
                .and_then(|hv| hv.parse().ok());
            if ip.is_none() {
                log::warn!(
                    "Something is wrong with your reverse proxy and the X-Real-IP header. Falling back."
                );
                get_ip()
            } else {
                ip
            }
        } else {
            get_ip()
        };

        let ippart = if let Some(ipu64) = ipu64 {
            match ipu64.to_canonical() {
                IpAddr::V4(ipv4) => u64::from(ipv4.to_bits()),
                // what should we do for subnets?
                // ip/48 is probably the most encompassing
                // ip/56 are common for some residential ISPs.
                // devices will be given a ip/64 for SLAAC at a minimum.
                IpAddr::V6(ipv6) => (ipv6.to_bits() >> 64) as u64,
            }
        } else {
            log::error!(
                "Your server is not set up correctly! Check that you're setting X-Real-IP."
            );
            return RlFuture {
                inner: RlFutType::Exceeded {
                    resp: Some(
                        ApiError::new_with_status(
                            StatusCode::INTERNAL_SERVER_ERROR,
                            "Invalid configuration or X-Real-IP header contains non-utf8 string or unparsable IP."
                        )
                        .into_response()
                    )
                }
            };
        };

        // NOTE:
        // We don't care about collisions. Collisions are almost a "feature" of this.
        // Worst case scenario, a random user will share a (likely stale) rate limit of another address.
        // If our service is so popular we have significant contention then we have other problems.
        let iphash = self.state.hasher.hash_one(ippart) as usize % self.state.buckets.len();
        if let Err(retry_after) = check_lim(&self.state.buckets[iphash], self.state.lim_secs) {
            RlFuture {
                inner: RlFutType::Exceeded {
                    resp: Some(
                        Response::builder()
                            .status(StatusCode::TOO_MANY_REQUESTS)
                            .header(JSON_TYPE.0, JSON_TYPE.1)
                            .header(RETRY_AFTER, retry_after.to_string())
                            .body(
                                ApiError::new(format!("Retry after {retry_after} seconds."))
                                    .to_json()
                                    .into(),
                            )
                            .unwrap(),
                    ),
                },
            }
        } else {
            RlFuture {
                inner: RlFutType::Ok {
                    fut: self.inner.call(request),
                },
            }
        }
    }
}
