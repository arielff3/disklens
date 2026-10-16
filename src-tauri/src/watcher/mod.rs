//! Keeps the index current: watches the scanned root (`ReadDirectoryChangesW`
//! through `notify`), debounces the events and hands over batches of changed
//! paths, which [`apply`] turns into index patches.

mod apply;
mod debounce;

pub use apply::{apply, commit, probe, Changes};
pub use debounce::{Batch, Debouncer};

use notify::{EventKind, RecursiveMode, Watcher as _};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

pub const QUIET: Duration = Duration::from_millis(500);
pub const MAX_WAIT: Duration = Duration::from_secs(2);
const IDLE: Duration = Duration::from_secs(3600);

/// Watches a folder tree until dropped.
pub struct Watcher {
    _inner: notify::RecommendedWatcher,
    stopped: Arc<AtomicBool>,
}

impl Watcher {
    /// Starts watching `root` recursively. `on_batch` runs on the watcher's own
    /// thread, at most once per debounce period.
    pub fn start(
        root: &Path,
        on_batch: impl FnMut(Batch) + Send + 'static,
    ) -> notify::Result<Watcher> {
        Watcher::start_with(root, QUIET, MAX_WAIT, on_batch)
    }

    pub fn start_with(
        root: &Path,
        quiet: Duration,
        max_wait: Duration,
        mut on_batch: impl FnMut(Batch) + Send + 'static,
    ) -> notify::Result<Watcher> {
        let (tx, rx) = mpsc::channel::<notify::Result<notify::Event>>();
        let mut inner = notify::recommended_watcher(tx)?;
        inner.watch(root, RecursiveMode::Recursive)?;
        let stopped = Arc::new(AtomicBool::new(false));
        let flag = stopped.clone();
        thread::Builder::new()
            .name("watcher".into())
            .spawn(move || {
                let mut debouncer = Debouncer::new(quiet, max_wait);
                loop {
                    let wait = debouncer
                        .deadline()
                        .map_or(IDLE, |d| d.saturating_duration_since(Instant::now()));
                    match rx.recv_timeout(wait) {
                        Ok(Ok(event)) if event.need_rescan() => debouncer.overflow(Instant::now()),
                        Ok(Ok(event)) if matches!(event.kind, EventKind::Access(_)) => {}
                        Ok(Ok(event)) => debouncer.push(event.paths, Instant::now()),
                        // The OS watch failed: we can't trust the index any more.
                        Ok(Err(_)) => debouncer.overflow(Instant::now()),
                        Err(mpsc::RecvTimeoutError::Timeout) => {}
                        Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    }
                    if flag.load(Ordering::Relaxed) {
                        break;
                    }
                    if let Some(batch) = debouncer.poll(Instant::now()) {
                        on_batch(batch);
                    }
                }
            })
            .map_err(notify::Error::io)?;
        Ok(Watcher {
            _inner: inner,
            stopped,
        })
    }
}

impl Drop for Watcher {
    fn drop(&mut self) {
        // Dropping `_inner` closes the channel, which ends the thread.
        self.stopped.store(true, Ordering::Relaxed);
    }
}
