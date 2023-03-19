#![allow(unused_imports)]

#[cfg(target_os = "hermit")]
use hermit_sys as _;

use clap::Parser;
use rust_tcp_io_perf::config::Config;
use rust_tcp_io_perf::connection;
use rust_tcp_io_perf::print_utils;
use rust_tcp_io_perf::threading;
use std::time::Instant;
use std::{thread, time};

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
				connection::setup(&args, &mut stream);
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
				print_utils::print_summary(hist);
			}
			Err(error) => {
				println!("Couldn't connect to server, retrying... Error {error}");
				thread::sleep(time::Duration::from_secs(1));
			}
		}
	}
}
