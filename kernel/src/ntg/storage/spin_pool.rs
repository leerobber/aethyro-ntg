//! A small fork-join thread pool for sub-millisecond jobs.
//!
//! One BitNet 2B decode token is 210 matrix products of roughly 250 microseconds each.
//! A general work-stealing pool pays two thread wake-ups per product (hand the job to a
//! worker, then block until it finishes), which on a virtual machine costs a large share of
//! the product itself. This pool keeps its workers spinning between jobs, lets the calling
//! thread run a share of the work, and has the caller spin (not block) on completion.
//!
//! Workers spin for a short budget, then yield, then sleep on a condition variable, so an
//! idle pool does not burn a core indefinitely. The first job after a long gap pays a normal
//! wake-up.
//!
//! [`SpinPool::run`] is blocking and serialised: concurrent callers take turns, and calling
//! `run` from inside a job on the same pool deadlocks.

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;

/// Busy-wait iterations (`spin_loop` hints) before a worker starts yielding.
pub const DEFAULT_SPIN_ITERS: u32 = 1_024;
/// `yield_now` calls before a worker sleeps on the condition variable.
pub const DEFAULT_YIELD_ITERS: u32 = 0;

/// Type-erased pointer to the job closure on the caller's stack.
#[derive(Clone, Copy)]
struct Job {
    data: *const (),
    call: unsafe fn(*const (), usize),
}

struct Shared {
    /// Bumped once per job (and on shutdown); workers wait for it to change.
    generation: AtomicUsize,
    /// Workers that have not finished the current job.
    pending: AtomicUsize,
    /// The current job, valid from the `generation` bump until `pending` reaches zero.
    job: AtomicPtr<Job>,
    shutdown: AtomicBool,
    panicked: AtomicBool,
    spin_iters: u32,
    yield_iters: u32,
    sleepers: AtomicUsize,
    sleep_lock: Mutex<()>,
    wake: Condvar,
}

pub struct SpinPool {
    shared: Arc<Shared>,
    workers: Vec<JoinHandle<()>>,
    threads: usize,
    run_lock: Mutex<()>,
}

// SAFETY: the raw pointers in `Shared` are only dereferenced under the protocol described in
// `run`, which keeps the pointee alive until every worker has finished with it.
unsafe impl Send for SpinPool {}
unsafe impl Sync for SpinPool {}

unsafe fn call_closure<F: Fn(usize) + Sync>(data: *const (), t: usize) {
    (*(data as *const F))(t)
}

impl SpinPool {
    /// A pool that runs jobs on `threads` threads in total: the caller plus `threads - 1`
    /// workers. `threads == 0` is treated as 1.
    pub fn new(threads: usize) -> std::io::Result<Self> {
        Self::with_budget(threads, DEFAULT_SPIN_ITERS, DEFAULT_YIELD_ITERS)
    }

    /// Like [`Self::new`], with explicit idle budgets: how many busy-wait iterations and then
    /// `yield_now` calls a worker makes before it sleeps. `(0, 0)` sleeps immediately (each
    /// job then pays a wake-up); larger budgets keep workers hot but take CPU from other
    /// threads in the process while they wait.
    pub fn with_budget(threads: usize, spin_iters: u32, yield_iters: u32) -> std::io::Result<Self> {
        let threads = threads.max(1);
        let shared = Arc::new(Shared {
            generation: AtomicUsize::new(0),
            pending: AtomicUsize::new(0),
            job: AtomicPtr::new(std::ptr::null_mut()),
            shutdown: AtomicBool::new(false),
            panicked: AtomicBool::new(false),
            spin_iters,
            yield_iters,
            sleepers: AtomicUsize::new(0),
            sleep_lock: Mutex::new(()),
            wake: Condvar::new(),
        });
        let mut workers = Vec::with_capacity(threads - 1);
        for t in 1..threads {
            let sh = Arc::clone(&shared);
            let spawned = std::thread::Builder::new()
                .name(format!("ntg-spin-{t}"))
                .spawn(move || worker_loop(&sh, t));
            match spawned {
                Ok(h) => workers.push(h),
                Err(e) => {
                    stop(&shared, &mut workers);
                    return Err(e);
                }
            }
        }
        Ok(Self { shared, workers, threads, run_lock: Mutex::new(()) })
    }

    /// Total threads that take part in a job (workers plus the caller).
    pub fn threads(&self) -> usize {
        self.threads
    }

    /// Run `f(t)` once for every `t` in `0..self.threads()`, concurrently, and return when
    /// all calls have finished. The caller runs `f(0)`. Panics in `f` are re-raised here
    /// after every thread has stopped touching the borrowed data.
    pub fn run<F: Fn(usize) + Sync>(&self, f: F) {
        if self.threads == 1 {
            f(0);
            return;
        }
        let _turn = self.run_lock.lock().unwrap_or_else(|e| e.into_inner());
        let sh = &*self.shared;
        let mut job = Job { data: &f as *const F as *const (), call: call_closure::<F> };

        sh.panicked.store(false, Ordering::Relaxed);
        sh.pending.store(self.threads - 1, Ordering::Relaxed);
        sh.job.store(&mut job, Ordering::Relaxed);
        // Release: publishes `job`, `pending` and everything `f` borrows to the workers.
        sh.generation.fetch_add(1, Ordering::SeqCst);
        if sh.sleepers.load(Ordering::SeqCst) > 0 {
            let _g = sh.sleep_lock.lock().unwrap_or_else(|e| e.into_inner());
            sh.wake.notify_all();
        }

        let mine = catch_unwind(AssertUnwindSafe(|| f(0)));

        // Wait for the workers before `job` and `f` go out of scope.
        let mut spins = 0u32;
        while sh.pending.load(Ordering::Acquire) != 0 {
            if spins < DEFAULT_SPIN_ITERS * 64 {
                std::hint::spin_loop();
                spins += 1;
            } else {
                std::thread::yield_now();
            }
        }
        if let Err(p) = mine {
            std::panic::resume_unwind(p);
        }
        if sh.panicked.load(Ordering::Relaxed) {
            panic!("a SpinPool worker panicked");
        }
    }
}

