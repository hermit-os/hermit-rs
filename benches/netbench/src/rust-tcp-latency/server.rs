#![allow(unused_imports)]

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

	let mut stream = connection::server_listen_and_get_first_connection(&args.port.to_string());
	connection::setup(&args, &stream);
	threading::setup(&args);

	// Make sure n_rounds is the same between client and server
	for _i in 0..(n_rounds * 2) {
		connection::receive_message(n_bytes, &mut stream, &mut buf);
		connection::send_message(n_bytes, &mut stream, &buf);
	}

	println!("Done exchanging stuff")
}
