use crate::net::{network_delay, network_poll};
use futures::task::{waker_ref, ArcWake, FutureObj, SpawnError};
use futures_util::pin_mut;
use smoltcp::time::{Duration, Instant};
use std::future::Future;
use std::sync::{
	atomic::{AtomicBool, AtomicUsize, Ordering},
	Arc, Mutex,
};
use std::task::{Context, Poll};

/// A thread handle type
type Tid = u32;

extern "C" {
	fn sys_getpid() -> u32;
	fn sys_yield();
	fn sys_wakeup_task(tid: Tid);
	fn sys_set_network_polling_mode(value: bool);
}

extern "Rust" {
	fn sys_block_current_task_with_timeout(timeout: Option<u64>);
}

thread_local! {
	static CURRENT_THREAD_NOTIFY: Arc<ThreadNotify> = {
		Arc::new(ThreadNotify::new())
	}
}

lazy_static! {
	static ref EXECUTOR: Mutex<SmoltcpExecutor> = Mutex::new(SmoltcpExecutor::new());
}

static NTHREADS_IN_EXECUTOR: ThreadsInExecutor = ThreadsInExecutor::new();

struct SmoltcpExecutor {
	pool: Vec<FutureObj<'static, ()>>,
}

impl SmoltcpExecutor {
	pub const fn new() -> Self {
		Self { pool: Vec::new() }
	}

	fn spawn_obj(&mut self, future: FutureObj<'static, ()>) -> Result<(), SpawnError> {
		self.pool.push(future);
		Ok(())
	}
}

/// Helper struct to determine, if the network interface
/// has to set in polling mode and to disable interrupts
/// from the network interface.
struct ThreadsInExecutor {
	/// number of threads in the executor
	nthreads: AtomicUsize,
	/// number of blocked threads
	blocked: AtomicUsize,
}

impl ThreadsInExecutor {
	pub const fn new() -> Self {
		Self {
			nthreads: AtomicUsize::new(0),
			blocked: AtomicUsize::new(0),
		}
	}

	pub fn increment(&self) {
		let old = self.nthreads.fetch_add(1, Ordering::SeqCst);
		if old + 1 > self.blocked.load(Ordering::SeqCst) {
			// a thread is waiting for message
			unsafe {
				sys_set_network_polling_mode(true);
			}
		}
	}

	pub fn decrement(&self) {
		let old = self.nthreads.fetch_sub(1, Ordering::SeqCst);
		if old - 1 <= self.blocked.load(Ordering::SeqCst) {
			// no thread is waiting for a message
			unsafe {
				sys_set_network_polling_mode(false);
			}
		}
	}

	pub fn unblock(&self) {
		let old = self.blocked.fetch_sub(1, Ordering::SeqCst);
		if self.nthreads.load(Ordering::SeqCst) > old - 1 {
			// a thread is waiting for message
			unsafe {
				sys_set_network_polling_mode(true);
			}
		}
	}

	pub fn block(&self) {
		let old = self.blocked.fetch_add(1, Ordering::SeqCst);
		if self.nthreads.load(Ordering::SeqCst) <= old + 1 {
			// no thread is waiting for a message
			unsafe {
				sys_set_network_polling_mode(false);
			}
		}
	}
}

struct ThreadNotify {
	/// The (single) executor thread.
	thread: Tid,
	/// A flag to ensure a wakeup is not "forgotten" before the next `block_current_task`
	unparked: AtomicBool,
}

impl ThreadNotify {
	pub fn new() -> Self {
		Self {
			thread: unsafe { sys_getpid() },
			unparked: AtomicBool::new(false),
		}
	}
}

impl Drop for ThreadNotify {
	fn drop(&mut self) {
		println!("Dropping ThreadNotify!");
	}
}

impl ArcWake for ThreadNotify {
	fn wake_by_ref(arc_self: &Arc<Self>) {
		// Make sure the wakeup is remembered until the next `park()`.
		let unparked = arc_self.unparked.swap(true, Ordering::Relaxed);
		if !unparked {
			unsafe {
				sys_wakeup_task(arc_self.thread);
			}
		}
	}
}

// Set up and run a basic single-threaded spawner loop, invoking `f` on each
// turn.
fn run_until<T, F: FnMut(&mut Context<'_>) -> Poll<T>>(
	mut f: F,
	timeout: Option<Duration>,
) -> Result<T, ()> {
	NTHREADS_IN_EXECUTOR.increment();
	let start = Instant::now();

	CURRENT_THREAD_NOTIFY.with(|thread_notify| {
		let waker = waker_ref(thread_notify);
		let mut cx = Context::from_waker(&waker);
		loop {
			if let Poll::Ready(t) = f(&mut cx) {
				NTHREADS_IN_EXECUTOR.decrement();
				return Ok(t);
			}

			if let Some(duration) = timeout {
				if Instant::now() >= start + duration {
					NTHREADS_IN_EXECUTOR.decrement();
					return Err(());
				}
			} else {
				let timestamp = Instant::now();
				let delay = network_delay(timestamp).map(|d| d.total_millis());

				if delay.is_none() || delay.unwrap() > 200 {
					let unparked = thread_notify.unparked.swap(false, Ordering::Acquire);
					if !unparked {
						NTHREADS_IN_EXECUTOR.block();
						unsafe {
							sys_block_current_task_with_timeout(delay);
							sys_yield();
						}
						NTHREADS_IN_EXECUTOR.unblock();
						thread_notify.unparked.store(false, Ordering::Release);
					}
				} else {
					network_poll(&mut cx, timestamp);
				}
			}
		}
	})
}

pub fn block_on<F: Future>(f: F, timeout: Option<Duration>) -> Result<F::Output, ()> {
	pin_mut!(f);
	run_until(|cx| f.as_mut().poll(cx), timeout)
}

pub fn spawn<F: Future<Output = ()> + std::marker::Send + 'static>(f: F) -> Result<(), SpawnError> {
	EXECUTOR
		.lock()
		.unwrap()
		.spawn_obj(FutureObj::from(Box::new(f)))
}
