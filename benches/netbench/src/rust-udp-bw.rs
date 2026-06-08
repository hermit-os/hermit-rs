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
	/// Run the bandwidth server
	Server(Config),
	/// Run the bandwidth client
	Client(Config),
}

fn run_client(args: Config) {
	println!("Connecting to the server {}:{}...", args.address, args.port);
	let n_rounds = args.n_rounds;
	let n_bytes = args.n_bytes;

	println!("Binding to address 0.0.0.0:9975...");
	if let Ok(socket) = UdpSocket::bind("0.0.0.0:9975") {
		println!("Socket open! Ready to send...");

		let wbuf: Vec<u8> = vec![0; n_bytes];
		let mut rbuf: Vec<u8> = vec![0; n_bytes];

		for _i in 0..n_rounds {
			socket
				.send_to(&wbuf, args.address_and_port())
				.expect("Couldn't send data");
			socket.recv(&mut rbuf).expect("Couldn't receive data");
		}

		println!("Sent everything!");
	} else {
		println!("Couldn't connect to server...");
	}
}

fn run_server(args: Config) {
	let n_bytes = args.n_bytes;
	let mut tot_bytes = 0;

	let mut buf = vec![0; n_bytes];

	let socket = UdpSocket::bind(format!("0.0.0.0:{}", args.port)).expect("Failed to bind socket");

	println!("Socket (0.0.0.0:{}) open! Ready to receive...", args.port);

	let mut start = Instant::now();

	for i in 0..args.n_rounds {
		let (amt, src) = socket.recv_from(&mut buf).expect("Didn't receive data");
		socket
			.send_to(&buf[..amt], src)
			.expect("Couldn't send data");

		if i == 0 {
			start = Instant::now();
		}

		if amt != n_bytes {
			println!("In Round {i}: Received {amt} bytes, expected {n_bytes}");
		}

		tot_bytes += amt * 2;
	}

	let end = Instant::now();
	let duration = end.duration_since(start);
	let mbits = (tot_bytes as f64 * 8.0f64) / (1024.0f64 * 1024.0f64 * duration.as_secs_f64());

	#[cfg(target_os = "hermit")]
	log_benchmark_data("UDP server", "Mbit/s", mbits);

	#[cfg(not(target_os = "hermit"))]
	log_benchmark_data("UDP client", "Mbit/s", mbits);
}

fn main() {
	match Cli::parse().command {
		Command::Server(args) => run_server(args),
		Command::Client(args) => run_client(args),
	}
}
