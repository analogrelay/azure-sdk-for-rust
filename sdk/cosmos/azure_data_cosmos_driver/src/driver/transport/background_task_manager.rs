// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Manages background tasks spawned by a client.
//!
//! [`BackgroundTaskManager`] holds on to the [`SpawnedTask`] handles returned
//! by [`AsyncRuntime::spawn`]. Dropping the manager calls
//! [`AbortableTask::abort`](azure_core::async_runtime::AbortableTask::abort)
//! on each tracked handle, cancelling the associated task (handles detach
//! on drop without explicit abort).
//!
//! The manager is intended for long-lived spawn-once tasks (periodic health
//! sweeps, refresh loops). It does not prune finished handles on each spawn
//! — production callers spawn a fixed number of tasks at construction and
//! they run for the lifetime of the owner. Tests cover both the
//! abort-on-drop and shutdown-and-await paths.

use azure_core::async_runtime::{AsyncRuntime, SpawnedTask};
use futures::FutureExt;
use std::future::Future;
use std::panic::AssertUnwindSafe;
use std::sync::{Arc, Mutex};
use tracing::{debug, error};

/// Manages the lifecycle of background tasks spawned on the configured
/// [`AsyncRuntime`].
///
/// Spawned tasks are kept alive by storing their [`SpawnedTask`] handles.
/// When the manager is dropped, all handles are aborted, cancelling the
/// associated tasks (handles detach on drop without explicit abort).
#[allow(dead_code)]
pub(crate) struct BackgroundTaskManager {
    /// Async runtime used to spawn new tasks. Stored as an `Arc` so
    /// [`spawn`](Self::spawn) can dispatch without re-resolving the runtime.
    runtime: Arc<dyn AsyncRuntime>,
    /// Stored task handles. Aborting these cancels the tasks.
    /// Uses a [`Mutex`] for interior mutability so that [`spawn`](Self::spawn)
    /// can accept `&self`, which is required when the manager lives inside an
    /// `Arc`.
    tasks: Mutex<Vec<SpawnedTask>>,
}

impl std::fmt::Debug for BackgroundTaskManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = f.debug_struct("BackgroundTaskManager");
        match self.tasks.lock() {
            Ok(tasks) => s.field("tasks_count", &tasks.len()),
            Err(_) => s.field("tasks_count", &"<poisoned>"),
        };
        s.finish()
    }
}

#[allow(dead_code)]
impl BackgroundTaskManager {
    /// Creates a new [`BackgroundTaskManager`] backed by the supplied
    /// [`AsyncRuntime`] with no active tasks.
    pub fn new(runtime: Arc<dyn AsyncRuntime>) -> Self {
        Self {
            runtime,
            tasks: Mutex::new(Vec::new()),
        }
    }

    /// Spawns a background task on the configured runtime and stores the
    /// handle.
    ///
    /// The task will remain alive as long as this manager is alive. When the
    /// manager is dropped, all stored handles are aborted, cancelling the
    /// tasks.
    pub fn spawn<F>(&self, future: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let handle = self.runtime.spawn(Box::pin(async move {
            if let Err(panic_payload) = AssertUnwindSafe(future).catch_unwind().await {
                let msg = panic_payload
                    .downcast_ref::<&str>()
                    .copied()
                    .or_else(|| panic_payload.downcast_ref::<String>().map(|s| s.as_str()))
                    .unwrap_or("<non-string panic>");
                error!("Background task panicked: {msg}");
            }
        }));
        let mut tasks = self
            .tasks
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        tasks.push(handle);
    }

    /// Aborts all tracked tasks and waits for them to fully terminate.
    ///
    /// Unlike [`Drop`] (which aborts tasks without awaiting), this method
    /// provides deterministic cleanup by ensuring all tasks have fully
    /// stopped before returning. Use this for graceful shutdown paths.
    pub async fn shutdown(&self) {
        let tasks: Vec<_> = self
            .tasks
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .drain(..)
            .collect();
        let count = tasks.len();
        debug!("BackgroundTaskManager: shutting down {count} background task(s).");
        for handle in tasks {
            handle.abort();
            // The runtime returns an abort-friendly future; awaiting it after
            // `abort` resolves promptly with either Ok or the abort error.
            let _ = handle.await;
        }
    }
}

