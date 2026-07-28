use std::io::{self, Read, Write};

#[cfg(target_os = "hermit")]
use hermit as _;

use crate::vsock::VsockStream;

mod vsock;

pub const DEFAULT_BUF_SIZE: usize = 8 * 1024;

// demo program to test the vsock interface
//
// The program is used to demonstrate issue hermit-os/kernel#880
// Use `socat - VSOCK-CONNECT:3:9975`
// to communicate with the unikernel.
#[cfg(not(feature = "client"))]
fn main() {
	let listener = vsock::VsockListener::bind(9975).unwrap();

	loop {
		let (stream, _addr) = listener.accept().unwrap();
		std::thread::spawn(|| echo(stream));
	}
}

// demo program to connect with a vsock server
//
// The program is used to demonstrate issue hermit-os/kernel#880
// Use `socat - SOCKET-LISTEN:9975` to communicate with the unikernel.
#[cfg(feature = "client")]
fn main() {
	use std::thread;
	use std::time::Duration;

	thread::sleep(Duration::from_secs(1));

	let addr = vsock::VsockAddr::new(2, 9975);
	let stream = VsockStream::connect(addr).expect("connection failed");

	echo(stream);
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
