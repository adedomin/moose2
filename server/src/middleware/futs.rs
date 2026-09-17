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

use std::{
    pin::Pin,
    task::{Context, Poll},
};

use axum::response::Response;
use pin_project_lite::pin_project;

pin_project! {
    pub struct EarlyRetFut<I> {
        #[pin]
        inner: EarlyRetFutType<I>,
    }
}

pin_project! {
    #[project = EarlyRetFutTypeProj]
    pub enum EarlyRetFutType<I> {
        Next {
            #[pin]
            fut: I,
        },
        Early {
            resp: Option<Response>,
        },
    }
}

impl<I> EarlyRetFut<I> {
    pub fn new_early(resp: Response) -> Self {
        Self {
            inner: EarlyRetFutType::Early { resp: Some(resp) },
        }
    }

    pub fn new_next(fut: I) -> Self {
        Self {
            inner: EarlyRetFutType::Next { fut },
        }
    }
}

impl<I, E> Future for EarlyRetFut<I>
where
    I: Future<Output = Result<Response, E>>,
{
    type Output = Result<Response, E>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.project().inner.project() {
            EarlyRetFutTypeProj::Next { fut } => fut.poll(cx),
            EarlyRetFutTypeProj::Early { resp } => Poll::Ready(Ok(resp
                .take()
                .expect("option used for take() out of projection."))),
        }
    }
}
