use std::hint::black_box;
use std::time::{Duration, Instant};

#[cfg(target_os = "hermit")]
use hermit as _;
use hermit_abi::getpid;

fn getpid_bench(count: usize) -> Duration {
	let start = Instant::now();
	for _ in 0..count {
		let _pid = black_box(unsafe { getpid() });
	}
	start.elapsed()
}

fn main() {
	println!("Determine overhead of getpid...");

	// warmup cache
	let _ = getpid_bench(2);

	let count = 10_0000_0000;
	let result = black_box(getpid_bench(count));

	println!("Number of iterations: {count}");
	println!("Total time: {result:?}");
	println!("Time per getpid: {:?}", result / count.try_into().unwrap());
}
