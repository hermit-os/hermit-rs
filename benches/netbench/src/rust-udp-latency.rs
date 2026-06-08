use std::net::UdpSocket;
use std::time::Instant;

use clap::{Parser, Subcommand};
#[cfg(target_os = "hermit")]
use hermit as _;
use hermit_bench_output::log_benchmark_data;
use rust_tcp_io_perf::config::Config;

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
			println!("{}% completed", i / progress_tracking_percentage);
		}
	}

	#[cfg(not(target_os = "hermit"))]
	{
		log_benchmark_data(
			"95th percentile UDP Server Latency",
			"ns",
			get_percentiles(hist.summary(), 0.95),
		);
		log_benchmark_data(
			"Max UDP Server Latency",
			"ns",
			get_percentiles(hist.summary(), 1.0),
		);
	}

	#[cfg(target_os = "hermit")]
	{
		log_benchmark_data(
			"95th percentile UDP Client Latency",
			"ns",
			get_percentiles(hist.summary(), 0.95),
		);
		log_benchmark_data(
			"Max UDP Client Latency",
			"ns",
			get_percentiles(hist.summary(), 1.0),
		);
	}
}

fn run_server(args: Config) {
	let n_bytes = args.n_bytes;
	let n_rounds = args.n_rounds;
	let mut buf = vec![0; n_bytes];

	let socket =
		UdpSocket::bind(format!("0.0.0.0:{}", args.port)).expect("Couldn't bind to address");
	println!("Server listening on port {}", args.port);

	for _i in 0..n_rounds {
		let (amt, src) = socket.recv_from(&mut buf).expect("Didn't receive data");
		socket
			.send_to(&buf[..amt], src)
			.expect("Couldn't send data");
	}

	println!("Done exchanging stuff");
}

fn main() {
	match Cli::parse().command {
		Command::Server(args) => run_server(args),
		Command::Client(args) => run_client(args),
	}
}
