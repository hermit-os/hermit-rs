//! This example requires setting HERMIT_IP=127.0.0.1

use std::io::{self, Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::thread;

#[cfg(target_os = "hermit")]
use hermit as _;

const TO_SEND: &[u8] = b"hello loopback";

fn handle_client(mut stream: TcpStream) -> io::Result<()> {
	let mut buf = [0u8; TO_SEND.len()];
	stream.read_exact(&mut buf)?;
	assert_eq!(&buf, TO_SEND);
	stream.write_all(TO_SEND)
}

fn main() -> io::Result<()> {
	let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, 9975));

	let t = thread::spawn(move || {
		let mut client = TcpStream::connect(addr)?;
		eprintln!("Client successfully connected");
		client.write_all(TO_SEND)?;
		let mut buf = [0u8; TO_SEND.len()];
		client.read_exact(&mut buf)?;
		assert_eq!(&buf, TO_SEND);
		Ok(())
	});

	let listener = TcpListener::bind(addr)?;
	eprintln!("Listening on {addr}");
	let (socket, socket_addr) = listener.accept()?;
	eprintln!("Accepted connection from {socket_addr}");
	handle_client(socket)?;

	t.join().unwrap()
}
