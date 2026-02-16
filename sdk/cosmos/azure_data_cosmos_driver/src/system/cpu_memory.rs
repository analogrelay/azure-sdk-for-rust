// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! CPU and memory monitoring with historical snapshots.

use std::{
    collections::VecDeque,
    sync::{Arc, OnceLock, RwLock, Weak},
    thread,
    time::{Duration, Instant},
};

#[cfg(target_os = "linux")]
use std::fs;

/// Default interval between CPU/memory samples.
const DEFAULT_REFRESH_INTERVAL: Duration = Duration::from_secs(5);

/// Number of historical samples to retain.
const HISTORY_LENGTH: usize = 6;

/// CPU load threshold percentage for considering the system overloaded.
const CPU_OVERLOAD_THRESHOLD: f32 = 90.0;

/// Global singleton for CPU/memory monitoring.
static CPU_MEMORY_MONITOR: OnceLock<Arc<CpuMemoryMonitorInner>> = OnceLock::new();

/// A single CPU load measurement at a point in time.
#[non_exhaustive]
#[derive(Clone, Copy, Debug)]
pub struct CpuLoad {
    /// When this measurement was taken.
    timestamp: Instant,
    /// CPU usage percentage (0.0 to 100.0).
    value: f32,
}

impl CpuLoad {
    /// Creates a new CPU load measurement.
    ///
    /// # Panics
    ///
    /// Panics if `value` is not between 0.0 and 100.0.
    pub fn new(timestamp: Instant, value: f32) -> Self {
        assert!(
            (0.0..=100.0).contains(&value),
            "CPU load must be between 0.0 and 100.0, got {}",
            value
        );
        Self { timestamp, value }
    }

    /// Returns when this measurement was taken.
    pub fn timestamp(&self) -> Instant {
        self.timestamp
    }

    /// Returns the CPU usage percentage (0.0 to 100.0).
    pub fn value(&self) -> f32 {
        self.value
    }
}

impl std::fmt::Display for CpuLoad {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({:.1}%)", self.value)
    }
}

/// A single memory measurement at a point in time.
#[non_exhaustive]
#[derive(Clone, Copy, Debug)]
pub struct MemoryUsage {
    /// When this measurement was taken.
    pub timestamp: Instant,
    /// Available memory in megabytes.
    pub available_mb: u64,
}

/// Historical CPU and memory usage data.
#[non_exhaustive]
#[derive(Clone, Debug)]
pub struct CpuMemoryHistory {
    /// Historical CPU load samples (oldest first).
    cpu_samples: Vec<CpuLoad>,
    /// Historical memory samples (oldest first).
    memory_samples: Vec<MemoryUsage>,
    /// The interval between samples.
    refresh_interval: Duration,
}

impl CpuMemoryHistory {
    /// Returns the CPU load samples.
    pub fn cpu_samples(&self) -> &[CpuLoad] {
        &self.cpu_samples
    }

    /// Returns the memory usage samples.
    pub fn memory_samples(&self) -> &[MemoryUsage] {
        &self.memory_samples
    }

    /// Returns the refresh interval between samples.
    pub fn refresh_interval(&self) -> Duration {
        self.refresh_interval
    }

    /// Returns `true` if the CPU appears to be overloaded.
    ///
    /// The CPU is considered overloaded if any recent sample exceeds 90%
    /// or if there are significant delays in thread scheduling.
    pub fn is_cpu_overloaded(&self) -> bool {
        self.is_cpu_over_threshold(CPU_OVERLOAD_THRESHOLD) || self.has_scheduling_delay()
    }

    /// Returns `true` if any CPU sample exceeds the given threshold.
    pub fn is_cpu_over_threshold(&self, threshold: f32) -> bool {
        self.cpu_samples.iter().any(|s| s.value > threshold)
    }

    /// Returns the most recent CPU load, if available.
    pub fn latest_cpu(&self) -> Option<CpuLoad> {
        self.cpu_samples.last().copied()
    }

    /// Returns the most recent memory usage, if available.
    pub fn latest_memory(&self) -> Option<MemoryUsage> {
        self.memory_samples.last().copied()
    }

    /// Returns `true` if there appears to be scheduling delays.
    fn has_scheduling_delay(&self) -> bool {
        // Check if there are gaps between consecutive samples larger than 1.5x the interval
        let threshold = self.refresh_interval.as_millis() * 3 / 2;
        for window in self.cpu_samples.windows(2) {
            let gap = window[1].timestamp.duration_since(window[0].timestamp);
            if gap.as_millis() > threshold {
                return true;
            }
        }
        false
    }
}

impl Default for CpuMemoryHistory {
    fn default() -> Self {
        Self {
            cpu_samples: Vec::new(),
            memory_samples: Vec::new(),
            refresh_interval: DEFAULT_REFRESH_INTERVAL,
        }
    }
}

impl std::fmt::Display for CpuMemoryHistory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.cpu_samples.is_empty() {
            write!(f, "empty")
        } else {
            let samples: Vec<String> = self.cpu_samples.iter().map(|s| s.to_string()).collect();
            write!(f, "{}", samples.join(", "))
        }
    }
}

