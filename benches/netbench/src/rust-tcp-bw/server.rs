use std::io::{ErrorKind, Read};
use std::net::TcpStream;
use std::time::Instant;

use clap::Parser;
#[cfg(target_os = "hermit")]
use hermit as _;
use hermit_bench_output::log_benchmark_data;
use rust_tcp_io_perf::config::Config;
use rust_tcp_io_perf::print_utils::BoxplotValues;
use rust_tcp_io_perf::{connection, threading};

fn receive_rounds(
	stream: &mut TcpStream,
	rounds: usize,
	bytes: usize,
	progress_print: bool,
) -> Vec<f64> {
	let mut buf = vec![0; bytes];
	let mut durations = Vec::with_capacity(rounds);

	let progress_prints = [
		1,
		rounds / 10,
		rounds / 10 * 2,
		rounds / 10 * 3,
		rounds / 10 * 4,
		rounds / 10 * 5,
		rounds / 10 * 6,
		rounds / 10 * 7,
		rounds / 10 * 8,
		rounds / 10 * 9,
	];
	for i in 0..rounds {
		if progress_print && progress_prints.contains(&i) {
			println!("round {i}/{}", rounds)
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
	durations
}

fn main() {
	let args = Config::parse();

	println!(
		"starting server with {} bytes, {} warmup rounds and {} rounds",
		args.n_bytes, args.warmup, args.n_rounds
	);
	let mut stream = connection::server_listen_and_get_first_connection(&args.port.to_string());
	connection::setup(&args, &stream);
	threading::setup(&args);

	let _ = receive_rounds(&mut stream, args.warmup, args.n_bytes, false);
	let durations = receive_rounds(&mut stream, args.n_rounds, args.n_bytes, true);

	let statistics = BoxplotValues::<f64>::from(durations.as_slice());
	log_benchmark_data("TCP server", "Mbit/s", statistics.mean);

	println!("{statistics:#.2?}");
	println!(
		"{} outliers ({:.1}%)",
		statistics.nr_outliers,
		100.0 * statistics.nr_outliers as f64 / durations.len() as f64
	);

	connection::close_connection(&stream);
}
