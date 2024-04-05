use std::net::UdpSocket;

#[cfg(target_os = "hermit")]
use hermit as _;

// demo program to test the udp interface
//
// Use `socat - UDP:localhost:9975` to communicate with the
// unikernel.

fn main() {
	let socket = UdpSocket::bind("0.0.0.0:9975").expect("couldn't bind to address");
	let mut buf = [0; 1000];

	loop {
		// Receives a single datagram message on the socket.
		// If `buf` is too small to hold, the message, it will be cut off.
		match socket.recv_from(&mut buf) {
			Ok((received, addr)) => {
				let msg = std::str::from_utf8(&buf[..received]).unwrap();

				// print msg without suffix `\n`
				match msg.strip_suffix('\n') {
					Some(striped_msg) => {
						println!("received \"{}\" from {}", striped_msg, addr);
					}
					_ => {
						println!("received \"{}\" from {}", msg, addr);
					}
				}

				// send message back
				socket
					.send_to(msg.as_bytes(), addr)
					.expect("Unable to send message back");

				if msg.starts_with("exit") {
					break;
				}
			}
			Err(e) => {
				println!("recv function failed: {e:?}");
				break;
			}
		}
	}
}
