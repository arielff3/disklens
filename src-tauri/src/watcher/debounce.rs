//! Collects changed paths and decides when to hand them over as one batch:
//! after a quiet period, or after a maximum wait when changes never stop.

use std::collections::HashSet;
use std::path::PathBuf;
use std::time::{Duration, Instant};

#[derive(Debug, PartialEq)]
pub enum Batch {
    /// Paths that changed, deduplicated.
    Paths(Vec<PathBuf>),
    /// Events were lost; only a rescan can bring the index back in sync.
    Overflow,
}

pub struct Debouncer {
    quiet: Duration,
    max_wait: Duration,
    paths: Vec<PathBuf>,
    seen: HashSet<PathBuf>,
    overflow: bool,
    first: Option<Instant>,
    last: Option<Instant>,
}

impl Debouncer {
    pub fn new(quiet: Duration, max_wait: Duration) -> Debouncer {
        Debouncer {
            quiet,
            max_wait,
            paths: Vec::new(),
            seen: HashSet::new(),
            overflow: false,
            first: None,
            last: None,
        }
    }

    pub fn push(&mut self, paths: impl IntoIterator<Item = PathBuf>, now: Instant) {
        if !self.overflow {
            for p in paths {
                if self.seen.insert(p.clone()) {
                    self.paths.push(p);
                }
            }
        }
        self.touch(now);
    }

    /// Events were lost. Pending paths no longer matter: the rescan covers them.
    pub fn overflow(&mut self, now: Instant) {
        self.overflow = true;
        self.paths.clear();
        self.seen.clear();
        self.touch(now);
    }

    /// When the pending batch is due, if there is one.
    pub fn deadline(&self) -> Option<Instant> {
        Some((self.last? + self.quiet).min(self.first? + self.max_wait))
    }

    /// Hands over the pending batch if it is due.
    pub fn poll(&mut self, now: Instant) -> Option<Batch> {
        if now < self.deadline()? {
            return None;
        }
        self.first = None;
        self.last = None;
        self.seen.clear();
        if std::mem::take(&mut self.overflow) {
            self.paths.clear();
            return Some(Batch::Overflow);
        }
        let paths = std::mem::take(&mut self.paths);
        (!paths.is_empty()).then_some(Batch::Paths(paths))
    }

    fn touch(&mut self, now: Instant) {
        self.first.get_or_insert(now);
        self.last = Some(now);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const QUIET: Duration = Duration::from_millis(500);
    const MAX: Duration = Duration::from_millis(2000);

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    fn p(s: &str) -> PathBuf {
        PathBuf::from(s)
    }

    #[test]
    fn nothing_pending_means_nothing_to_flush() {
        let mut d = Debouncer::new(QUIET, MAX);
        assert_eq!(d.deadline(), None);
        assert_eq!(d.poll(Instant::now()), None);
    }

    #[test]
    fn flushes_after_a_quiet_period_with_duplicates_removed() {
        let t0 = Instant::now();
        let mut d = Debouncer::new(QUIET, MAX);
        d.push([p("a"), p("b")], t0);
        d.push([p("a")], t0 + ms(100));
        assert_eq!(d.deadline(), Some(t0 + ms(600)));
        assert_eq!(d.poll(t0 + ms(599)), None);
        assert_eq!(
            d.poll(t0 + ms(600)),
            Some(Batch::Paths(vec![p("a"), p("b")]))
        );
        assert_eq!(d.poll(t0 + ms(5000)), None);
        // A path flushed earlier is reported again the next time it changes.
        d.push([p("a")], t0 + ms(6000));
        assert_eq!(d.poll(t0 + ms(6500)), Some(Batch::Paths(vec![p("a")])));
    }

    #[test]
    fn continuous_changes_flush_at_the_max_wait() {
        let t0 = Instant::now();
        let mut d = Debouncer::new(QUIET, MAX);
        for i in 0..30 {
            d.push([p(&format!("f{i}"))], t0 + ms(i * 100));
            if i * 100 < 2000 {
                assert_eq!(d.poll(t0 + ms(i * 100)), None, "at {}ms", i * 100);
            }
        }
        let Some(Batch::Paths(paths)) = d.poll(t0 + ms(2900)) else {
            panic!("expected a batch")
        };
        assert_eq!(paths.len(), 30);
    }

    #[test]
    fn overflow_replaces_pending_paths() {
        let t0 = Instant::now();
        let mut d = Debouncer::new(QUIET, MAX);
        d.push([p("a")], t0);
        d.overflow(t0 + ms(10));
        d.push([p("b")], t0 + ms(20));
        assert_eq!(d.poll(t0 + ms(520)), Some(Batch::Overflow));
        d.push([p("c")], t0 + ms(1000));
        assert_eq!(d.poll(t0 + ms(1500)), Some(Batch::Paths(vec![p("c")])));
    }
}
