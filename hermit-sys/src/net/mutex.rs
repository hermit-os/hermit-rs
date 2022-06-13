use hermit_abi as abi;

use std::cell::UnsafeCell;
use std::ops::{Deref, DerefMut};

/// To use the latest version of RustyHermit's mutex implementation,
/// the network interface comes with an own RAII implementation of
/// a "scoped lock", which based on the hermit-abi's mutex implementation
pub(crate) struct Mutex<T: ?Sized> {
	inner: abi::mutex::Mutex,
	data: UnsafeCell<T>,
}

unsafe impl<T: ?Sized + Send> Send for Mutex<T> {}
unsafe impl<T: ?Sized + Send> Sync for Mutex<T> {}

/// A guard to which the protected data can be accessed
///
/// When the guard falls out of scope it will release the lock.
pub(crate) struct MutexGuard<'a, T: ?Sized + 'a> {
	inner: &'a abi::mutex::Mutex,
	data: &'a mut T,
}

impl<T> Mutex<T> {
	pub fn new(t: T) -> Mutex<T> {
		Mutex {
			inner: abi::mutex::Mutex::new(),
			data: UnsafeCell::new(t),
		}
	}
}

impl<T: ?Sized> Mutex<T> {
	pub fn lock(&self) -> MutexGuard<'_, T> {
		unsafe {
			self.inner.lock();
		}
		MutexGuard {
			inner: &self.inner,
			data: unsafe { &mut *self.data.get() },
		}
	}
}

impl<T: ?Sized + Default> Default for Mutex<T> {
	fn default() -> Mutex<T> {
		Mutex::new(Default::default())
	}
}

impl<'a, T: ?Sized> Deref for MutexGuard<'a, T> {
	type Target = T;
	fn deref(&self) -> &T {
		&*self.data
	}
}

impl<'a, T: ?Sized> DerefMut for MutexGuard<'a, T> {
	fn deref_mut(&mut self) -> &mut T {
		&mut *self.data
	}
}

impl<'a, T: ?Sized> Drop for MutexGuard<'a, T> {
	fn drop(&mut self) {
		unsafe {
			self.inner.unlock();
		}
	}
}
