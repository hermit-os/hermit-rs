#[cfg(not(feature = "rustc-dep-of-std"))]
extern crate alloc;

use crate::{
	block_current_task, get_priority, getpid, wakeup_task, yield_now, Priority, Tid, NO_PRIORITIES,
};
use alloc::collections::vec_deque::VecDeque;
use core::cell::UnsafeCell;
use core::hint;
use core::ops::{Deref, DerefMut, Drop};
use core::sync::atomic::{AtomicBool, Ordering};

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
	inner: AtomicBool,
	data: UnsafeCell<T>,
}

unsafe impl<T: ?Sized + Send> Sync for Spinlock<T> {}
unsafe impl<T: ?Sized + Send> Send for Spinlock<T> {}

/// A guard to which the protected data can be accessed
///
/// When the guard falls out of scope it will release the lock.
struct SpinlockGuard<'a, T: ?Sized> {
	inner: &'a AtomicBool,
	data: &'a mut T,
}

impl<T> Spinlock<T> {
	pub const fn new(user_data: T) -> Spinlock<T> {
		Spinlock {
			inner: AtomicBool::new(false),
			data: UnsafeCell::new(user_data),
		}
	}

	#[inline]
	fn obtain_lock(&self) {
		let mut counter: u16 = 0;
		while self.inner.swap(true, Ordering::SeqCst) {
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
	fn try_obtain_lock(&self) -> bool {
		!self.inner.swap(true, Ordering::SeqCst)
	}

	#[inline]
	pub unsafe fn lock(&self) -> SpinlockGuard<'_, T> {
		self.obtain_lock();
		SpinlockGuard {
			inner: &self.inner,
			data: &mut *self.data.get(),
		}
	}

	#[inline]
	pub unsafe fn try_lock(&self) -> Result<SpinlockGuard<'_, T>, ()> {
		if self.try_obtain_lock() {
			Ok(SpinlockGuard {
				inner: &self.inner,
				data: &mut *self.data.get(),
			})
		} else {
			Err(())
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
		self.inner.swap(false, Ordering::SeqCst);
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

struct MutexInner {
	locked: bool,
	/// Priority queue of blocked tasks
	blocked_tasks: PriorityQueue,
}

impl MutexInner {
	pub const fn new() -> MutexInner {
		Self {
			locked: false,
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
	pub unsafe fn lock(&self) {
		loop {
			let mut guard = self.inner.lock();
			if !guard.locked {
				guard.locked = true;
				return;
			} else {
				let prio = get_priority();
				let id = getpid();

				guard.blocked_tasks.push(prio, id);
				block_current_task();
				drop(guard);
				yield_now();
			}
		}
	}

	#[inline]
	pub unsafe fn unlock(&self) {
		let mut guard = self.inner.lock();
		guard.locked = false;
		if let Some(tid) = guard.blocked_tasks.pop() {
			wakeup_task(tid);
		}
	}

	#[inline]
	pub unsafe fn try_lock(&self) -> bool {
		if let Ok(mut guard) = self.inner.try_lock() {
			if !guard.locked {
				guard.locked = true;

				true
			} else {
				false
			}
		} else {
			false
		}
	}
}
