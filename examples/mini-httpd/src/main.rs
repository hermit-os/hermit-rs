use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream};

#[cfg(target_os = "hermit")]
use hermit as _;

fn handle_connection(mut stream: TcpStream) -> io::Result<()> {
	let mut request = Vec::new();
	let mut buf = [0u8; 1024];
	while !request.windows(4).any(|w| w == b"\r\n\r\n") {
		let n = stream.read(&mut buf)?;
		if n == 0 {
			break;
		}
		request.extend_from_slice(&buf[..n]);
	}

	let request = String::from_utf8_lossy(&request);
	eprintln!("{:?}", request.lines().next().unwrap_or_default());

	let body = "Hello from mini httpd\n";
	let response = format!(
		"HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
		body.len()
	);
	stream.write_all(response.as_bytes())
}

fn main() {
	let listener = TcpListener::bind("0.0.0.0:9975").unwrap();
	eprintln!("Now listening on port 9975");

	loop {
		for stream in listener.incoming() {
			if let Err(e) = stream.and_then(handle_connection) {
				eprintln!("Error: {e}");
			}

			if cfg!(feature = "ci") {
				break;
			}
		}
	}
}
