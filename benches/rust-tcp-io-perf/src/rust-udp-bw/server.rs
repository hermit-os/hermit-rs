use std::net::UdpSocket;
use std::time::Instant;

use clap::Parser;
#[cfg(target_os = "hermit")]
use hermit as _;
use hermit_bench_output::log_benchmark_data;
use rust_tcp_io_perf::config::Config;

fn main() {
	let args = Config::parse();
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
			// Start the timer after the first message is received
			start = Instant::now();
		}

		if amt != n_bytes {
			println!("In Round {i}: Received {amt} bytes, expected {n_bytes}");
		}

		tot_bytes += amt * 2;
	}

	let end = Instant::now();
	let duration = end.duration_since(start);

	#[cfg(target_os = "hermit")]
	log_benchmark_data(
		"UDP server",
		"Mbit/s",
		(tot_bytes as f64 * 8.0f64) / (1024.0f64 * 1024.0f64 * duration.as_secs_f64()),
	);

	#[cfg(not(target_os = "hermit"))]
	log_benchmark_data(
		"UDP client",
		"Mbit/s",
		(tot_bytes as f64 * 8.0f64) / (1024.0f64 * 1024.0f64 * duration.as_secs_f64()),
	);
}
