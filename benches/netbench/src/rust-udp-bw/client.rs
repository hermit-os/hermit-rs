#![allow(unused_imports)]

use std::fmt::format;
use std::io::{self, Write};
use std::net::UdpSocket;
use std::{thread, time};

use clap::Parser;
#[cfg(target_os = "hermit")]
use hermit as _;
use rust_tcp_io_perf::config::Config;
use rust_tcp_io_perf::connection;

fn main() {
	let args = Config::parse();

	println!("Connecting to the server {}:{}...", args.address, args.port);
	let n_rounds = args.n_rounds;
	let n_bytes = args.n_bytes;

	println!("Binding to address 0.0.0.0:9975...");
	if let Ok(socket) = UdpSocket::bind("0.0.0.0:9975") {
		println!("Socket open! Ready to send...");

		// Create buffers to read/write
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