/// Handle to the CPU/memory monitor singleton.
///
/// This handle keeps the monitor alive. When all handles are dropped,
/// the background monitoring thread will stop.
#[non_exhaustive]
#[derive(Clone, Debug)]
pub struct CpuMemoryMonitor {
    inner: Arc<CpuMemoryMonitorInner>,
}

impl CpuMemoryMonitor {
    /// Gets or creates the global CPU/memory monitor singleton.
    ///
    /// The monitor starts a background thread that periodically samples
    /// CPU and memory usage. The thread runs as long as at least one
    /// `CpuMemoryMonitor` handle exists.
    pub fn get_or_init() -> Self {
        let inner = CPU_MEMORY_MONITOR
            .get_or_init(|| {
                let inner = Arc::new(CpuMemoryMonitorInner::new());
                inner.start();
                inner
            })
            .clone();

        // Register this as a listener to keep the monitor alive
        inner.register();

        Self { inner }
    }

    /// Returns a snapshot of the current CPU and memory history.
    pub fn snapshot(&self) -> CpuMemoryHistory {
        self.inner.snapshot()
    }

    /// Returns `true` if the CPU appears to be overloaded.
    pub fn is_cpu_overloaded(&self) -> bool {
        self.snapshot().is_cpu_overloaded()
    }
}

impl Drop for CpuMemoryMonitor {
    fn drop(&mut self) {
        self.inner.unregister();
    }
}

/// Internal state for the CPU/memory monitor.
#[derive(Debug)]
struct CpuMemoryMonitorInner {
    /// Current history, protected by a read-write lock.
    history: RwLock<CpuMemoryHistory>,
    /// Circular buffer for CPU samples.
    cpu_buffer: RwLock<VecDeque<CpuLoad>>,
    /// Circular buffer for memory samples.
    memory_buffer: RwLock<VecDeque<MemoryUsage>>,
    /// Number of active listeners (handles).
    listener_count: RwLock<usize>,
    /// Weak reference to self for the background thread.
    self_ref: RwLock<Option<Weak<CpuMemoryMonitorInner>>>,
    /// The refresh interval.
    refresh_interval: Duration,
}

impl CpuMemoryMonitorInner {
    fn new() -> Self {
        Self {
            history: RwLock::new(CpuMemoryHistory::default()),
            cpu_buffer: RwLock::new(VecDeque::with_capacity(HISTORY_LENGTH)),
            memory_buffer: RwLock::new(VecDeque::with_capacity(HISTORY_LENGTH)),
            listener_count: RwLock::new(0),
            self_ref: RwLock::new(None),
            refresh_interval: DEFAULT_REFRESH_INTERVAL,
        }
    }

    fn start(self: &Arc<Self>) {
        // Store weak reference for the background thread
        *self.self_ref.write().unwrap() = Some(Arc::downgrade(self));

        let weak = Arc::downgrade(self);
        thread::Builder::new()
            .name("cosmos-cpu-monitor".into())
            .spawn(move || {
                Self::monitor_loop(weak);
            })
            .expect("failed to spawn CPU monitor thread");
    }

    fn register(&self) {
        let mut count = self.listener_count.write().unwrap();
        *count += 1;
    }

    fn unregister(&self) {
        let mut count = self.listener_count.write().unwrap();
        *count = count.saturating_sub(1);
    }

    fn has_listeners(&self) -> bool {
        *self.listener_count.read().unwrap() > 0
    }

    fn snapshot(&self) -> CpuMemoryHistory {
        self.history.read().unwrap().clone()
    }

    fn monitor_loop(weak: Weak<CpuMemoryMonitorInner>) {
        loop {
            thread::sleep(DEFAULT_REFRESH_INTERVAL);

            let Some(inner) = weak.upgrade() else {
                // Monitor was dropped, exit the thread
                break;
            };

            if !inner.has_listeners() {
                // No listeners, but keep the thread alive in case new ones register
                continue;
            }

            inner.refresh();
        }
    }

    fn refresh(&self) {
        let now = Instant::now();

        // Read CPU usage
        let cpu_value = read_cpu_usage();
        if let Some(cpu) = cpu_value {
            let mut cpu_buffer = self.cpu_buffer.write().unwrap();
            if cpu_buffer.len() >= HISTORY_LENGTH {
                cpu_buffer.pop_front();
            }
            cpu_buffer.push_back(CpuLoad::new(now, cpu));
        }

        // Read memory usage
        let memory_mb = read_available_memory_mb();
        {
            let mut memory_buffer = self.memory_buffer.write().unwrap();
            if memory_buffer.len() >= HISTORY_LENGTH {
                memory_buffer.pop_front();
            }
            memory_buffer.push_back(MemoryUsage {
                timestamp: now,
                available_mb: memory_mb,
            });
        }

        // Update the history snapshot
        let cpu_samples: Vec<CpuLoad> = self.cpu_buffer.read().unwrap().iter().copied().collect();
        let memory_samples: Vec<MemoryUsage> =
            self.memory_buffer.read().unwrap().iter().copied().collect();

        let new_history = CpuMemoryHistory {
            cpu_samples,
            memory_samples,
            refresh_interval: self.refresh_interval,
        };

        *self.history.write().unwrap() = new_history;
    }
}

