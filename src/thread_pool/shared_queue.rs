use std::thread;

use crossbeam::channel::{self, Receiver, Sender};
use log::{debug, error};

use super::ThreadPool;
use crate::Result;

/// A thread pool using a shared job queue.
///
/// Workers pull jobs from a single MPMC channel. If a worker thread
/// panics, a new one is spawned to replace it.
pub struct SharedQueueThreadPool {
    tx: Sender<Box<dyn FnOnce() + Send + 'static>>,
}

impl ThreadPool for SharedQueueThreadPool {
    fn new(threads: u32) -> Result<Self> {
        let (tx, rx) = channel::unbounded::<Box<dyn FnOnce() + Send + 'static>>();

        for id in 0..threads {
            let rx = rx.clone();
            spawn_worker(id, rx);
        }

        Ok(SharedQueueThreadPool { tx })
    }

    fn spawn<F>(&self, job: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.tx
            .send(Box::new(job))
            .expect("thread pool has no active threads");
    }
}

/// Spawns a single worker thread that pulls jobs from the receiver.
/// If the worker panics, a replacement is spawned automatically.
fn spawn_worker(id: u32, rx: Receiver<Box<dyn FnOnce() + Send + 'static>>) {
    thread::Builder::new()
        .name(format!("pool-worker-{id}"))
        .spawn(move || {
            loop {
                match rx.recv() {
                    Ok(job) => {
                        debug!("Worker {id} executing job");
                        // Catch panics so the worker loop continues
                        if std::panic::catch_unwind(std::panic::AssertUnwindSafe(job)).is_err() {
                            error!("Worker {id} job panicked, continuing");
                        }
                    }
                    Err(_) => {
                        debug!("Worker {id}: channel closed, shutting down");
                        return;
                    }
                }
            }
        })
        .expect("failed to spawn worker thread");
}

impl Drop for SharedQueueThreadPool {
    fn drop(&mut self) {
        // Dropping the sender closes the channel, causing workers to exit
    }
}
