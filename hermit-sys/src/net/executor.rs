/// An executor, which is run when idling on network I/O.
use crate::net::network_delay;
use async_task::{Runnable, Task};
use concurrent_queue::ConcurrentQueue;
use futures_lite::pin;
use smoltcp::time::{Duration, Instant};
use std::sync::atomic::Ordering;
use std::{
	future::Future,
	sync::{atomic::AtomicBool, Arc},
	task::{Context, Poll, Wake},
};

/// A thread handle type
type Tid = u32;

extern "C" {
	fn sys_getpid() -> Tid;
	fn sys_yield();
	fn sys_wakeup_task(tid: Tid);
	fn sys_set_network_polling_mode(value: bool);
	fn sys_block_current_task_with_timeout(timeout: u64);
	fn sys_block_current_task();
}

lazy_static! {
	static ref QUEUE: ConcurrentQueue<Runnable> = ConcurrentQueue::unbounded();
}

fn run_executor() {
	while let Ok(runnable) = QUEUE.pop() {
		runnable.run();
	}
}

/// Spawns a future on the executor.
pub fn spawn<F, T>(future: F) -> Task<T>
where
	F: Future<Output = T> + Send + 'static,
	T: Send + 'static,
{
	let schedule = |runnable| QUEUE.push(runnable).unwrap();
	let (runnable, task) = async_task::spawn(future, schedule);
	runnable.schedule();
	task
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

impl Wake for ThreadNotify {
	fn wake(self: Arc<Self>) {
		self.wake_by_ref()
	}

	fn wake_by_ref(self: &Arc<Self>) {
		// Make sure the wakeup is remembered until the next `park()`.
		let unparked = self.unparked.swap(true, Ordering::Relaxed);
		if !unparked {
			unsafe {
				sys_wakeup_task(self.thread);
			}
		}
	}
}

thread_local! {
	static CURRENT_THREAD_NOTIFY: Arc<ThreadNotify> = Arc::new(ThreadNotify::new());
}

pub fn poll_on<F, T>(future: F, timeout: Option<Duration>) -> Result<T, ()>
where
	F: Future<Output = T>,
{
	CURRENT_THREAD_NOTIFY.with(|thread_notify| {
		unsafe {
			sys_set_network_polling_mode(true);
		}

		let start = Instant::now();
		let waker = thread_notify.clone().into();
		let mut cx = Context::from_waker(&waker);
		pin!(future);

		loop {
			if let Poll::Ready(t) = future.as_mut().poll(&mut cx) {
				unsafe {
					sys_set_network_polling_mode(false);
				}
				return Ok(t);
			}

			if let Some(duration) = timeout {
				if Instant::now() >= start + duration {
					unsafe {
						sys_set_network_polling_mode(false);
					}
					return Err(());
				}
			}

			run_executor()
		}
	})
}

/// Blocks the current thread on `f`, running the executor when idling.
pub fn block_on<F, T>(future: F, timeout: Option<Duration>) -> Result<T, ()>
where
	F: Future<Output = T>,
{
	CURRENT_THREAD_NOTIFY.with(|thread_notify| {
		let start = Instant::now();
		let waker = thread_notify.clone().into();
		let mut cx = Context::from_waker(&waker);
		pin!(future);

		loop {
			if let Poll::Ready(t) = future.as_mut().poll(&mut cx) {
				return Ok(t);
			}

			if let Some(duration) = timeout {
				if Instant::now() >= start + duration {
					return Err(());
				}
			}

			let delay = network_delay(Instant::now()).map(|d| d.total_millis());

			if delay.is_none() || delay.unwrap() > 100 {
				let unparked = thread_notify.unparked.swap(false, Ordering::Acquire);
				if !unparked {
					unsafe {
						match delay {
							Some(d) => sys_block_current_task_with_timeout(d),
							None => sys_block_current_task(),
						};
						sys_yield();
					}
					thread_notify.unparked.store(false, Ordering::Release);
					run_executor()
				}
			} else {
				run_executor()
			}
		}
	})
}
