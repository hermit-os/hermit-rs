use std::time::Instant;
use std::{thread, time};

use clap::{Parser, Subcommand};
#[cfg(target_os = "hermit")]
use hermit as _;
use hermit_bench_output::log_benchmark_data;
use rust_tcp_io_perf::config::Config;
use rust_tcp_io_perf::print_utils::BoxplotValues;
use rust_tcp_io_perf::{connection, threading};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
	#[command(subcommand)]
	command: Command,
}

#[derive(Subcommand)]
enum Command {
	/// Run the latency server
	Server(Config),
	/// Run the latency client
	Client(Config),
}

fn get_percentiles(summary: impl Iterator<Item = (f64, u64, u64)>, percentile: f64) -> f64 {
	let mut res = 0.0;

	for (quantile, lower, upper) in summary {
		if quantile == percentile {
			res = (lower as f64 + upper as f64) / 2.0;
		}
	}

	res
}

fn run_client(args: Config) {
	println!("Connecting to the server {}...", args.address);
	let n_rounds = args.n_rounds;
	let n_bytes = args.n_bytes;

	let wbuf: Vec<u8> = vec![0; n_bytes];
	let mut rbuf: Vec<u8> = vec![0; n_bytes];

	let progress_tracking_percentage = n_rounds / 100;

	const MAX_RETRIES: i32 = 30;
	let mut retries = 0;
	let mut stream = loop {
		match connection::client_connect(args.address_and_port()) {
			Ok(stream) => break stream,
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

	for _ in 0..args.warmup {
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
			println!("{}% completed", i / progress_tracking_percentage);
		}
	}
	connection::close_connection(&stream);

	log_benchmark_data(
		"95th percentile TCP Client Latency",
		"ns",
		get_percentiles(hist.summary(), 0.95),
	);
	log_benchmark_data(
		"Max TCP Client Latency",
		"ns",
		get_percentiles(hist.summary(), 1.0),
	);

	let statistics = BoxplotValues::from(latencies.as_slice());
	println!("{statistics:#.2?}");
}

fn run_server(args: Config) {
	let n_bytes = args.n_bytes;
	let n_rounds = args.n_rounds;
	let mut buf = vec![0; n_bytes];

	let mut stream = connection::server_listen_and_get_first_connection(&args.port.to_string());
	connection::setup(&args, &stream);
	threading::setup(&args);

	for _i in 0..(n_rounds + args.warmup) {
		connection::receive_message(n_bytes, &mut stream, &mut buf);
		connection::send_message(n_bytes, &mut stream, &buf);
	}

	println!("Done exchanging stuff")
}

fn main() {
	match Cli::parse().command {
		Command::Server(args) => run_server(args),
		Command::Client(args) => run_client(args),
	}
}