/// Reads the current system-wide CPU usage as a percentage (0.0 to 100.0).
fn read_cpu_usage() -> Option<f32> {
    #[cfg(target_os = "linux")]
    {
        read_cpu_usage_linux()
    }

    #[cfg(target_os = "windows")]
    {
        read_cpu_usage_windows()
    }

    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        None
    }
}

#[cfg(target_os = "linux")]
fn read_cpu_usage_linux() -> Option<f32> {
    // Read /proc/stat for CPU statistics
    // This is a simplified implementation; a proper one would track deltas
    static PREV_IDLE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    static PREV_TOTAL: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

    let content = fs::read_to_string("/proc/stat").ok()?;
    let cpu_line = content.lines().find(|l| l.starts_with("cpu "))?;
    let values: Vec<u64> = cpu_line
        .split_whitespace()
        .skip(1)
        .filter_map(|s| s.parse().ok())
        .collect();

    if values.len() < 4 {
        return None;
    }

    let idle = values.get(3).copied().unwrap_or(0);
    let total: u64 = values.iter().sum();

    let prev_idle = PREV_IDLE.swap(idle, std::sync::atomic::Ordering::Relaxed);
    let prev_total = PREV_TOTAL.swap(total, std::sync::atomic::Ordering::Relaxed);

    if prev_total == 0 {
        return None; // First reading
    }

    let idle_delta = idle.saturating_sub(prev_idle);
    let total_delta = total.saturating_sub(prev_total);

    if total_delta == 0 {
        return Some(0.0);
    }

    let usage = 100.0 * (1.0 - (idle_delta as f32 / total_delta as f32));
    Some(usage.clamp(0.0, 100.0))
}

#[cfg(target_os = "windows")]
fn read_cpu_usage_windows() -> Option<f32> {
    // On Windows, we'd use GetSystemTimes or PDH
    // For now, return None as a placeholder
    // TODO: Implement using windows-sys crate
    None
}

/// Reads the available system memory in megabytes.
fn read_available_memory_mb() -> u64 {
    #[cfg(target_os = "linux")]
    {
        read_available_memory_linux()
    }

    #[cfg(target_os = "windows")]
    {
        read_available_memory_windows()
    }

    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        0
    }
}

#[cfg(target_os = "linux")]
fn read_available_memory_linux() -> u64 {
    // Read /proc/meminfo for MemAvailable
    let content = match fs::read_to_string("/proc/meminfo") {
        Ok(c) => c,
        Err(_) => return 0,
    };

    for line in content.lines() {
        if line.starts_with("MemAvailable:") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if let Some(kb_str) = parts.get(1) {
                if let Ok(kb) = kb_str.parse::<u64>() {
                    return kb / 1024; // Convert KB to MB
                }
            }
        }
    }

    0
}

#[cfg(target_os = "windows")]
fn read_available_memory_windows() -> u64 {
    // On Windows, we'd use GlobalMemoryStatusEx
    // For now, return 0 as a placeholder
    // TODO: Implement using windows-sys crate
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpu_load_valid_range() {
        let load = CpuLoad::new(Instant::now(), 50.0);
        assert_eq!(load.value, 50.0);
    }

    #[test]
    #[should_panic(expected = "CPU load must be between 0.0 and 100.0")]
    fn cpu_load_invalid_negative() {
        CpuLoad::new(Instant::now(), -1.0);
    }

    #[test]
    #[should_panic(expected = "CPU load must be between 0.0 and 100.0")]
    fn cpu_load_invalid_over_100() {
        CpuLoad::new(Instant::now(), 101.0);
    }

    #[test]
    fn cpu_memory_history_empty() {
        let history = CpuMemoryHistory::default();
        assert!(history.cpu_samples().is_empty());
        assert!(history.memory_samples().is_empty());
        assert!(!history.is_cpu_overloaded());
    }

    #[test]
    fn cpu_memory_history_overload_detection() {
        let history = CpuMemoryHistory {
            cpu_samples: vec![CpuLoad::new(Instant::now(), 95.0)],
            memory_samples: vec![],
            refresh_interval: DEFAULT_REFRESH_INTERVAL,
        };
        assert!(history.is_cpu_overloaded());
        assert!(history.is_cpu_over_threshold(90.0));
        assert!(!history.is_cpu_over_threshold(96.0));
    }

    #[test]
    fn cpu_memory_monitor_singleton() {
        let monitor1 = CpuMemoryMonitor::get_or_init();
        let monitor2 = CpuMemoryMonitor::get_or_init();

        // Both should point to the same inner
        assert!(Arc::ptr_eq(&monitor1.inner, &monitor2.inner));
    }
}
