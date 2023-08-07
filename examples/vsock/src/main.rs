use std::io::{Read, Write};

#[cfg(target_os = "hermit")]
use hermit as _;

use crate::vsock::VsockListener;

#[cfg(target_os = "hermit")]
mod vsock;

// demo program to test the vsock interface
//
// The program is used to demonstrate issue hermit-os/kernel#880
// Use `socat - SOCKET-CONNECT:40:0:x00x00xF7x26x00x00x03x00x00x00x00x00x00x00`
// to communicate with the unikernel.

fn main() {
	let listener = VsockListener::bind(9975).unwrap();
	let (mut socket, addr) = listener.accept().unwrap();
	let mut buf = [0u8; 1000];

	println!("Connected with {:?}", addr);
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
