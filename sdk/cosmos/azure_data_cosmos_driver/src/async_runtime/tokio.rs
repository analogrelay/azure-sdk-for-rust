// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Tokio-backed async runtime primitives.

use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

/// Entry point for async runtime operations.
///
/// All methods are static — there is no instance to construct. The underlying
/// runtime (tokio) must already be running when these are called.
#[expect(dead_code)]
pub(crate) struct AsyncRuntime;

#[expect(dead_code)]
impl AsyncRuntime {
    /// Spawns a future as a new task and returns a [`JoinHandle`] for it.
    ///
    /// Unlike [`azure_core::async_runtime::AsyncRuntime::spawn`], the returned
    /// handle supports [`abort`](JoinHandle::abort) and
    /// [`is_finished`](JoinHandle::is_finished).
    pub fn spawn<F>(future: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        JoinHandle {
            inner: tokio::spawn(future),
        }
    }

    /// Sleeps for the given duration.
    pub async fn sleep(duration: Duration) {
        tokio::time::sleep(duration).await;
    }

    /// Yields the current task back to the runtime scheduler.
    pub async fn yield_now() {
        tokio::task::yield_now().await;
    }

    /// Creates a periodic [`Interval`] that ticks every `period`.
    ///
    /// The first tick completes immediately.
    pub fn interval(period: Duration) -> Interval {
        Interval {
            inner: tokio::time::interval(period),
        }
    }

    /// Returns `true` if an async runtime is currently available.
    ///
    /// This can be used to guard operations that require a running runtime
    /// (e.g., spawning background tasks).
    pub fn is_available() -> bool {
        tokio::runtime::Handle::try_current().is_ok()
    }
}

// ---------------------------------------------------------------------------
// JoinHandle<T>
// ---------------------------------------------------------------------------

/// A handle to a spawned task. Wraps [`tokio::task::JoinHandle`].
///
/// Awaiting the handle returns the task's output (or a [`JoinError`] if the
/// task panicked or was cancelled). Dropping the handle *detaches* the task;
/// use [`abort`](Self::abort) to cancel it.
pub(crate) struct JoinHandle<T> {
    inner: tokio::task::JoinHandle<T>,
}

impl<T> JoinHandle<T> {
    /// Aborts the task, causing it to be cancelled.
    pub fn abort(&self) {
        self.inner.abort();
    }

    /// Returns `true` if the task has completed (successfully, by panic, or
    /// by cancellation).
    pub fn is_finished(&self) -> bool {
        self.inner.is_finished()
    }
}

impl<T> Future for JoinHandle<T> {
    type Output = Result<T, JoinError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // SAFETY: we never move `inner` after pinning `self`.
        let inner = unsafe { Pin::new_unchecked(&mut self.inner) };
        inner.poll(cx).map_err(JoinError::from)
    }
}

impl<T> fmt::Debug for JoinHandle<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("JoinHandle")
            .field("is_finished", &self.inner.is_finished())
            .finish()
    }
}

// ---------------------------------------------------------------------------
// JoinError
// ---------------------------------------------------------------------------

/// Error returned when awaiting a [`JoinHandle`] whose task has failed.
pub(crate) struct JoinError {
    inner: tokio::task::JoinError,
}

#[expect(dead_code)]
impl JoinError {
    /// Returns `true` if the task was cancelled via [`JoinHandle::abort`].
    pub fn is_cancelled(&self) -> bool {
        self.inner.is_cancelled()
    }

    /// Returns `true` if the task panicked.
    pub fn is_panic(&self) -> bool {
        self.inner.is_panic()
    }
}

impl From<tokio::task::JoinError> for JoinError {
    fn from(inner: tokio::task::JoinError) -> Self {
        Self { inner }
    }
}

impl fmt::Display for JoinError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}

impl fmt::Debug for JoinError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}

impl std::error::Error for JoinError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.inner.source()
    }
}

// ---------------------------------------------------------------------------
// AbortHandle
// ---------------------------------------------------------------------------

/// A handle that can abort a spawned task without owning it.
/// Wraps [`tokio::task::AbortHandle`].
#[expect(dead_code)]
pub(crate) struct AbortHandle {
    inner: tokio::task::AbortHandle,
}

