#[cfg(target_os = "hermit")]
use hermit as _;

use std::io::Read;
use std::net::TcpListener;
use std::os::hermit::io::AsRawFd;

// demo program to test the tcp interface
//
// The program is used to demonstrate issue hermit-os/kernel#880
// Use `socat - TCP:localhost:9975` to communicate with the
// unikernel.

fn main() {
	let listener = TcpListener::bind("0.0.0.0:9975").unwrap();
	let (mut socket, _) = listener.accept().unwrap();
	let mut buf = [0u8; 1000];
	let mut fds: [hermit_abi::pollfd; 1] = [Default::default(); 1];
	fds[0].fd = socket.as_raw_fd();
	fds[0].events = hermit_abi::POLLIN;
	loop {
		println!("about to read");
		let _ret = unsafe { hermit_abi::poll(fds.as_mut_ptr(), 1, -1) };
		println!("revents {:?}", fds[0]);
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
