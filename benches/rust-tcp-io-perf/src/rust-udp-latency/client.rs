use std::net::UdpSocket;
use std::time::Instant;

use clap::Parser;
#[cfg(target_os = "hermit")]
use hermit as _;
use rust_tcp_io_perf::config::Config;

fn main() {
	let args = Config::parse();

	println!("Connecting to the server {}...", args.address);
	let n_rounds = args.n_rounds;
	let n_bytes = args.n_bytes;

	// Create buffers to read/write
	let wbuf: Vec<u8> = vec![0; n_bytes];
	let mut rbuf: Vec<u8> = vec![0; n_bytes];

	let progress_tracking_percentage = (n_rounds * 2) / 100;

	let socket = UdpSocket::bind("0.0.0.0:9975").expect("Couldn't bind to address");

	let mut hist = hdrhist::HDRHist::new();

	println!("Ready to send...");

	for i in 0..n_rounds {
		let start = Instant::now();

		socket
			.send_to(&wbuf, args.address_and_port())
			.expect("Couldn't send data");
		socket.recv(&mut rbuf).expect("Couldn't receive data");

		let duration = Instant::now().duration_since(start);

		hist.add_value(duration.as_secs() * 1_000_000_000u64 + duration.subsec_nanos() as u64);

		if i % progress_tracking_percentage == 0 {
			// Track progress on screen
			println!("{}% completed", i / progress_tracking_percentage);
		}
	}

	#[cfg(not(target_os = "hermit"))]
	hermit_bench_output::log_benchmark_data(
		"95th percentile UDP Server Latency",
		"ns",
		get_percentiles(hist.summary(), 0.95),
	);
	#[cfg(not(target_os = "hermit"))]
	hermit_bench_output::log_benchmark_data(
		"Max UDP Server Latency",
		"ns",
		get_percentiles(hist.summary(), 1.0),
	);

	#[cfg(target_os = "hermit")]
	hermit_bench_output::log_benchmark_data(
		"95th percentile UDP Client Latency",
		"ns",
		get_percentiles(hist.summary(), 0.95),
	);
	#[cfg(target_os = "hermit")]
	hermit_bench_output::log_benchmark_data(
		"Max UDP Client Latency",
		"ns",
		get_percentiles(hist.summary(), 1.0),
	);
}

fn get_percentiles(summary: impl Iterator<Item = (f64, u64, u64)>, percentile: f64) -> f64 {
	let mut res = 0.0;

	for (quantile, lower, upper) in summary {
		if quantile == percentile {
			res = (lower as f64 + upper as f64) / 2.0; // average of lower and upper bound
		}
	}

	// Return the 95th percentile and max value
	res
}