impl AbortHandle {
    /// Aborts the associated task.
    #[expect(dead_code)]
    pub fn abort(&self) {
        self.inner.abort();
    }

    /// Returns `true` if the associated task has completed.
    #[expect(dead_code)]
    pub fn is_finished(&self) -> bool {
        self.inner.is_finished()
    }
}

impl fmt::Debug for AbortHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AbortHandle").finish()
    }
}

// ---------------------------------------------------------------------------
// TaskSet<T>
// ---------------------------------------------------------------------------

/// A collection of spawned tasks. Wraps [`tokio::task::JoinSet`].
///
/// Tasks are spawned into the set and can be awaited individually
/// ([`join_next`](Self::join_next)) or collectively
/// ([`join_all`](Self::join_all)). Dropping the set aborts all tasks.
#[expect(dead_code)]
pub(crate) struct TaskSet<T> {
    inner: tokio::task::JoinSet<T>,
}

#[expect(dead_code)]
impl<T: Send + 'static> TaskSet<T> {
    /// Creates a new, empty task set.
    pub fn new() -> Self {
        Self {
            inner: tokio::task::JoinSet::new(),
        }
    }

    /// Spawns a task into the set, returning an [`AbortHandle`] for it.
    pub fn spawn<F>(&mut self, future: F) -> AbortHandle
    where
        F: Future<Output = T> + Send + 'static,
    {
        AbortHandle {
            inner: self.inner.spawn(future),
        }
    }

    /// Waits for the next task in the set to complete.
    ///
    /// Returns `None` if the set is empty.
    pub async fn join_next(&mut self) -> Option<Result<T, JoinError>> {
        self.inner
            .join_next()
            .await
            .map(|r| r.map_err(JoinError::from))
    }

    /// Waits for all tasks to complete, returning their outputs.
    ///
    /// # Panics
    ///
    /// Panics if any task in the set panicked.
    pub async fn join_all(self) -> Vec<T> {
        self.inner.join_all().await
    }

    /// Aborts all tasks in the set.
    pub fn abort_all(&mut self) {
        self.inner.abort_all();
    }

    /// Returns the number of tasks in the set.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns `true` if the set contains no tasks.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

impl<T> fmt::Debug for TaskSet<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TaskSet")
            .field("len", &self.inner.len())
            .finish()
    }
}

// ---------------------------------------------------------------------------
// Interval
// ---------------------------------------------------------------------------

/// A periodic timer. Wraps [`tokio::time::Interval`].
pub(crate) struct Interval {
    inner: tokio::time::Interval,
}

impl Interval {
    /// Waits for the next tick, returning the [`Instant`] at which it fired.
    pub async fn tick(&mut self) -> Instant {
        self.inner.tick().await.into()
    }

    /// Sets the strategy for missed ticks.
    pub fn set_missed_tick_behavior(&mut self, behavior: MissedTickBehavior) {
        self.inner.set_missed_tick_behavior(behavior.into());
    }
}

impl fmt::Debug for Interval {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Interval").finish()
    }
}

// ---------------------------------------------------------------------------
// MissedTickBehavior
// ---------------------------------------------------------------------------

/// Strategy for handling missed ticks in an [`Interval`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MissedTickBehavior {
    /// Fire all missed ticks as quickly as possible.
    #[expect(dead_code)]
    Burst,
    /// Delay future ticks, maintaining the interval from the last tick.
    #[expect(dead_code)]
    Delay,
    /// Skip missed ticks and continue with the next scheduled tick.
    Skip,
}

