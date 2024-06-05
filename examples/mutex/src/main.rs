#![feature(duration_millis_float)]

use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;

#[cfg(target_os = "hermit")]
use hermit as _;

const NUMBER_OF_ITERATIONS: usize = 1000;

fn mutex_stress_test(no_threads: usize) {
	let counter = Arc::new(Mutex::new(0));

	let now = Instant::now();
	let handlers = (0..no_threads)
		.map(|_| {
			let counter = counter.clone();
			thread::spawn(move || {
				for _ in 0..NUMBER_OF_ITERATIONS {
					let mut guard = counter.lock().unwrap();
					*guard += 1;
				}
			})
		})
		.collect::<Vec<_>>();

	for handler in handlers {
		handler.join().unwrap();
	}

	let elapsed = now.elapsed();
	println!("Time to solve {elapsed:?} with {} threads", no_threads);
	println!(
		"Time to increment counter: {}ms",
		elapsed.as_millis_f64() / (NUMBER_OF_ITERATIONS * no_threads) as f64
	);

	assert_eq!(*counter.lock().unwrap(), NUMBER_OF_ITERATIONS * no_threads);
}

fn main() {
	let available_parallelism: usize = thread::available_parallelism().unwrap().into();
	println!("available_parallelism = {available_parallelism}");

	for i in 1..=available_parallelism {
		mutex_stress_test(i);
	}
}
