// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Driver-local [`AsyncRuntime`] adapter for tokio.
//!
//! Used as the default runtime by [`CosmosDriverRuntimeBuilder`](super::CosmosDriverRuntimeBuilder)
//! when no override is supplied via `with_async_runtime`. The driver
//! deliberately constructs this adapter directly rather than going through
//! `azure_core::async_runtime::get_async_runtime` so the default runtime
//! is decoupled from process-wide global state.

use azure_core::async_runtime::{AbortableTask, AsyncRuntime, SpawnedTask, TaskFuture};
use azure_core::time::Duration;
use std::{
    error::Error,
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

#[derive(Debug, Default)]
pub(crate) struct TokioRuntime;

impl AsyncRuntime for TokioRuntime {
    fn spawn(&self, f: TaskFuture) -> SpawnedTask {
        let handle = ::tokio::spawn(f);
        Box::pin(JoinHandle {
            handle: Some(handle),
        })
    }

    fn sleep(&self, duration: Duration) -> TaskFuture {
        let std_duration: std::time::Duration = duration
            .try_into()
            .expect("sleep duration out of range for tokio");
        Box::pin(::tokio::time::sleep(std_duration))
    }

    fn yield_now(&self) -> TaskFuture {
        Box::pin(async {
            ::tokio::task::yield_now().await;
        })
    }
}

struct JoinHandle {
    handle: Option<::tokio::task::JoinHandle<()>>,
}

impl Future for JoinHandle {
    type Output = Result<(), Box<dyn Error + Send>>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.as_mut().get_mut();
        let handle = match this.handle.as_mut() {
            Some(handle) => handle,
            None => return Poll::Ready(Ok(())),
        };
        match Pin::new(handle).poll(cx) {
            Poll::Ready(Ok(())) => {
                this.handle = None;
                Poll::Ready(Ok(()))
            }
            Poll::Ready(Err(join_err)) => {
                this.handle = None;
                Poll::Ready(Err(Box::new(join_err) as Box<dyn Error + Send>))
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

impl AbortableTask for JoinHandle {
    fn abort(&self) {
        if let Some(handle) = &self.handle {
            handle.abort();
        }
    }
}
