use std::io;
use std::io::ErrorKind::WouldBlock;
use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream, ToSocketAddrs, UdpSocket};

use crate::config::Config;

pub const UDP_CLIENT_BIND_ADDR: &str = "0.0.0.0:9975";

pub fn send_message(n_bytes: usize, stream: &mut TcpStream, wbuf: &[u8]) {
	let mut send = 0;
	while send < n_bytes {
		match stream.write(&wbuf[send..]) {
			Ok(n) => send += n,
			Err(err) => match err.kind() {
				WouldBlock => {}
				_ => panic!("Error occurred while writing: {err:?}"),
			},
		}
	}
}

pub fn receive_message(n_bytes: usize, stream: &mut TcpStream, rbuf: &mut [u8]) {
	let mut recv = 0;
	while recv < n_bytes {
		match stream.read(&mut rbuf[recv..]) {
			Ok(n) => {
				if n == 0 {
					panic!("Connection closed prematurely")
				} else {
					recv += n
				}
			}
			Err(err) => match err.kind() {
				WouldBlock => {}
				_ => panic!("Error occurred while reading: {err:?}"),
			},
		}
	}
}

pub fn setup(config: &Config, stream: &TcpStream) {
	if config.no_delay {
		stream
			.set_nodelay(true)
			.expect("Can't set no_delay to true");
	}
	if config.non_blocking {
		stream
			.set_nonblocking(true)
			.expect("Can't set channel to be non-blocking");
	}
}

pub fn client_connect<A: ToSocketAddrs>(addr: A) -> io::Result<TcpStream> {
	TcpStream::connect(addr)
}

pub fn close_connection(stream: &TcpStream) {
	stream
		.shutdown(Shutdown::Both)
		.expect("shutdown call failed");
}

pub fn server_listen_and_get_first_connection(port: &str) -> TcpStream {
	let listener = TcpListener::bind("0.0.0.0:".to_owned() + port).unwrap();
	println!("Server running, listening for connection on 0.0.0.0:{port}");
	let stream = listener.incoming().next().unwrap().unwrap();
	println!(
		"Connection established with {:?}!",
		stream.peer_addr().unwrap()
	);

	stream
}

pub fn bind_udp_client() -> io::Result<UdpSocket> {
	UdpSocket::bind(UDP_CLIENT_BIND_ADDR)
}

pub fn bind_udp_server(port: u16) -> io::Result<UdpSocket> {
	UdpSocket::bind(format!("0.0.0.0:{port}"))
}

pub fn udp_exchange(
	socket: &UdpSocket,
	wbuf: &[u8],
	rbuf: &mut [u8],
	dest: &str,
) -> io::Result<()> {
	socket.send_to(wbuf, dest)?;
	socket.recv(rbuf)?;
	Ok(())
}

pub fn udp_echo_round(socket: &UdpSocket, buf: &mut [u8], expected: usize) -> io::Result<usize> {
	let (amt, src) = socket.recv_from(buf)?;
	socket.send_to(&buf[..amt], src)?;
	if amt != expected {
		println!("Received {amt} bytes, expected {expected}");
	}
	Ok(amt * 2)
}

pub fn udp_echo(socket: &UdpSocket, buf: &mut [u8]) -> io::Result<()> {
	let (amt, src) = socket.recv_from(buf)?;
	socket.send_to(&buf[..amt], src)?;
	Ok(())
}
