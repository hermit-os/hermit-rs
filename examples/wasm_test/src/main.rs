use std::fs;
use std::fs::File;
use std::hint::black_box;
use std::io::{Read, Write};
use std::time::Instant;

#[cfg(target_os = "hermit")]
use hermit as _;

// Number of iteration to stress the benchmark
const N: u64 = 1000000;
// Size of the temporary file
const FILE_SIZE: u64 = 1024 * 1024 * 100; // = 100 MB
										  // Path for temporary file
const FILE_PATH: &str = "/tmp/large_file.bin";
// Number of iterations for I/O benchmarking
const M: u64 = 100;

// Creating 100 MB file
pub fn create_large_file() {
	let mut file = File::create(FILE_PATH).expect("Could not create file");
	let buffer = vec![0u8; FILE_SIZE as usize];
	file.write_all(&buffer).expect("Could not write to file");
}

// Reading file
pub fn read_large_file() {
	let mut file = File::open(FILE_PATH).expect("Could not open file");
	let mut buffer = Vec::new();
	file.read_to_end(&mut buffer).expect("Could not read file");
}

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

	// Cache warmup
	for _i in 0..10 {
		black_box(fibonacci(black_box(30)));
	}

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

	// Cache warmup
	for _i in 0..10 {
		create_large_file();
		fs::remove_file(FILE_PATH).expect("Could not delete file");
	}

	let start_time = Instant::now();
	for _i in 0..M {
		create_large_file();
		fs::remove_file(FILE_PATH).expect("Could not delete file");
	}
	let elapsed_time = start_time.elapsed();
	println!("Total Create File Time: {:?}", elapsed_time);
}
