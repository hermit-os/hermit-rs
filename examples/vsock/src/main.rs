#[allow(unused_imports)]
use std::io::{Read, Write};

#[cfg(target_os = "hermit")]
use hermit as _;

mod vsock;

// demo program to test the vsock interface
//
// The program is used to demonstrate issue hermit-os/kernel#880
// Use `socat - VSOCK-CONNECT:3:9975`
// to communicate with the unikernel.
#[cfg(not(feature = "client"))]
fn main() {
	let listener = vsock::VsockListener::bind(9975).unwrap();
	let (mut socket, _addr) = listener.accept().unwrap();
	let mut buf = [0u8; 1000];

	println!("Try to read from vsock stream...");

	loop {
		match socket.read(&mut buf) {
			Err(e) => {
				println!("read err {e:?}");
				break;
			}
			Ok(received) => {
				print!("{}", std::str::from_utf8(&buf[..received]).unwrap());
				if received == 0 {
					break;
				}

				socket.write_all(&buf[..received]).unwrap();
			}
		}
	}
}

// demo program to connect with a vsock server
//
// The program is used to demonstrate issue hermit-os/kernel#880
// Use `socat - SOCKET-LISTEN:9975` to communicate with the unikernel.
#[cfg(feature = "client")]
fn main() {
	let addr = vsock::VsockAddr::new(2, 9975);
	let mut socket = vsock::VsockStream::connect(addr).expect("connection failed");
	let mut buf = [0u8; 1000];

	loop {
		match socket.read(&mut buf) {
			Err(e) => {
				println!("read err {e:?}");
				break;
			}
			Ok(received) => {
				let msg = std::str::from_utf8(&buf[..received]).unwrap();
				print!("{}", msg);

				if msg.trim() == "exit" {
					break;
				}
			}
		}
	}
}
