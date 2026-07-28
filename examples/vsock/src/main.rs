use std::io::{Read, Write};

#[cfg(target_os = "hermit")]
use hermit as _;

use crate::vsock::VsockStream;

mod vsock;

// demo program to test the vsock interface
//
// The program is used to demonstrate issue hermit-os/kernel#880
// Use `socat - VSOCK-CONNECT:3:9975`
// to communicate with the unikernel.
#[cfg(not(feature = "client"))]
fn main() {
	let listener = vsock::VsockListener::bind(9975).unwrap();
	let (stream, _addr) = listener.accept().unwrap();

	echo(stream);
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
	let mut buf = [0u8; 1000];

	eprintln!("Echoing on new connection...");

	loop {
		match stream.read(&mut buf) {
			Err(e) => {
				println!("read err {e:?}");
				break;
			}
			Ok(received) => {
				if received == 0 {
					break;
				}
				let msg = std::str::from_utf8(&buf[..received]).unwrap();
				print!("{}", msg);
				stream.write_all(&buf[..received]).unwrap();
			}
		}
	}
}
