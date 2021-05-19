use crate::net::{network_delay, network_poll};
use futures::pin_mut;
use futures::task::{waker_ref, ArcWake, FutureObj, SpawnError};
use smoltcp::time::{Duration, Instant};
use std::future::Future;
use std::sync::{
	atomic::{AtomicBool, Ordering},
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
	unsafe {
		sys_set_network_polling_mode(true);
	}
	let start = Instant::now();

	CURRENT_THREAD_NOTIFY.with(|thread_notify| {
		let waker = waker_ref(thread_notify);
		let mut cx = Context::from_waker(&waker);
		loop {
			if let Poll::Ready(t) = f(&mut cx) {
				unsafe {
					sys_set_network_polling_mode(false);
				}
				return Ok(t);
			}

			let now = Instant::now();

			if let Some(duration) = timeout {
				if now >= start + duration {
					unsafe {
						sys_set_network_polling_mode(false);
					}
					return Err(());
				}
			} else {
				let delay = network_delay(now).map(|d| d.total_millis());

				if delay.is_none() || delay.unwrap() > 100 {
					let unparked = thread_notify.unparked.swap(false, Ordering::Acquire);
					if !unparked {
						unsafe {
							sys_set_network_polling_mode(false);
							sys_block_current_task_with_timeout(delay);
							sys_yield();
							sys_set_network_polling_mode(true);
						}
						thread_notify.unparked.store(false, Ordering::Release);
						network_poll(&mut cx, Instant::now());
					}
				} else {
					network_poll(&mut cx, now);
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
