/// The waker registration is derived from smoltcp waker registration
use core::task::Waker;

/// Utility struct to register and wake a waker.
#[derive(Debug)]
pub(crate) struct WakerRegistration {
	waker: Option<Waker>,
}

impl WakerRegistration {
	pub(crate) const fn new() -> Self {
		Self { waker: None }
	}

	/// Register a waker. Overwrites the previous waker, if any.
	pub(crate) fn register(&mut self, w: &Waker) {
		match self.waker {
			// Optimization: If both the old and new Wakers wake the same task, we can simply
			// keep the old waker, skipping the clone. (In most executor implementations,
			// cloning a waker is somewhat expensive, comparable to cloning an Arc).
			Some(ref w2) if (w2.will_wake(w)) => {}
			_ => {
				// clone the new waker and store it
				if let Some(old_waker) = std::mem::replace(&mut self.waker, Some(w.clone())) {
					// We had a waker registered for another task. Wake it, so the other task can
					// reregister itself if it's still interested.
					//
					// If two tasks are waiting on the same thing concurrently, this will cause them
					// to wake each other in a loop fighting over this WakerRegistration. This wastes
					// CPU but things will still work.
					//
					// If the user wants to have two tasks waiting on the same thing they should use
					// a more appropriate primitive that can store multiple wakers.
					old_waker.wake()
				}
			}
		}
	}

	/// Wake the registered waker, if any.
	pub(crate) fn wake(&mut self) {
		self.waker.take().map(|w| w.wake());
	}
}
