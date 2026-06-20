#[allow(unused_imports)]
use std::io::{Read, Write};

#[cfg(target_os = "hermit")]
use hermit as _;

mod vsock;

/// Number of sequential ping/pong connections the server accepts after the
/// initial echo connection.
///
/// Accepting further connections on the same listener is a regression test for
/// hermit-os/kernel#2433 (a vsock listener could not accept a second
/// connection).
#[cfg(not(feature = "client"))]
pub const PING_PONG_CONNECTIONS: usize = 2;

// demo program to test the vsock interface
//
// The program demonstrates issues hermit-os/kernel#880 and #2433 on a single
// listener. It first echoes back everything received on one connection (#880),
// then accepts PING_PONG_CONNECTIONS further connections, replying "pong" to
// each "ping" (#2433 — accepting more than one connection). The host-side
// client is driven by xtask (see xtask/src/ci/qemu.rs test_vsock).
//
// Use `socat - VSOCK-CONNECT:3:9975` to communicate with the unikernel.
#[cfg(not(feature = "client"))]
fn main() {
	let listener = vsock::VsockListener::bind(9975).unwrap();

	// First connection: echo everything back until the peer closes (#880).
	{
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

	// Further connections: reply "pong" to each "ping" (#2433).
	for i in 1..=PING_PONG_CONNECTIONS {
		println!("[server] waiting for ping/pong connection {i}/{PING_PONG_CONNECTIONS}");
		let (mut socket, _addr) = listener.accept().expect("accept failed");

		let mut buf = [0u8; 64];
		let n = socket.read(&mut buf).expect("read failed");
		let msg = std::str::from_utf8(&buf[..n]).unwrap_or("<invalid>");
		assert_eq!(msg, "ping", "connection {i}: unexpected message");

		socket.write_all(b"pong").expect("write failed");
		println!("[server] sent pong for connection {i}");
		// socket drops here, closing the connection
	}

	println!("vsock_test: PASSED");
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
				socket.write_all(&buf[..received]).unwrap();

				if msg.trim() == "exit" {
					break;
				}
			}
		}
	}
}
