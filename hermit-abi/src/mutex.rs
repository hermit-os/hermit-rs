#[cfg(not(feature = "rustc-dep-of-std"))]
extern crate alloc;

use crate::{
	block_current_task, get_priority, getpid, set_priority, wakeup_task, yield_now, Priority, Tid,
	NO_PRIORITIES,
};
use alloc::collections::vec_deque::VecDeque;
use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut, Drop};
use core::sync::atomic::{AtomicUsize, Ordering};
use core::{hint, mem};

/// This type provides a lock based on busy waiting to realize mutual exclusion
///
/// # Description
///
/// This structure behaves a lot like a common mutex. There are some differences:
///
/// - By using busy waiting, it can be used outside the runtime.
/// - It is a so called ticket lock and is completely fair.
#[cfg_attr(target_arch = "x86_64", repr(align(128)))]
#[cfg_attr(not(target_arch = "x86_64"), repr(align(64)))]
struct Spinlock<T: ?Sized> {
	queue: AtomicUsize,
	dequeue: AtomicUsize,
	data: UnsafeCell<T>,
}

unsafe impl<T: ?Sized + Send> Sync for Spinlock<T> {}
unsafe impl<T: ?Sized + Send> Send for Spinlock<T> {}

/// A guard to which the protected data can be accessed
///
/// When the guard falls out of scope it will release the lock.
struct SpinlockGuard<'a, T: ?Sized> {
	dequeue: &'a AtomicUsize,
	data: &'a mut T,
}

impl<T> Spinlock<T> {
	pub const fn new(user_data: T) -> Spinlock<T> {
		Spinlock {
			queue: AtomicUsize::new(0),
			dequeue: AtomicUsize::new(1),
			data: UnsafeCell::new(user_data),
		}
	}

	#[inline]
	fn obtain_lock(&self) {
		let ticket = self.queue.fetch_add(1, Ordering::SeqCst) + 1;
		let mut counter: u16 = 0;
		while self.dequeue.load(Ordering::SeqCst) != ticket {
			counter += 1;
			if counter < 100 {
				hint::spin_loop();
			} else {
				counter = 0;
				unsafe {
					yield_now();
				}
			}
		}
	}

	#[inline]
	pub unsafe fn lock(&self) -> SpinlockGuard<'_, T> {
		self.obtain_lock();
		SpinlockGuard {
			dequeue: &self.dequeue,
			data: &mut *self.data.get(),
		}
	}
}

impl<T: ?Sized + Default> Default for Spinlock<T> {
	fn default() -> Spinlock<T> {
		Spinlock::new(Default::default())
	}
}

impl<'a, T: ?Sized> Deref for SpinlockGuard<'a, T> {
	type Target = T;
	fn deref(&self) -> &T {
		&*self.data
	}
}

impl<'a, T: ?Sized> DerefMut for SpinlockGuard<'a, T> {
	fn deref_mut(&mut self) -> &mut T {
		&mut *self.data
	}
}

impl<'a, T: ?Sized> Drop for SpinlockGuard<'a, T> {
	/// The dropping of the SpinlockGuard will release the lock it was created from.
	fn drop(&mut self) {
		self.dequeue.fetch_add(1, Ordering::SeqCst);
	}
}

/// Realize a priority queue for tasks
struct PriorityQueue {
	queues: [Option<VecDeque<Tid>>; NO_PRIORITIES],
	prio_bitmap: u64,
}

impl PriorityQueue {
	pub const fn new() -> PriorityQueue {
		PriorityQueue {
			queues: [
				None, None, None, None, None, None, None, None, None, None, None, None, None, None,
				None, None, None, None, None, None, None, None, None, None, None, None, None, None,
				None, None, None,
			],
			prio_bitmap: 0,
		}
	}

	/// Add a task id by its priority to the queue
	pub fn push(&mut self, prio: Priority, id: Tid) {
		let i: usize = prio.into().into();
		self.prio_bitmap |= (1 << i) as u64;
		if let Some(queue) = &mut self.queues[i] {
			queue.push_back(id);
		} else {
			let mut queue = VecDeque::new();
			queue.push_back(id);
			self.queues[i] = Some(queue);
		}
	}

	fn pop_from_queue(&mut self, queue_index: usize) -> Option<Tid> {
		if let Some(queue) = &mut self.queues[queue_index] {
			let id = queue.pop_front();

			if queue.is_empty() {
				self.prio_bitmap &= !(1 << queue_index as u64);
			}

			id
		} else {
			None
		}
	}

	/// Pop the task handle with the highest priority from the queue
	pub fn pop(&mut self) -> Option<Tid> {
		for i in 0..NO_PRIORITIES {
			if self.prio_bitmap & (1 << i) != 0 {
				return self.pop_from_queue(i);
			}
		}

		None
	}
}

enum MutexState {
	Unlocked,
	Locked {
		/// Identifies the task.
		id: Tid,
		/// Current priority of the task, which holds the lock
		current_prio: Priority,
		/// Original priority of the task, which holds the lock
		base_prio: Priority,
	},
}

struct MutexInner {
	state: MutexState,
	/// Priority queue of blocked tasks
	blocked_tasks: PriorityQueue,
}

impl MutexInner {
	pub const fn new() -> MutexInner {
		Self {
			state: MutexState::Unlocked,
			blocked_tasks: PriorityQueue::new(),
		}
	}
}

pub struct Mutex {
	inner: Spinlock<MutexInner>,
}

unsafe impl Send for Mutex {}
unsafe impl Sync for Mutex {}

impl Mutex {
	pub const fn new() -> Mutex {
		Mutex {
			inner: Spinlock::new(MutexInner::new()),
		}
	}

	#[inline]
	pub unsafe fn init(&mut self) {
		self.inner = Spinlock::new(MutexInner::new());
	}

	#[inline]
	pub unsafe fn lock(&self) {
		loop {
			let mut guard = self.inner.lock();
			match guard.state {
				MutexState::Unlocked => {
					let prio = get_priority();
					guard.state = MutexState::Locked {
						id: getpid(),
						current_prio: prio,
						base_prio: prio,
					}
				}
				MutexState::Locked {
					id,
					ref mut current_prio,
					base_prio: _,
				} => {
					let prio = get_priority();

					if *current_prio < prio {
						set_priority(id, prio);
						*current_prio = prio;
					}
					guard.blocked_tasks.push(prio, getpid());
					block_current_task();
					drop(guard);
					yield_now();
				}
			}
		}
	}

	#[inline]
	pub unsafe fn unlock(&self) {
		let mut guard = self.inner.lock();

		if let MutexState::Locked {
			id,
			current_prio,
			base_prio,
		} = mem::replace(&mut guard.state, MutexState::Unlocked)
		{
			if current_prio != base_prio {
				set_priority(id, base_prio);
			}

			guard.state = MutexState::Unlocked;

			if let Some(tid) = guard.blocked_tasks.pop() {
				wakeup_task(tid);
			}
		}
	}

	#[inline]
	pub unsafe fn try_lock(&self) -> bool {
		let mut guard = self.inner.lock();

		if matches!(guard.state, MutexState::Unlocked) {
			let prio = get_priority();
			guard.state = MutexState::Locked {
				id: getpid(),
				current_prio: prio,
				base_prio: prio,
			};

			true
		} else {
			false
		}
	}

	#[inline]
	pub unsafe fn destroy(&self) {}
}