impl Drop for BackgroundTaskManager {
    fn drop(&mut self) {
        // Use unwrap_or_else to recover from a poisoned mutex instead of
        // panicking — panicking in Drop during unwinding would abort the process.
        let tasks = self.tasks.get_mut().unwrap_or_else(|e| e.into_inner());
        let count = tasks.len();
        debug!(
            "BackgroundTaskManager: aborting {} background task(s).",
            count,
        );
        for handle in tasks.drain(..) {
            handle.abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use azure_core::async_runtime::get_async_runtime;
    use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
    use std::sync::Arc;
    use tokio::time::Duration;

    fn manager() -> BackgroundTaskManager {
        BackgroundTaskManager::new(get_async_runtime())
    }

    #[test]
    fn new_manager_has_no_tasks() {
        let manager = manager();
        assert_eq!(manager.tasks.lock().unwrap().len(), 0);
    }

    #[test]
    fn debug_shows_task_count() {
        let manager = manager();
        let debug_str = format!("{:?}", manager);
        assert!(debug_str.contains("tasks_count"));
    }

    #[tokio::test]
    async fn drop_cleans_up_tasks() {
        let manager = manager();
        manager.spawn(async {});
        assert_eq!(manager.tasks.lock().unwrap().len(), 1);
        drop(manager);
    }

    #[tokio::test]
    async fn task_runs_to_completion() {
        let counter = Arc::new(AtomicU32::new(0));

        let manager = manager();
        {
            let counter = Arc::clone(&counter);
            manager.spawn(async move {
                for _ in 0..5 {
                    counter.fetch_add(1, Ordering::SeqCst);
                    tokio::task::yield_now().await;
                }
            });
        }

        // Wait for task completion with timeout instead of fixed sleep
        tokio::time::timeout(Duration::from_secs(5), async {
            while counter.load(Ordering::SeqCst) < 5 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("task should complete within timeout");

        assert_eq!(counter.load(Ordering::SeqCst), 5);
    }

    #[tokio::test]
    async fn drop_aborts_running_task() {
        let started = Arc::new(AtomicBool::new(false));
        let completed = Arc::new(AtomicBool::new(false));

        let manager = manager();
        {
            let started = Arc::clone(&started);
            let completed = Arc::clone(&completed);
            manager.spawn(async move {
                started.store(true, Ordering::SeqCst);
                // Simulate long-running work — will be aborted before finishing
                for _ in 0..1_000_000 {
                    tokio::task::yield_now().await;
                }
                completed.store(true, Ordering::SeqCst);
            });
        }

        // Wait for task to start with timeout instead of fixed sleep
        tokio::time::timeout(Duration::from_secs(5), async {
            while !started.load(Ordering::SeqCst) {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("task should start within timeout");

        drop(manager);

        // Give the scheduler a chance to process the abort
        tokio::task::yield_now().await;

        assert!(
            !completed.load(Ordering::SeqCst),
            "task should have been aborted, not completed"
        );
    }

    #[tokio::test]
    async fn shutdown_awaits_task_termination() {
        let started = Arc::new(AtomicBool::new(false));

        let manager = manager();
        {
            let started = Arc::clone(&started);
            manager.spawn(async move {
                started.store(true, Ordering::SeqCst);
                for _ in 0..1_000_000 {
                    tokio::task::yield_now().await;
                }
            });
        }

        tokio::time::timeout(Duration::from_secs(5), async {
            while !started.load(Ordering::SeqCst) {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("task should start within timeout");

        // shutdown should abort and await — deterministic cleanup
        manager.shutdown().await;

        // After shutdown, no tasks should remain tracked
        assert_eq!(manager.tasks.lock().unwrap().len(), 0);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn concurrent_spawn_is_safe() {
        let manager = Arc::new(manager());
        let done_count = Arc::new(AtomicU32::new(0));

        let mut spawner_handles = Vec::new();
        for _ in 0..20 {
            let mgr = Arc::clone(&manager);
            let done_count = Arc::clone(&done_count);
            spawner_handles.push(tokio::spawn(async move {
                mgr.spawn(async move {
                    done_count.fetch_add(1, Ordering::SeqCst);
                });
            }));
        }

        for jh in spawner_handles {
            jh.await.unwrap();
        }

        // Wait for all background tasks to complete
        tokio::time::timeout(Duration::from_secs(5), async {
            while done_count.load(Ordering::SeqCst) < 20 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("all background tasks should complete within timeout");

        assert_eq!(done_count.load(Ordering::SeqCst), 20);
    }
}
