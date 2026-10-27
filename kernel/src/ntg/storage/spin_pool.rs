//! A small fork-join thread pool for sub-millisecond jobs.
//!
//! One BitNet 2B decode token is 210 matrix products of roughly 250 microseconds each.
//! A general work-stealing pool pays two thread wake-ups per product (hand the job to a
//! worker, then block until it finishes), which on a virtual machine costs a large share of
//! the product itself. This pool keeps its workers spinning between jobs, lets the calling
//! thread run a share of the work, and has the caller spin (not block) on completion.
//!
//! Workers spin for a time budget (default 200 microseconds), then sleep on a condition
//! variable, so an idle pool does not burn a core indefinitely. The first job after a longer gap
//! pays a normal wake-up; [`SpinPool::prewake`] lets the caller overlap that with its own
//! preparation, and [`SpinPool::set_hold`] keeps the workers awake across a whole stretch of
//! back-to-back jobs (for example one model forward pass) whatever the gaps between them.
//!
//! [`SpinPool::run`] is blocking and serialised: concurrent callers take turns, and calling
//! `run` from inside a job on the same pool deadlocks.

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// How long an idle worker spins before it sleeps, in microseconds. A count of `spin_loop`
/// hints was tried first and was far too short on a VM (1,024 hints is a few microseconds).
pub const DEFAULT_SPIN_MICROS: u64 = 200;

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
    spin: Duration,
    /// While set, workers never sleep (see [`SpinPool::set_hold`]).
    hold: AtomicBool,
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
        Self::with_spin(threads, DEFAULT_SPIN_MICROS)
    }

    /// Like [`Self::new`], with an explicit idle-spin budget in microseconds. `0` sleeps
    /// immediately (each job then pays a wake-up); larger budgets keep workers hot but take
    /// CPU from other threads in the process while they wait.
    pub fn with_spin(threads: usize, spin_micros: u64) -> std::io::Result<Self> {
        let threads = threads.max(1);
        let shared = Arc::new(Shared {
            generation: AtomicUsize::new(0),
            pending: AtomicUsize::new(0),
            job: AtomicPtr::new(std::ptr::null_mut()),
            shutdown: AtomicBool::new(false),
            panicked: AtomicBool::new(false),
            spin: Duration::from_micros(spin_micros),
            hold: AtomicBool::new(false),
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

    /// Wake any sleeping workers now, ahead of a job that is a little way off, so their wake-up
    /// overlaps the caller's own preparation (for example quantising the activations) instead of
    /// adding to the dispatch. Workers that are already awake are untouched. A spurious call costs
    /// at most one idle-spin budget of CPU on each worker.
    pub fn prewake(&self) {
        let sh = &*self.shared;
        if sh.sleepers.load(Ordering::SeqCst) > 0 {
            let _g = sh.sleep_lock.lock().unwrap_or_else(|e| e.into_inner());
            sh.wake.notify_all();
        }
    }

    /// While `on`, workers never go to sleep: they keep spinning (yielding the CPU now and then)
    /// until the hold is released, then fall back to the time budget. Use it around a stretch of
    /// back-to-back jobs whose gaps are longer than the spin budget, such as one model forward
    /// pass. It costs a core per worker for as long as it is held, so release it afterwards.
    pub fn set_hold(&self, on: bool) {
        self.shared.hold.store(on, Ordering::SeqCst);
        if on {
            self.prewake();
        }
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
            if spins < 65_536 {
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
        // Wait for a new generation: spin for the time budget (indefinitely while held), then sleep.
        let mut deadline = Instant::now() + sh.spin;
        let mut polls = 0u32;
        loop {
            if sh.generation.load(Ordering::Acquire) != seen {
                break;
            }
            std::hint::spin_loop();
            polls = polls.wrapping_add(1);
            if !polls.is_multiple_of(64) {
                continue;
            }
            if sh.hold.load(Ordering::Relaxed) {
                // Held awake: be polite to other threads on this CPU, never sleep.
                if polls.is_multiple_of(4096) {
                    std::thread::yield_now();
                }
                deadline = Instant::now() + sh.spin;
                continue;
            }
            if Instant::now() >= deadline {
                let g = sh.sleep_lock.lock().unwrap_or_else(|e| e.into_inner());
                sh.sleepers.fetch_add(1, Ordering::SeqCst);
                if sh.generation.load(Ordering::SeqCst) == seen && !sh.hold.load(Ordering::SeqCst) {
                    drop(sh.wake.wait(g).unwrap_or_else(|e| e.into_inner()));
                } else {
                    drop(g);
                }
                sh.sleepers.fetch_sub(1, Ordering::SeqCst);
                deadline = Instant::now() + sh.spin;
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
    fn prewake_wakes_sleepers_and_jobs_still_run() {
        let pool = SpinPool::new(3).unwrap();
        pool.run(|_| {});
        std::thread::sleep(std::time::Duration::from_millis(300)); // let the workers sleep
        pool.prewake();
        pool.prewake(); // harmless when repeated or when nothing sleeps
        let n = AtomicUsize::new(0);
        pool.run(|_| {
            n.fetch_add(1, Ordering::Relaxed);
        });
        assert_eq!(n.load(Ordering::Relaxed), 3);
    }

    #[test]
    fn hold_keeps_jobs_running_and_release_lets_workers_sleep_again() {
        let pool = SpinPool::new(3).unwrap();
        pool.set_hold(true);
        let n = AtomicUsize::new(0);
        for _ in 0..50 {
            pool.run(|_| {
                n.fetch_add(1, Ordering::Relaxed);
            });
            std::thread::sleep(std::time::Duration::from_micros(500)); // longer than the spin budget
        }
        pool.set_hold(false);
        std::thread::sleep(std::time::Duration::from_millis(300)); // now they may sleep
        pool.run(|_| {
            n.fetch_add(1, Ordering::Relaxed);
        });
        assert_eq!(n.load(Ordering::Relaxed), 51 * 3);
    }

    #[test]
    fn zero_spin_budget_still_runs_jobs() {
        let pool = SpinPool::with_spin(4, 0).unwrap();
        let n = AtomicUsize::new(0);
        for _ in 0..100 {
            pool.run(|_| {
                n.fetch_add(1, Ordering::Relaxed);
            });
        }
        assert_eq!(n.load(Ordering::Relaxed), 400);
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
