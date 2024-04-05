use std::io::Read;
use std::net::TcpListener;

#[cfg(target_os = "hermit")]
use hermit as _;

// demo program to test the tcp interface
//
// The program is used to demonstrate issue hermit-os/kernel#880
// Use `socat - TCP:localhost:9975` to communicate with the
// unikernel.

fn main() {
	let listener = TcpListener::bind("0.0.0.0:9975").unwrap();
	let (mut socket, _) = listener.accept().unwrap();
	let mut buf = [0u8; 1000];
	loop {
		println!("about to read");
		match socket.read(&mut buf) {
			Err(e) => {
				println!("read err {e:?}");
				break;
			}
			Ok(received) => {
				print!("read {}", std::str::from_utf8(&buf[..received]).unwrap());
				if received == 0 {
					break;
				}
			}
		}
	}
}
