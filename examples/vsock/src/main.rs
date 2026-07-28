//! A VSOCK echo application.
//!
//! This application echos all incoming data via VSOCK while printing the messages to stdout.
//!
//! When run with default features, it is run as a server. You can connect with:
//!
//! ```bash
//! socat - VSOCK-CONNECT:3:9975
//! ```
//!
//! When run with the `client` feature, it is run as a client. You can create a corresponding server with:
//!
//! ```bash
//! socat - VSOCK-LISTEN:9975
//! ```

use std::io::{self, Read, Write};

#[cfg(target_os = "hermit")]
use hermit as _;

use crate::vsock::VsockStream;

mod vsock;

pub const DEFAULT_BUF_SIZE: usize = 8 * 1024;

fn main() {
	let port = 9975;

	if cfg!(feature = "client") {
		use std::thread;
		use std::time::Duration;

		thread::sleep(Duration::from_secs(1));

		let addr = vsock::VsockAddr::new(2, port);
		let stream = VsockStream::connect(addr).expect("connection failed");

		echo(stream);
		return;
	}

	let listener = vsock::VsockListener::bind(port).unwrap();

	loop {
		let (stream, _addr) = listener.accept().unwrap();
		std::thread::spawn(|| echo(stream));
	}
}

fn echo(mut stream: VsockStream) {
	let mut buf = vec![0; DEFAULT_BUF_SIZE];

	eprintln!("Echoing on new connection...");

	loop {
		let n = stream.read(&mut buf).unwrap();
		if n == 0 {
			break;
		}

		let buf = &buf[..n];

		io::stdout().write_all(buf).unwrap();

		stream.write_all(buf).unwrap();
	}
}
