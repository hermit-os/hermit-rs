use std::io::{self, Write};

#[cfg(feature = "rftrace")]
extern crate rftrace as _;
use clap::Parser;
#[cfg(target_os = "hermit")]
use hermit as _;
#[cfg(feature = "rftrace")]
use rftrace_frontend as rftrace;
use rust_tcp_io_perf::config::Config;
use rust_tcp_io_perf::connection;

fn main() {
	#[cfg(feature = "rftrace")]
	let events = rftrace::init(100000, false);

	let args = Config::parse();

	println!("Connecting to the server {}:{}...", args.address, args.port);

	if let Ok(mut stream) = connection::client_connect(args.address_and_port()) {
		connection::setup(&args, &stream);
		println!("Connection established! Ready to send...");

		// Create a buffer of 0s, size n_bytes, to be sent over multiple times
		let buf = vec![0; args.n_bytes];

		#[cfg(feature = "rftrace")]
		rftrace::enable();

		for _i in 0..args.n_rounds {
			let mut pos = 0;

			while pos < buf.len() {
				let bytes_written = match stream.write(&buf[pos..]) {
					Ok(len) => len,
					Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => 0,
					Err(e) => panic!("encountered IO error: {e}"),
				};
				pos += bytes_written;
			}
		}
		stream.flush().expect("Unexpected behaviour");

		#[cfg(feature = "rftrace")]
		rftrace::disable();

		connection::close_connection(&stream);

		println!("Sent everything!");

		#[cfg(feature = "rftrace")]
		rftrace::dump_full_uftrace(events, "/tracedir", "tcp-client-bw").unwrap();
	} else {
		println!("Couldn't connect to server...");
	}
}
