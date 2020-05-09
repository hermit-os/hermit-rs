#![allow(unused_imports)]

extern crate bytes;
#[cfg(target_os = "hermit")]
extern crate hermit_sys;
extern crate rust_tcp_io_perf;

use rust_tcp_io_perf::config;
use rust_tcp_io_perf::connection;
use rust_tcp_io_perf::print_utils;
use std::io::Read;
use std::time::Instant;

fn main() {
	let args = config::parse_config();
	let n_bytes = args.n_bytes;
	let tot_n_bytes = (n_bytes * args.n_rounds) as u64;

	let mut buf = vec![0; n_bytes];
	let mut tot_bytes: u64 = 0;

	let mut stream = connection::server_listen_and_get_first_connection(&args.port);

	let start = Instant::now();
	while tot_bytes <= tot_n_bytes {
		let recv = stream.read(&mut buf).unwrap();
		if recv > 0 {
			tot_bytes += recv as u64;
		} else {
			break;
		}
	}
	let end = Instant::now();
	let duration = end.duration_since(start);

	println!("Sent in total {} KBytes", tot_bytes / 1024);
	println!(
		"Available approximated bandwidth: {} Mbit/s",
		(tot_bytes as f64 * 8.0f64) / (1024.0f64 * 1024.0f64 * duration.as_secs_f64())
	);
}
