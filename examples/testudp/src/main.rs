#[cfg(target_os = "hermit")]
use hermit as _;

use std::net::UdpSocket;

// demo program to test the udp interface
//
// Use `socat - UDP:localhost:8080` to communicate with the
// unikernel.

fn main() {
	let socket = UdpSocket::bind("0.0.0.0:8080").expect("couldn't bind to address");
	let mut buf = [0; 1000];

	loop {
		// Receives a single datagram message on the socket.
		// If `buf` is too small to hold, the message, it will be cut off.
		println!("about to recv");
		match socket.recv(&mut buf) {
			Ok(received) => print!(
				"received {}",
				std::str::from_utf8(&buf[..received]).unwrap()
			),
			Err(e) => {
				println!("recv function failed: {e:?}");
				break;
			}
		}
	}
}
