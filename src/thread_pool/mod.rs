use crate::Result;

/// A thread pool for executing jobs concurrently.
///
/// Implementors manage a pool of worker threads and distribute
/// incoming jobs across them.
pub trait ThreadPool {
    /// Creates a new thread pool with the given number of threads.
    ///
    /// # Errors
    ///
    /// Returns an error if the pool cannot be created (e.g., invalid size).
    fn new(threads: u32) -> Result<Self>
    where
        Self: Sized;

    /// Spawns a function into the thread pool.
    ///
    /// The function will be executed by one of the threads in the pool.
    fn spawn<F>(&self, job: F)
    where
        F: FnOnce() + Send + 'static;
}

mod naive;
mod rayon_pool;
mod shared_queue;

pub use self::naive::NaiveThreadPool;
pub use self::rayon_pool::RayonThreadPool;
pub use self::shared_queue::SharedQueueThreadPool;
