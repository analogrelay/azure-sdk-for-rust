// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Runtime-agnostic primitives for racing two futures against each other.
//!
//! This module powers [`AsyncRuntime::timeout`](super::AsyncRuntime::timeout)
//! and is exposed so other runtime-agnostic combinators can be built on top
//! of it. The macro [`select_two!`](crate::select_two!) is the public
//! entry-point; [`SelectTwoResult`] is the value the macro resolves to.
//!
//! The implementation does **not** spawn, block, or use any reactor — it is
//! a plain hand-written [`Future`](std::future::Future) that polls each input
//! in turn and resolves with whichever finishes first.

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

/// Outcome of [`select_two!`](crate::select_two!).
///
/// Each variant carries the resolved value of the side that completed first
/// and the still-running future for the other side. The caller can drop or
/// continue to poll the still-running future as they see fit.
///
/// * `A` / `B` — the `Output` types of the two input futures.
/// * `FA` / `FB` — the input future types themselves (returned in the variant
///   for the side that did *not* complete).
#[derive(Debug)]
pub enum SelectTwoResult<A, FA, B, FB> {
    /// The first input future resolved with `A`. `FB` is the still-pending
    /// second future.
    First((A, FB)),

    /// The second input future resolved with `B`. `FA` is the still-pending
    /// first future.
    Second((B, FA)),
}

/// Polls two `Unpin` futures concurrently and resolves with whichever
/// finishes first.
///
/// This is the runtime-agnostic building block used by
/// [`AsyncRuntime::timeout`](super::AsyncRuntime::timeout). Application code
/// can use it directly to compose simple two-way selects without depending
/// on any particular runtime's `select!` machinery.
///
/// # Requirements
///
/// Both arguments must implement [`Future`] **and** [`Unpin`]. The common
/// path is to pass already-pinned futures — values of type
/// [`TaskFuture`](super::TaskFuture) (returned by every [`AsyncRuntime`](super::AsyncRuntime)
/// method) are `Pin<Box<…>>` and therefore `Unpin`, so they work as-is.
/// For an arbitrary `impl Future` value, wrap it in [`Box::pin`] before
/// passing it to the macro.
///
/// # Fairness
///
/// The implementation alternates which side it polls first on each wakeup,
/// so neither input can starve the other.
///
/// # Example
///
/// ```
/// use typespec_client_core::async_runtime::{get_async_runtime, SelectTwoResult};
/// use typespec_client_core::select_two;
/// use typespec_client_core::time::Duration;
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() {
/// let runtime = get_async_runtime();
/// let work = runtime.sleep(Duration::milliseconds(10));
/// let deadline = runtime.sleep(Duration::milliseconds(500));
///
/// match select_two!(work, deadline).await {
///     SelectTwoResult::First(((), _deadline)) => {
///         // `work` finished first.
///     }
///     SelectTwoResult::Second(((), _work)) => {
///         // `deadline` fired first.
///     }
/// }
/// # }
/// ```
///
/// # Future extensions
///
/// `select_two!` is intentionally the minimum primitive needed for
/// [`AsyncRuntime::timeout`](super::AsyncRuntime::timeout). Multi-arm
/// variants (`select_three!`, `select_all!`, …) can be added in future
/// versions. Application code that needs the richer ergonomics of
/// `futures::select!` should reach for that macro instead.
#[macro_export]
macro_rules! select_two {
    ($a:expr, $b:expr) => {
        $crate::async_runtime::select::__SelectTwoFuture::new($a, $b)
    };
}

/// Implementation detail of [`select_two!`](crate::select_two!).
///
/// Constructed exclusively by the macro; the only public entry point is
/// [`new`](Self::new). The struct is marked `pub` (rather than `pub(crate)`)
/// so the macro can expand to its constructor from any crate that depends
/// on `typespec_client_core`, but it is not part of the supported public
/// API — its representation and bounds may change at any time.
#[doc(hidden)]
pub struct __SelectTwoFuture<FA, FB>
where
    FA: Future + Unpin,
    FB: Future + Unpin,
{
    a: Option<FA>,
    b: Option<FB>,
    poll_b_first: bool,
}

impl<FA, FB> __SelectTwoFuture<FA, FB>
where
    FA: Future + Unpin,
    FB: Future + Unpin,
{
    /// Constructs a new select future. Both inputs must already be `Unpin`;
    /// see the [`select_two!`](crate::select_two!) macro docs for guidance.
    #[doc(hidden)]
    pub fn new(a: FA, b: FB) -> Self {
        Self {
            a: Some(a),
            b: Some(b),
            poll_b_first: false,
        }
    }
}

impl<FA, FB> Future for __SelectTwoFuture<FA, FB>
where
    FA: Future + Unpin,
    FB: Future + Unpin,
{
    type Output = SelectTwoResult<FA::Output, FA, FB::Output, FB>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.as_mut().get_mut();
        let poll_b_first = this.poll_b_first;
        this.poll_b_first = !poll_b_first;

        if poll_b_first {
            if let Some(out) = poll_side_b(this, cx) {
                return Poll::Ready(out);
            }
            if let Some(out) = poll_side_a(this, cx) {
                return Poll::Ready(out);
            }
        } else {
            if let Some(out) = poll_side_a(this, cx) {
                return Poll::Ready(out);
            }
            if let Some(out) = poll_side_b(this, cx) {
                return Poll::Ready(out);
            }
        }

        Poll::Pending
    }
}

