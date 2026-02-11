use std::io::Read;
use std::time::Instant;

use clap::Parser;
#[cfg(target_os = "hermit")]
use hermit as _;
use hermit_bench_output::log_benchmark_data;
use rust_tcp_io_perf::config::Config;
use rust_tcp_io_perf::connection;

fn main() {
	let args = Config::parse();
	let tot_bytes = args.n_rounds * args.n_bytes;

	let mut buf = vec![0; args.n_bytes];

	let mut stream = connection::server_listen_and_get_first_connection(&args.port.to_string());
	connection::setup(&args, &stream);

	let start = Instant::now();
	for i in 0..args.n_rounds {
		print!("round {i}: ");
		let round_start = Instant::now();
		stream.read_exact(&mut buf).unwrap();
		let round_end = Instant::now();
		let duration = round_end.duration_since(round_start);
		let mbits = buf.len() as f64 * 8.0f64 / (1024.0f64 * 1024.0f64 * duration.as_secs_f64());
		println!("{mbits} Mbit/s");
	}
	let end = Instant::now();
	let duration = end.duration_since(start);

	log_benchmark_data(
		"TCP server",
		"Mbit/s",
		(tot_bytes as f64 * 8.0f64) / (1024.0f64 * 1024.0f64 * duration.as_secs_f64()),
	);

	connection::close_connection(&stream);
}
