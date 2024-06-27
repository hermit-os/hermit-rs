use std::thread;
use std::time::{Duration, Instant};

#[cfg(target_os = "hermit")]
use hermit as _;
use hermit_bench_output::log_benchmark_data;

fn calc_pi_multi_threaded(n: u64, threads: u64) -> f64 {
	let mut handles = vec![];
	let mut pi = 0.0;
	let step = 1.0 / n as f64;
	for i in 0..threads {
		let start = i * n / threads;
		let end = (i + 1) * n / threads;
		let handle = thread::spawn(move || {
			let mut pi = 0.0;
			for i in start..end {
				let x = (i as f64 + 0.5) * step;
				pi += 4.0 / (1.0 + x * x);
			}
			pi
		});
		handles.push(handle);
	}
	for handle in handles {
		pi += handle.join().unwrap();
	}
	pi *= step;
	pi
}

fn main() {
	let n = 100000000;
	//let n = 1000000000;
	let mut times: Vec<Duration> = Vec::new();

	for i in 1..=8 {
		let now = Instant::now();
		calc_pi_multi_threaded(n, i);
		let elapsed = now.elapsed();
		times.push(elapsed);
	}

	// Calc speedup
	let speedup1_2 = times[0].as_secs_f64() / times[1].as_secs_f64();
	let speedup1_4 = times[0].as_secs_f64() / times[2].as_secs_f64();
	let speedup1_8 = times[0].as_secs_f64() / times[7].as_secs_f64();

	// Calc efficiency
	let efficiency1_2 = speedup1_2 / 2.0;
	let efficiency1_4 = speedup1_4 / 4.0;
	let efficiency1_8 = speedup1_8 / 8.0;

	log_benchmark_data("2 Threads", "%", efficiency1_2 * 100.0);
	log_benchmark_data("4 Threads", "%", efficiency1_4 * 100.0);
	log_benchmark_data("8 Threads", "%", efficiency1_8 * 100.0);
}
