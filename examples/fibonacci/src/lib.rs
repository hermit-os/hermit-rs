// Test library to test the wasmtime demo
use core::hint::black_box;

extern "C" {
	fn now() -> f64;
}

// Just a dummy function to measure the overhead
#[no_mangle]
pub extern "C" fn foo() {}

// Calculating fibonacci numbers
#[no_mangle]
pub extern "C" fn fibonacci(n: u64) -> u64 {
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

#[no_mangle]
pub extern "C" fn bench(iterations: u64, number: u64) -> f64 {
	let start = unsafe { now() };
	for _ in 0..iterations {
		black_box(fibonacci(black_box(number)));
	}
	let end = unsafe { now() };

	end - start
}
