use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::{hint, thread};

#[cfg(target_os = "hermit")]
use hermit as _;

const NUMBER_OF_ITERATIONS: usize = 1000;

pub struct SpinBarrier {
	num_threads: AtomicUsize,
}

impl SpinBarrier {
	pub const fn new(n: usize) -> Self {
		Self {
			num_threads: AtomicUsize::new(n),
		}
	}

	pub fn wait(&self) {
		self.num_threads.fetch_sub(1, Ordering::Relaxed);
		while self.num_threads.load(Ordering::Relaxed) != 0 {
			hint::spin_loop();
		}
	}
}

fn main() {
	let counter = Arc::new(Mutex::new(0));
	let available_parallelism = thread::available_parallelism().unwrap().get();
	println!("available_parallelism = {available_parallelism}");

	let barrier = Arc::new(SpinBarrier::new(available_parallelism));

	let handles = (0..available_parallelism)
		.map(|_| {
			let barrier = barrier.clone();
			let counter = counter.clone();
			thread::spawn(move || {
				// Warmup
				let now = Instant::now();
				for _ in 0..NUMBER_OF_ITERATIONS {
					let mut guard = counter.lock().unwrap();
					*guard += 1;
				}
				let _ = now.elapsed();
				barrier.wait();

				let now = Instant::now();
				for _ in 0..NUMBER_OF_ITERATIONS {
					let mut guard = counter.lock().unwrap();
					*guard += 1;
				}
				now.elapsed()
			})
		})
		.collect::<Vec<_>>();

	let durations = handles
		.into_iter()
		.map(|handle| handle.join().unwrap())
		.collect::<Vec<_>>();

	assert_eq!(
		*counter.lock().unwrap(),
		2 * NUMBER_OF_ITERATIONS * available_parallelism
	);

	let print_duration = |duration| {
		let time_per_iteration =
			duration / u32::try_from(NUMBER_OF_ITERATIONS * available_parallelism).unwrap();
		println!("Time to solve: {duration:?}");
		println!("Time per iteration: {time_per_iteration:?}");
	};

	for (i, duration) in durations.iter().copied().enumerate() {
		println!("Thread {i}");
		print_duration(duration);
	}

	let average =
		durations.iter().sum::<Duration>() / u32::try_from(available_parallelism).unwrap();
	println!("Average");
	print_duration(average);
}
