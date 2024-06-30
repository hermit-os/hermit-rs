use std::hint::black_box;
use std::time::Instant;

#[cfg(target_os = "hermit")]
use hermit as _;

// Number of iteration to stress the benchmark
const N: u64 = 1000000;

// Calculating fibonacci numbers
fn fibonacci(n: u64) -> u64 {
	let mut fib: u64 = 1;
	let mut fib1: u64 = 1;
	let mut fib2: u64 = 1;

	for _ in 3..=n {
		fib = fib1 + fib2;
		fib1 = fib2;
		fib2 = fib;
	}

	fib
}

pub fn main() {
	println!("Call function fibonacci");
	let result = fibonacci(30);
	println!("fibonacci(30) = {}", result);
	assert!(
		result == 832040,
		"Error in the calculation of fibonacci(30) "
	);

	let now = Instant::now();
	for _ in 0..N {
		black_box(fibonacci(black_box(30)));
	}
	let elapsed = now.elapsed();
	println!(
		"Time to call {} times native_fibonacci(30): {} s",
		N,
		elapsed.as_secs_f32()
	);
}
