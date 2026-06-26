use std::net::TcpStream;
use std::time::Instant;
use std::{thread, time};

use crate::benchmark::{log_latency_statistics, Benchmark, LatencyResult};
use crate::config::Config;
use crate::{connection, threading, Protocol};

fn duration_nanos(duration: std::time::Duration) -> u64 {
	duration.as_secs() * 1_000_000_000u64 + duration.subsec_nanos() as u64
}

fn run_latency_rounds(
	warmup: usize,
	n_rounds: usize,
	progress_interval: usize,
	mut exchange: impl FnMut() -> std::time::Duration,
) -> LatencyResult {
	let mut hist = hdrhist::HDRHist::new();
	let mut latencies = Vec::with_capacity(n_rounds);

	for _ in 0..warmup {
		exchange();
	}

	for i in 0..n_rounds {
		let duration = exchange();
		let duration_nanos = duration_nanos(duration);
		hist.add_value(duration_nanos);
		latencies.push(duration_nanos);

		if progress_interval > 0 && i % progress_interval == 0 {
			println!("{}% completed", i / progress_interval);
		}
	}

	LatencyResult { hist, latencies }
}

fn tcp_client_connect(args: &Config) -> TcpStream {
	const MAX_RETRIES: i32 = 30;
	let mut retries = 0;

	loop {
		match connection::client_connect(args.address_and_port()) {
			Ok(stream) => return stream,
			Err(error) => {
				retries += 1;
				println!(
					"Couldn't connect to server, retrying ({retries}/{MAX_RETRIES})... ({error})"
				);
				if retries >= MAX_RETRIES {
					panic!("Can't establish connection to server. Aborting after {MAX_RETRIES} attempts");
				}
				thread::sleep(time::Duration::from_secs(1));
			}
		}
	}
}

pub enum LatencyClient {
	Tcp {
		stream: TcpStream,
		wbuf: Vec<u8>,
		rbuf: Vec<u8>,
		n_bytes: usize,
	},
	Udp {
		socket: std::net::UdpSocket,
		wbuf: Vec<u8>,
		rbuf: Vec<u8>,
		dest: String,
	},
}

impl LatencyClient {
	fn connect(protocol: Protocol, args: &Config) -> Self {
		match protocol {
			Protocol::Tcp => {
				let stream = tcp_client_connect(args);
				connection::setup(args, &stream);
				threading::setup(args);
				println!("Connection established! Ready to send...");
				Self::Tcp {
					stream,
					wbuf: vec![0; args.n_bytes],
					rbuf: vec![0; args.n_bytes],
					n_bytes: args.n_bytes,
				}
			}
			Protocol::Udp => {
				let socket = connection::bind_udp_client().expect("Couldn't bind to address");
				println!("Ready to send...");
				Self::Udp {
					socket,
					wbuf: vec![0; args.n_bytes],
					rbuf: vec![0; args.n_bytes],
					dest: args.address_and_port(),
				}
			}
		}
	}

	fn warmup(&self, config_warmup: usize) -> usize {
		match self {
			Self::Tcp { .. } => config_warmup,
			Self::Udp { .. } => 0,
		}
	}

	fn progress_interval(&self, n_rounds: usize) -> usize {
		match self {
			Self::Tcp { .. } => n_rounds / 100,
			Self::Udp { .. } => (n_rounds * 2) / 100,
		}
	}

	fn exchange(&mut self) -> std::time::Duration {
		let start = Instant::now();
		match self {
			Self::Tcp {
				stream,
				wbuf,
				rbuf,
				n_bytes,
			} => {
				connection::send_message(*n_bytes, stream, wbuf);
				connection::receive_message(*n_bytes, stream, rbuf);
			}
			Self::Udp {
				socket,
				wbuf,
				rbuf,
				dest,
			} => {
				connection::udp_exchange(socket, wbuf, rbuf, dest).expect("Couldn't exchange data");
			}
		}
		start.elapsed()
	}

	fn teardown(self) {
		if let Self::Tcp { stream, .. } = self {
			connection::close_connection(&stream);
		}
	}
}

impl Benchmark for LatencyClient {
	type Result = LatencyResult;

	const LABEL: &'static str = "latency client";

	fn run(protocol: Protocol, args: &Config) -> Self::Result {
		println!("Connecting to the server {}...", args.address);

		let mut client = Self::connect(protocol, args);
		let warmup = client.warmup(args.warmup);
		let progress_interval = client.progress_interval(args.n_rounds);

		let result = run_latency_rounds(warmup, args.n_rounds, progress_interval, || {
			client.exchange()
		});

		client.teardown();
		result
	}

	fn log_statistics(protocol: Protocol, result: &Self::Result) {
		log_latency_statistics(protocol, result);
	}
}

pub enum LatencyServer {
	Tcp {
		stream: TcpStream,
		buf: Vec<u8>,
		n_bytes: usize,
	},
	Udp {
		socket: std::net::UdpSocket,
		buf: Vec<u8>,
	},
}

impl LatencyServer {
	fn listen(protocol: Protocol, args: &Config) -> Self {
		match protocol {
			Protocol::Tcp => {
				let n_bytes = args.n_bytes;
				let stream =
					connection::server_listen_and_get_first_connection(&args.port.to_string());
				connection::setup(args, &stream);
				threading::setup(args);
				Self::Tcp {
					stream,
					buf: vec![0; n_bytes],
					n_bytes,
				}
			}
			Protocol::Udp => {
				let socket =
					connection::bind_udp_server(args.port).expect("Couldn't bind to address");
				println!("Server listening on port {}", args.port);
				Self::Udp {
					socket,
					buf: vec![0; args.n_bytes],
				}
			}
		}
	}

	fn serve(&mut self, warmup: usize, n_rounds: usize) {
		match self {
			Self::Tcp {
				stream,
				buf,
				n_bytes,
			} => {
				for _ in 0..(n_rounds + warmup) {
					connection::receive_message(*n_bytes, stream, buf);
					connection::send_message(*n_bytes, stream, buf);
				}
			}
			Self::Udp { socket, buf } => {
				for _ in 0..n_rounds {
					connection::udp_echo(socket, buf).expect("Couldn't echo data");
				}
			}
		}
	}
}

impl Benchmark for LatencyServer {
	type Result = ();

	const LABEL: &'static str = "latency server";

	fn run(protocol: Protocol, args: &Config) -> Self::Result {
		let mut server = Self::listen(protocol, args);
		server.serve(args.warmup, args.n_rounds);
		println!("Done exchanging stuff");
	}

	fn log_statistics(_protocol: Protocol, _result: &Self::Result) {}
}
