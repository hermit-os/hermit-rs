use std::io::{self, ErrorKind, Read, Write};
use std::net::{TcpStream, UdpSocket};
use std::time::Instant;

use crate::benchmark::{log_bw_statistics, Benchmark};
use crate::config::Config;
use crate::print_utils::ProgressPrinter;
use crate::{connection, threading, Protocol};

fn bytes_to_mbits(bytes: usize, duration: std::time::Duration) -> f64 {
	bytes as f64 * 8.0 / (1024.0 * 1024.0 * duration.as_secs_f64())
}

fn send_rounds<F>(rounds: usize, bytes: usize, progress_print: bool, mut send_round: F)
where
	F: FnMut(&[u8]),
{
	let buf = vec![0; bytes];
	let progress_printer = ProgressPrinter::new(rounds, progress_print);

	for i in 0..rounds {
		progress_printer.print(i);
		send_round(&buf);
	}
}

fn receive_rounds<F>(rounds: usize, bytes: usize, progress_print: bool, mut receive: F) -> Vec<f64>
where
	F: FnMut(&mut [u8]) -> Result<usize, io::Error>,
{
	let mut buf = vec![0; bytes];
	let mut mbits = Vec::with_capacity(rounds);
	let progress_printer = ProgressPrinter::new(rounds, progress_print);

	for i in 0..rounds {
		progress_printer.print(i);
		let round_start = Instant::now();
		match receive(&mut buf) {
			Ok(amt) => mbits.push(bytes_to_mbits(amt, round_start.elapsed())),
			Err(e) if e.kind() == ErrorKind::UnexpectedEof => {
				println!("Client ended transmission after {i} rounds");
				break;
			}
			Err(e) => panic!("Error in reading from stream: {}", e.kind()),
		}
	}
	mbits
}

fn run_send_rounds(
	warmup: usize,
	n_rounds: usize,
	n_bytes: usize,
	mut send_round: impl FnMut(&[u8]),
) {
	send_rounds(warmup, n_bytes, false, &mut send_round);
	println!("Warmup done!");
	send_rounds(n_rounds, n_bytes, true, &mut send_round);
}

fn run_receive_rounds(
	warmup: usize,
	n_rounds: usize,
	n_bytes: usize,
	mut receive_round: impl FnMut(&mut [u8]) -> Result<usize, io::Error>,
) -> Vec<f64> {
	let _ = receive_rounds(warmup, n_bytes, false, &mut receive_round);
	println!("Warmup done!");
	receive_rounds(n_rounds, n_bytes, true, &mut receive_round)
}

pub enum BwClient {
	Tcp {
		stream: TcpStream,
		n_bytes: usize,
	},
	Udp {
		socket: UdpSocket,
		dest: String,
		rbuf: Vec<u8>,
	},
}

impl BwClient {
	fn connect(protocol: Protocol, args: &Config) -> Option<Self> {
		match protocol {
			Protocol::Tcp => {
				let stream = connection::client_connect(args.address_and_port()).ok()?;
				connection::setup(args, &stream);
				println!("Connection established! Ready to send...");
				Some(Self::Tcp {
					stream,
					n_bytes: args.n_bytes,
				})
			}
			Protocol::Udp => {
				println!("Binding to address {}...", connection::UDP_CLIENT_BIND_ADDR);
				let socket = connection::bind_udp_client().ok()?;
				println!("Socket open! Ready to send...");
				Some(Self::Udp {
					socket,
					dest: args.address_and_port(),
					rbuf: vec![0; args.n_bytes],
				})
			}
		}
	}

	fn send_round(&mut self, buf: &[u8]) {
		match self {
			Self::Tcp { stream, n_bytes } => connection::send_message(*n_bytes, stream, buf),
			Self::Udp { socket, dest, rbuf } => {
				connection::udp_exchange(socket, buf, rbuf, dest).expect("Couldn't exchange data");
			}
		}
	}

	fn teardown(self) {
		match self {
			Self::Tcp { mut stream, .. } => {
				stream.flush().expect("Unexpected behaviour");
				connection::close_connection(&stream);
			}
			Self::Udp { .. } => {}
		}
	}
}

impl Benchmark for BwClient {
	type Result = ();

	const LABEL: &'static str = "bandwidth client";

	fn run(protocol: Protocol, args: &Config) -> Self::Result {
		println!("Connecting to the server {}:{}...", args.address, args.port);

		let Some(mut client) = Self::connect(protocol, args) else {
			println!("Couldn't connect to server...");
			return;
		};

		run_send_rounds(args.warmup, args.n_rounds, args.n_bytes, |buf| {
			client.send_round(buf)
		});
		client.teardown();

		println!("Sent everything!");
	}

	fn log_statistics(_protocol: Protocol, _result: &Self::Result) {}
}

pub enum BwServer {
	Tcp { stream: TcpStream },
	Udp { socket: UdpSocket, n_bytes: usize },
}

impl BwServer {
	fn listen(protocol: Protocol, args: &Config) -> Self {
		match protocol {
			Protocol::Tcp => {
				let stream =
					connection::server_listen_and_get_first_connection(&args.port.to_string());
				connection::setup(args, &stream);
				threading::setup(args);
				Self::Tcp { stream }
			}
			Protocol::Udp => {
				let n_bytes = args.n_bytes;
				let socket = connection::bind_udp_server(args.port).expect("Failed to bind socket");
				Self::Udp { socket, n_bytes }
			}
		}
	}

	fn receive_round(&mut self, buf: &mut [u8]) -> Result<usize, io::Error> {
		match self {
			Self::Tcp { stream } => stream.read_exact(buf).map(|()| buf.len()),
			Self::Udp { socket, n_bytes } => connection::udp_echo_round(socket, buf, *n_bytes),
		}
	}

	fn teardown(self) {
		if let Self::Tcp { stream } = self {
			connection::close_connection(&stream);
		}
	}
}

impl Benchmark for BwServer {
	type Result = Vec<f64>;

	const LABEL: &'static str = "bandwidth server";

	fn run(protocol: Protocol, args: &Config) -> Self::Result {
		let mut server = Self::listen(protocol, args);
		let durations = run_receive_rounds(args.warmup, args.n_rounds, args.n_bytes, |buf| {
			server.receive_round(buf)
		});
		server.teardown();
		durations
	}

	fn log_statistics(protocol: Protocol, result: &Self::Result) {
		log_bw_statistics(protocol, result);
	}
}