fn worker_loop(sh: &Shared, t: usize) {
    let mut seen = 0usize;
    loop {
        // Wait for a new generation: spin, then yield, then sleep.
        let mut spins = 0u32;
        let mut yields = 0u32;
        loop {
            if sh.generation.load(Ordering::Acquire) != seen {
                break;
            }
            if spins < sh.spin_iters {
                std::hint::spin_loop();
                spins += 1;
            } else if yields < sh.yield_iters {
                std::thread::yield_now();
                yields += 1;
            } else {
                let g = sh.sleep_lock.lock().unwrap_or_else(|e| e.into_inner());
                sh.sleepers.fetch_add(1, Ordering::SeqCst);
                if sh.generation.load(Ordering::SeqCst) == seen {
                    drop(sh.wake.wait(g).unwrap_or_else(|e| e.into_inner()));
                } else {
                    drop(g);
                }
                sh.sleepers.fetch_sub(1, Ordering::SeqCst);
                spins = 0;
                yields = 0;
            }
        }
        seen = sh.generation.load(Ordering::Acquire);
        if sh.shutdown.load(Ordering::Acquire) {
            return;
        }
        // SAFETY: `run` stored a pointer to a live `Job` before bumping `generation` and does
        // not return (so the `Job` and the closure stay alive) until `pending` is zero.
        let job = unsafe { *sh.job.load(Ordering::Relaxed) };
        let ran = catch_unwind(AssertUnwindSafe(|| unsafe { (job.call)(job.data, t) }));
        if ran.is_err() {
            sh.panicked.store(true, Ordering::Relaxed);
        }
        sh.pending.fetch_sub(1, Ordering::AcqRel);
    }
}

fn stop(shared: &Shared, workers: &mut Vec<JoinHandle<()>>) {
    shared.shutdown.store(true, Ordering::SeqCst);
    shared.generation.fetch_add(1, Ordering::SeqCst);
    {
        let _g = shared.sleep_lock.lock().unwrap_or_else(|e| e.into_inner());
        shared.wake.notify_all();
    }
    for h in workers.drain(..) {
        let _ = h.join();
    }
}

impl Drop for SpinPool {
    fn drop(&mut self) {
        stop(&self.shared, &mut self.workers);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicU64;

    #[test]
    fn every_thread_index_runs_exactly_once_per_job() {
        let pool = SpinPool::new(4).unwrap();
        for round in 0..200u64 {
            let hits: Vec<AtomicU64> = (0..4).map(|_| AtomicU64::new(0)).collect();
            pool.run(|t| {
                hits[t].fetch_add(round + 1, Ordering::Relaxed);
            });
            for h in &hits {
                assert_eq!(h.load(Ordering::Relaxed), round + 1);
            }
        }
    }

    #[test]
    fn single_thread_pool_runs_inline() {
        let pool = SpinPool::new(1).unwrap();
        let n = AtomicUsize::new(0);
        pool.run(|t| {
            assert_eq!(t, 0);
            n.fetch_add(1, Ordering::Relaxed);
        });
        assert_eq!(n.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn workers_that_went_to_sleep_wake_up() {
        let pool = SpinPool::new(3).unwrap();
        let n = AtomicUsize::new(0);
        pool.run(|_| {
            n.fetch_add(1, Ordering::Relaxed);
        });
        // Long enough for the spin and yield budgets to run out and the workers to sleep.
        std::thread::sleep(std::time::Duration::from_millis(300));
        pool.run(|_| {
            n.fetch_add(1, Ordering::Relaxed);
        });
        assert_eq!(n.load(Ordering::Relaxed), 6);
    }

    #[test]
    fn concurrent_callers_take_turns() {
        let pool = Arc::new(SpinPool::new(3).unwrap());
        let total = Arc::new(AtomicUsize::new(0));
        let handles: Vec<_> = (0..4)
            .map(|_| {
                let (pool, total) = (Arc::clone(&pool), Arc::clone(&total));
                std::thread::spawn(move || {
                    for _ in 0..100 {
                        pool.run(|_| {
                            total.fetch_add(1, Ordering::Relaxed);
                        });
                    }
                })
            })
            .collect();
        for h in handles {
            h.join().unwrap();
        }
        assert_eq!(total.load(Ordering::Relaxed), 4 * 100 * 3);
    }

    #[test]
    fn a_panicking_job_is_reported_and_the_pool_survives() {
        let pool = SpinPool::new(3).unwrap();
        let r = catch_unwind(AssertUnwindSafe(|| {
            pool.run(|t| {
                if t == 2 {
                    panic!("boom");
                }
            })
        }));
        assert!(r.is_err());
        let n = AtomicUsize::new(0);
        pool.run(|_| {
            n.fetch_add(1, Ordering::Relaxed);
        });
        assert_eq!(n.load(Ordering::Relaxed), 3);
    }
}
