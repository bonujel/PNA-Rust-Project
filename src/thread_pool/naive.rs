use super::ThreadPool;
use crate::Result;

/// A naive thread pool that spawns a new thread for every job.
///
/// This is the simplest possible "pool" — it doesn't reuse threads at all.
/// Useful as a baseline for benchmarking against real thread pools.
pub struct NaiveThreadPool;

impl ThreadPool for NaiveThreadPool {
    fn new(_threads: u32) -> Result<Self> {
        Ok(NaiveThreadPool)
    }

    fn spawn<F>(&self, job: F)
    where
        F: FnOnce() + Send + 'static,
    {
        std::thread::spawn(job);
    }
}
