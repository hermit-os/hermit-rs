use hermit_abi as abi;

use std::cell::UnsafeCell;

/// To use the latest version of RustyHermit's mutex implementation,
/// the network interface comes with an own lambda implementation,
/// which based on the hermit-abi's mutex implementation
pub(crate) struct Mutex<T: ?Sized> {
	inner: abi::mutex::Mutex,
	data: UnsafeCell<T>,
}

unsafe impl<T: ?Sized + Send> Send for Mutex<T> {}
unsafe impl<T: ?Sized + Send> Sync for Mutex<T> {}

impl<T> Mutex<T> {
	pub fn new(t: T) -> Mutex<T> {
		Mutex {
			inner: abi::mutex::Mutex::new(),
			data: UnsafeCell::new(t),
		}
	}
}

impl<T: ?Sized> Mutex<T> {
	pub fn lock<RET>(&self, f: impl FnOnce(&mut T) -> RET) -> RET {
		unsafe {
			self.inner.lock();

			let ret = f(&mut *self.data.get());

			self.inner.unlock();

			ret
		}
	}
}

impl<T: ?Sized + Default> Default for Mutex<T> {
	fn default() -> Mutex<T> {
		Mutex::new(Default::default())
	}
}
