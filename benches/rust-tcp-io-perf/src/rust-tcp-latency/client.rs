use std::time::Instant;
use std::{thread, time};

use clap::Parser;
#[cfg(target_os = "hermit")]
use hermit as _;
use rust_tcp_io_perf::config::Config;
use rust_tcp_io_perf::{connection, threading};

fn main() {
	let args = Config::parse();

	println!("Connecting to the server {}...", args.address);
	let n_rounds = args.n_rounds;
	let n_bytes = args.n_bytes;

	// Create buffers to read/write
	let wbuf: Vec<u8> = vec![0; n_bytes];
	let mut rbuf: Vec<u8> = vec![0; n_bytes];

	let progress_tracking_percentage = (n_rounds * 2) / 100;

	let mut connected = false;

	while !connected {
		match connection::client_connect(args.address_and_port()) {
			Ok(mut stream) => {
				connection::setup(&args, &stream);
				threading::setup(&args);
				connected = true;
				let mut hist = hdrhist::HDRHist::new();

				println!("Connection established! Ready to send...");

				// To avoid TCP slowstart we do double iterations and measure only the second half
				for i in 0..(n_rounds * 2) {
					let start = Instant::now();

					connection::send_message(n_bytes, &mut stream, &wbuf);
					connection::receive_message(n_bytes, &mut stream, &mut rbuf);

					let duration = Instant::now().duration_since(start);
					if i >= n_rounds {
						hist.add_value(
							duration.as_secs() * 1_000_000_000u64 + duration.subsec_nanos() as u64,
						);
					}

					if i % progress_tracking_percentage == 0 {
						// Track progress on screen
						println!("{}% completed", i / progress_tracking_percentage);
					}
				}
				connection::close_connection(&stream);

				#[cfg(not(target_os = "hermit"))]
				hermit_bench_output::log_benchmark_data(
					"95th percentile TCP Server Latency",
					"ns",
					get_percentiles(hist.summary(), 0.95),
				);
				#[cfg(not(target_os = "hermit"))]
				hermit_bench_output::log_benchmark_data(
					"Max TCP Server Latency",
					"ns",
					get_percentiles(hist.summary(), 1.0),
				);

				#[cfg(target_os = "hermit")]
				hermit_bench_output::log_benchmark_data(
					"95th percentile TCP Client Latency",
					"ns",
					get_percentiles(hist.summary(), 0.95),
				);
				#[cfg(target_os = "hermit")]
				hermit_bench_output::log_benchmark_data(
					"Max TCP Client Latency",
					"ns",
					get_percentiles(hist.summary(), 1.0),
				);
			}
			Err(error) => {
				println!("Couldn't connect to server, retrying... Error {error}");
				thread::sleep(time::Duration::from_secs(1));
			}
		}
	}
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
