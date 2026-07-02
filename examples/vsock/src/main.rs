use std::io::{Read, Write};

#[cfg(target_os = "hermit")]
use hermit as _;

mod vsock;

/// CID of the host, which the guest connects to in the echo phase.
const HOST_CID: u32 = 2;
/// CID assigned to this guest, which it binds its listener to in the ping/pong
/// phase.
const GUEST_CID: u32 = 3;
/// Port the guest connects to on the host for the echo phase.
const ECHO_PORT: u32 = 9975;
/// Port the guest listens on for the ping/pong phase.
const PING_PONG_PORT: u32 = 9976;
/// Number of sequential ping/pong connections the guest accepts.
///
/// Accepting more than one connection on the same listener is a regression test
/// for hermit-os/kernel#2433 (a vsock listener could not accept a second
/// connection).
const PING_PONG_CONNECTIONS: usize = 2;

// demo program to test the vsock interface
//
// The program exercises both directions a hermit app can use vsock, and the
// host side is driven by xtask (see xtask/src/ci/qemu.rs test_vsock):
//   1. it connects out to the host and echoes back what the host sends (#880);
//   2. it then listens and accepts PING_PONG_CONNECTIONS connections, replying
//      "pong" to each "ping" (#2433 — accepting more than one connection).
fn main() {
	// Example 1: connect to the host and echo everything back until it closes.
	{
		// Give the host side time to start listening before we connect.
		std::thread::sleep(std::time::Duration::from_secs(1));

		let addr = vsock::VsockAddr::new(HOST_CID, ECHO_PORT);
		let mut socket = vsock::VsockStream::connect(addr).expect("connect failed");
		let mut buf = [0u8; 1000];

		println!("connected to host; echoing back received data");

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

	// Example 2: listen and reply "pong" to each "ping".
    println!("Binding to vsock as guest and listening for connections");

    let listener = vsock::VsockListener::bind(GUEST_CID, PING_PONG_PORT).expect("bind failed");
	for i in 1..=PING_PONG_CONNECTIONS {
		println!("waiting for ping/pong connection {i}/{PING_PONG_CONNECTIONS}");
		let (mut socket, _addr) = listener.accept().expect("accept failed");

		let mut buf = [0u8; 64];
		let n = socket.read(&mut buf).expect("read failed");
		let msg = std::str::from_utf8(&buf[..n]).unwrap_or("<invalid>");
		assert_eq!(msg, "ping", "connection {i}: unexpected message");

		socket.write_all(b"pong").expect("write failed");
		println!("sent pong for connection {i}");
		// socket drops here, closing the connection
	}

	println!("vsock_test: PASSED");
}
