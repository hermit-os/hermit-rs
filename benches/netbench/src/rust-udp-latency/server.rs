#![allow(unused_imports)]

use std::net::UdpSocket;

use clap::Parser;
#[cfg(target_os = "hermit")]
use hermit as _;
use rust_tcp_io_perf::config::Config;
use rust_tcp_io_perf::{connection, threading};

fn main() {
	let args = Config::parse();
	let n_bytes = args.n_bytes;
	let n_rounds = args.n_rounds;
	let mut buf = vec![0; n_bytes];

	// Bind the UDP socket to the specified port
	let socket =
		UdpSocket::bind(format!("0.0.0.0:{}", args.port)).expect("Couldn't bind to address");
	println!("Server listening on port {}", args.port);

	// No need for connection setup for UDP, just start receiving and sending messages
	for _i in 0..n_rounds {
		let (amt, src) = socket.recv_from(&mut buf).expect("Didn't receive data");
		socket
			.send_to(&buf[..amt], src)
			.expect("Couldn't send data");
	}

	println!("Done exchanging stuff");
}