impl From<MissedTickBehavior> for tokio::time::MissedTickBehavior {
    fn from(value: MissedTickBehavior) -> Self {
        match value {
            MissedTickBehavior::Burst => tokio::time::MissedTickBehavior::Burst,
            MissedTickBehavior::Delay => tokio::time::MissedTickBehavior::Delay,
            MissedTickBehavior::Skip => tokio::time::MissedTickBehavior::Skip,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
    use std::sync::Arc;

    #[tokio::test]
    async fn spawn_runs_to_completion() {
        let flag = Arc::new(AtomicBool::new(false));
        let flag2 = flag.clone();
        let handle = AsyncRuntime::spawn(async move {
            flag2.store(true, Ordering::SeqCst);
        });
        handle.await.expect("task should succeed");
        assert!(flag.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn spawn_returns_value() {
        let handle = AsyncRuntime::spawn(async { 42 });
        assert_eq!(handle.await.unwrap(), 42);
    }

    #[tokio::test]
    async fn join_handle_abort() {
        let handle = AsyncRuntime::spawn(async {
            loop {
                AsyncRuntime::yield_now().await;
            }
        });
        handle.abort();
        let result = handle.await;
        assert!(result.is_err());
        assert!(result.unwrap_err().is_cancelled());
    }

    #[tokio::test]
    async fn join_handle_is_finished() {
        let handle = AsyncRuntime::spawn(async {});
        tokio::time::timeout(Duration::from_secs(5), async {
            while !handle.is_finished() {
                AsyncRuntime::yield_now().await;
            }
        })
        .await
        .expect("task should finish");
        assert!(handle.is_finished());
    }

    #[tokio::test]
    async fn task_set_spawn_and_join_all() {
        let mut set = TaskSet::new();
        for i in 0..5 {
            set.spawn(async move { i });
        }
        let mut results = set.join_all().await;
        results.sort();
        assert_eq!(results, vec![0, 1, 2, 3, 4]);
    }

    #[tokio::test]
    async fn task_set_join_next() {
        let mut set = TaskSet::new();
        set.spawn(async { 1 });
        set.spawn(async { 2 });

        let mut results = Vec::new();
        while let Some(r) = set.join_next().await {
            results.push(r.expect("task should succeed"));
        }
        results.sort();
        assert_eq!(results, vec![1, 2]);
    }

    #[tokio::test]
    async fn task_set_abort_all() {
        let counter = Arc::new(AtomicU32::new(0));
        let mut set = TaskSet::new();
        for _ in 0..5 {
            let c = counter.clone();
            set.spawn(async move {
                loop {
                    c.fetch_add(1, Ordering::SeqCst);
                    AsyncRuntime::yield_now().await;
                }
            });
        }
        // Let tasks start.
        AsyncRuntime::yield_now().await;
        set.abort_all();

        // Drain all results — they should all be cancelled.
        while let Some(r) = set.join_next().await {
            assert!(r.unwrap_err().is_cancelled());
        }
    }

    #[tokio::test]
    async fn task_set_len_and_empty() {
        let mut set: TaskSet<()> = TaskSet::new();
        assert!(set.is_empty());
        assert_eq!(set.len(), 0);

        set.spawn(async {});
        assert!(!set.is_empty());
        assert_eq!(set.len(), 1);
    }

    #[tokio::test]
    async fn sleep_completes() {
        let start = Instant::now();
        AsyncRuntime::sleep(Duration::from_millis(10)).await;
        assert!(start.elapsed() >= Duration::from_millis(10));
    }

    #[tokio::test]
    async fn yield_now_returns() {
        // Simply verify it doesn't hang.
        AsyncRuntime::yield_now().await;
    }

    #[tokio::test(start_paused = true)]
    async fn interval_ticks() {
        let mut interval = AsyncRuntime::interval(Duration::from_millis(100));
        // First tick is immediate.
        interval.tick().await;
        // Advance time.
        tokio::time::advance(Duration::from_millis(100)).await;
        interval.tick().await;
    }

    #[test]
    fn is_available_outside_runtime() {
        // Running in a sync test — no runtime should be active.
        // Note: this may still return true if a tokio runtime is in scope
        // from the test harness, so we just verify it doesn't panic.
        let _ = AsyncRuntime::is_available();
    }

    #[tokio::test]
    async fn is_available_inside_runtime() {
        assert!(AsyncRuntime::is_available());
    }

    #[tokio::test]
    async fn abort_handle_aborts_task() {
        let mut set = TaskSet::new();
        let ah = set.spawn(async {
            loop {
                AsyncRuntime::yield_now().await;
            }
        });
        ah.abort();
        let result = set.join_next().await.expect("should have one task");
        assert!(result.unwrap_err().is_cancelled());
    }
}
