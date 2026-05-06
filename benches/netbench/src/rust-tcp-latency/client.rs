use std::time::Instant;
use std::{thread, time};

use clap::Parser;
#[cfg(target_os = "hermit")]
use hermit as _;
use rust_tcp_io_perf::config::Config;
use rust_tcp_io_perf::print_utils::BoxplotValues;
use rust_tcp_io_perf::{connection, threading};

fn main() {
	let args = Config::parse();

	println!("Connecting to the server {}...", args.address);
	let n_rounds = args.n_rounds;
	let n_bytes = args.n_bytes;

	// Create buffers to read/write
	let wbuf: Vec<u8> = vec![0; n_bytes];
	let mut rbuf: Vec<u8> = vec![0; n_bytes];

	let progress_tracking_percentage = (n_rounds) / 100;

	const MAX_RETRIES: i32 = 30;
	let mut retries = 0;
	let mut stream =
		loop {
			match connection::client_connect(args.address_and_port()) {
				Ok(stream) => {
					break stream;
				}
				Err(error) => {
					retries += 1;
					println!("Couldn't connect to server, retrying ({retries}/{MAX_RETRIES})... ({error})");
					if retries >= MAX_RETRIES {
						panic!("Can't establish connection to server. Aborting after {MAX_RETRIES} attempts");
					}
					thread::sleep(time::Duration::from_secs(1));
				}
			}
		};

	connection::setup(&args, &stream);
	threading::setup(&args);
	let mut hist = hdrhist::HDRHist::new();
	let mut latencies = Vec::with_capacity(n_rounds);

	println!("Connection established! Ready to send...");

	for _ in 0..(args.warmup) {
		connection::send_message(n_bytes, &mut stream, &wbuf);
		connection::receive_message(n_bytes, &mut stream, &mut rbuf);
	}

	for i in 0..n_rounds {
		let start = Instant::now();

		connection::send_message(n_bytes, &mut stream, &wbuf);
		connection::receive_message(n_bytes, &mut stream, &mut rbuf);

		let duration = Instant::now().duration_since(start);
		let duration_u64 = duration.as_secs() * 1_000_000_000u64 + duration.subsec_nanos() as u64;
		hist.add_value(duration_u64);
		latencies.push(duration_u64);

		if i % progress_tracking_percentage == 0 {
			// Track progress on screen
			println!("{}% completed", i / progress_tracking_percentage);
		}
	}
	connection::close_connection(&stream);

	hermit_bench_output::log_benchmark_data(
		"95th percentile TCP Client Latency",
		"ns",
		get_percentiles(hist.summary(), 0.95),
	);
	hermit_bench_output::log_benchmark_data(
		"Max TCP Client Latency",
		"ns",
		get_percentiles(hist.summary(), 1.0),
	);

	let statistics = BoxplotValues::from(latencies.as_slice());
	println!("{statistics:#.2?}");
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
