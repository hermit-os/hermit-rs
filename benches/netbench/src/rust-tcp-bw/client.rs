#![allow(unused_imports)]

extern crate bytes;
#[cfg(target_os = "hermit")]
extern crate hermit_sys;
extern crate rust_tcp_io_perf;

use rust_tcp_io_perf::config;
use rust_tcp_io_perf::connection;
use std::io::Write;

fn main() {
	let args = config::parse_config();

	println!("Connecting to the server {}...", args.address);
	let n_rounds = args.n_rounds;
	let n_bytes = args.n_bytes;

	if let Ok(mut stream) = connection::client_connect(args.address_and_port()) {
		connection::setup(&args, &mut stream);
		println!("Connection established! Ready to send...");

		// Create a buffer of 0s, size n_bytes, to be sent over multiple times
		let buf = vec![0; n_bytes];

		for _i in 0..n_rounds {
			match stream.write_all(&buf) {
				Ok(_) => {}
				Err(err) => panic!("crazy stuff happened while sending {}", err),
			}
		}
		stream.flush().expect("Unexpected behaviour");
		connection::close_connection(&stream);

		println!("Sent everything!");
	} else {
		println!("Couldn't connect to server...");
	}
}
