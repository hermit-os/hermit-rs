use std::io::{ErrorKind, Read};
use std::time::Instant;

use clap::Parser;
#[cfg(target_os = "hermit")]
use hermit as _;
use hermit_bench_output::log_benchmark_data;
use rust_tcp_io_perf::config::Config;
use rust_tcp_io_perf::connection;

fn main() {
	let args = Config::parse();

	let mut buf = vec![0; args.n_bytes];
	let mut durations = Vec::with_capacity(args.n_rounds);

	println!(
		"starting server with {} bytes and {} rounds",
		args.n_bytes, args.n_rounds
	);
	let mut stream = connection::server_listen_and_get_first_connection(&args.port.to_string());
	connection::setup(&args, &stream);

	let progress_prints = [
		1,
		args.n_rounds / 10,
		args.n_rounds / 10 * 2,
		args.n_rounds / 10 * 3,
		args.n_rounds / 10 * 4,
		args.n_rounds / 10 * 5,
		args.n_rounds / 10 * 6,
		args.n_rounds / 10 * 7,
		args.n_rounds / 10 * 8,
		args.n_rounds / 10 * 9,
	];

	for i in 0..args.n_rounds {
		if progress_prints.contains(&i) {
			println!("round {i}/{}", args.n_rounds)
		}
		let round_start = Instant::now();
		if let Err(e) = stream.read_exact(&mut buf) {
			if e.kind() == ErrorKind::UnexpectedEof {
				println!("Client ended transmission after {i} rounds");
				break;
			} else {
				panic!("Error in reading from stream: {}", e.kind());
			}
		}
		let round_end = Instant::now();
		let duration = round_end.duration_since(round_start);
		let mbits = buf.len() as f64 * 8.0f64 / (1024.0f64 * 1024.0f64 * duration.as_secs_f64());
		durations.push(mbits);
	}

	log_benchmark_data(
		"TCP server",
		"Mbit/s",
		durations.iter().sum::<f64>() / durations.len() as f64,
	);

	connection::close_connection(&stream);
}