fn poll_side_a<FA, FB>(
    this: &mut __SelectTwoFuture<FA, FB>,
    cx: &mut Context<'_>,
) -> Option<SelectTwoResult<FA::Output, FA, FB::Output, FB>>
where
    FA: Future + Unpin,
    FB: Future + Unpin,
{
    let fa = this.a.as_mut()?;
    match Pin::new(fa).poll(cx) {
        Poll::Ready(out) => {
            let fb = this
                .b
                .take()
                .expect("__SelectTwoFuture polled after completion");
            Some(SelectTwoResult::First((out, fb)))
        }
        Poll::Pending => None,
    }
}

fn poll_side_b<FA, FB>(
    this: &mut __SelectTwoFuture<FA, FB>,
    cx: &mut Context<'_>,
) -> Option<SelectTwoResult<FA::Output, FA, FB::Output, FB>>
where
    FA: Future + Unpin,
    FB: Future + Unpin,
{
    let fb = this.b.as_mut()?;
    match Pin::new(fb).poll(cx) {
        Poll::Ready(out) => {
            let fa = this
                .a
                .take()
                .expect("__SelectTwoFuture polled after completion");
            Some(SelectTwoResult::Second((out, fa)))
        }
        Poll::Pending => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    fn ready<T: Send + 'static>(value: T) -> Pin<Box<dyn Future<Output = T> + Send>> {
        Box::pin(std::future::ready(value))
    }

    fn pending<T: Send + 'static>() -> Pin<Box<dyn Future<Output = T> + Send>> {
        Box::pin(std::future::pending())
    }

    #[test]
    fn first_ready_resolves_first() {
        let fut = select_two!(ready::<u32>(7), pending::<u32>());
        let result = futures::executor::block_on(fut);
        match result {
            SelectTwoResult::First((value, _)) => assert_eq!(value, 7),
            SelectTwoResult::Second(_) => panic!("expected First"),
        }
    }

    #[test]
    fn second_ready_resolves_second() {
        let fut = select_two!(pending::<u32>(), ready::<&'static str>("hi"));
        let result = futures::executor::block_on(fut);
        match result {
            SelectTwoResult::Second((value, _)) => assert_eq!(value, "hi"),
            SelectTwoResult::First(_) => panic!("expected Second"),
        }
    }

    #[test]
    fn both_ready_resolves_first_due_to_initial_order() {
        let fut = select_two!(ready::<u8>(1), ready::<u8>(2));
        let result = futures::executor::block_on(fut);
        match result {
            SelectTwoResult::First((value, _)) => assert_eq!(value, 1),
            SelectTwoResult::Second(_) => {
                panic!("first poll should prefer side A given the initial order")
            }
        }
    }

    #[test]
    fn alternates_poll_order_to_avoid_starvation() {
        // Both sides eventually resolve; the side that loses still gets
        // polled enough times to demonstrate the executor is alternating
        // rather than always polling A first. (The winning side reaches
        // ready_at; the losing side reaches ready_at - 1 because we
        // short-circuit on the winning Poll::Ready.)
        let polls_a = Arc::new(AtomicUsize::new(0));
        let polls_b = Arc::new(AtomicUsize::new(0));

        let fa = CountingPending {
            counter: Arc::clone(&polls_a),
            ready_at: 4,
            value: 'a',
        };
        let fb = CountingPending {
            counter: Arc::clone(&polls_b),
            ready_at: 4,
            value: 'b',
        };

        let fut = select_two!(Box::pin(fa), Box::pin(fb));
        let _ = futures::executor::block_on(fut);

        let a = polls_a.load(Ordering::SeqCst);
        let b = polls_b.load(Ordering::SeqCst);
        assert!(
            a >= 3,
            "side A should have been polled at least 3 times, got {a}"
        );
        assert!(
            b >= 3,
            "side B should have been polled at least 3 times, got {b}"
        );
        assert!(
            (a as isize - b as isize).abs() <= 1,
            "alternation should keep poll counts within 1 of each other; got A={a}, B={b}"
        );
    }

    struct CountingPending {
        counter: Arc<AtomicUsize>,
        ready_at: usize,
        value: char,
    }

    impl Future for CountingPending {
        type Output = char;
        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            let n = self.counter.fetch_add(1, Ordering::SeqCst) + 1;
            if n >= self.ready_at {
                Poll::Ready(self.value)
            } else {
                cx.waker().wake_by_ref();
                Poll::Pending
            }
        }
    }
}
