#![allow(unused_imports)]

use std::io::Read;
use std::time::Instant;

use clap::Parser;
#[cfg(target_os = "hermit")]
use hermit as _;
use rust_tcp_io_perf::config::Config;
use rust_tcp_io_perf::{connection, print_utils};

fn main() {
	let args = Config::parse();
	let n_bytes = args.n_bytes;
	let tot_bytes = args.n_rounds * args.n_bytes;

	let mut buf = vec![0; n_bytes];

	let mut stream = connection::server_listen_and_get_first_connection(&args.port.to_string());
	connection::setup(&args, &stream);

	let start = Instant::now();
	for _i in 0..args.n_rounds {
		stream.read_exact(&mut buf).unwrap();
	}
	let end = Instant::now();
	let duration = end.duration_since(start);

	println!("Sent in total {} KBytes", tot_bytes / 1024);
	println!(
		"Available approximated bandwidth: {} Mbit/s",
		(tot_bytes as f64 * 8.0f64) / (1024.0f64 * 1024.0f64 * duration.as_secs_f64())
	);
}
