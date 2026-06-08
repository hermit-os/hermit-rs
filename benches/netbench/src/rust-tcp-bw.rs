use std::io::{self, ErrorKind, Read, Write};
use std::net::TcpStream;
use std::time::Instant;

use clap::{Parser, Subcommand};
#[cfg(target_os = "hermit")]
use hermit as _;
use hermit_bench_output::log_benchmark_data;
use rust_tcp_io_perf::config::Config;
use rust_tcp_io_perf::print_utils::{BoxplotValues, ProgressPrinter};
use rust_tcp_io_perf::{connection, threading};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
	#[command(subcommand)]
	command: Command,
}

#[derive(Subcommand)]
enum Command {
	/// Run the bandwidth server
	Server(Config),
	/// Run the bandwidth client
	Client(Config),
}

fn send_rounds(stream: &mut TcpStream, rounds: usize, bytes: usize, progress_print: bool) {
	let buf = vec![0; bytes];
	let progress_printer = ProgressPrinter::new(rounds, progress_print);

	for i in 0..rounds {
		progress_printer.print(i);
		let mut pos = 0;

		while pos < buf.len() {
			let bytes_written = match stream.write(&buf[pos..]) {
				Ok(len) => len,
				Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => 0,
				Err(e) => panic!("encountered IO error: {e}"),
			};
			pos += bytes_written;
		}
	}
	stream.flush().expect("Unexpected behaviour");
}

fn receive_rounds(
	stream: &mut TcpStream,
	rounds: usize,
	bytes: usize,
	progress_print: bool,
) -> Vec<f64> {
	let mut buf = vec![0; bytes];
	let mut durations = Vec::with_capacity(rounds);

	let progress_printer = ProgressPrinter::new(rounds, progress_print);

	for i in 0..rounds {
		progress_printer.print(i);
		let round_start = Instant::now();
		if let Err(e) = stream.read_exact(&mut buf) {
			if e.kind() == ErrorKind::UnexpectedEof {
				println!("Client ended transmission after {i} rounds");
				break;
			} else {
				panic!("Error in reading from stream: {}", e.kind());
			}
		}
		let round_end = Instant::now();
		let duration = round_end.duration_since(round_start);
		let mbits = buf.len() as f64 * 8.0f64 / (1024.0f64 * 1024.0f64 * duration.as_secs_f64());
		durations.push(mbits);
	}
	durations
}

fn run_client(args: Config) {
	println!("Connecting to the server {}:{}...", args.address, args.port);

	if let Ok(mut stream) = connection::client_connect(args.address_and_port()) {
		connection::setup(&args, &stream);
		println!("Connection established! Ready to send...");

		send_rounds(&mut stream, args.warmup, args.n_bytes, false);
		println!("Warmup done!");
		send_rounds(&mut stream, args.n_rounds, args.n_bytes, true);

		stream.flush().expect("Unexpected behaviour");
		connection::close_connection(&stream);

		println!("Sent everything!");
	} else {
		println!("Couldn't connect to server...");
	}
}

fn run_server(args: Config) {
	println!(
		"starting server with {} bytes, {} warmup rounds and {} rounds",
		args.n_bytes, args.warmup, args.n_rounds
	);
	let mut stream = connection::server_listen_and_get_first_connection(&args.port.to_string());
	connection::setup(&args, &stream);
	threading::setup(&args);

	let _ = receive_rounds(&mut stream, args.warmup, args.n_bytes, false);
	let durations = receive_rounds(&mut stream, args.n_rounds, args.n_bytes, true);

	let statistics = BoxplotValues::<f64>::from(durations.as_slice());
	log_benchmark_data("TCP server", "Mbit/s", statistics.mean);

	println!("{statistics:#.2?}");
	println!(
		"{} outliers ({:.1}%)",
		statistics.nr_outliers,
		100.0 * statistics.nr_outliers as f64 / durations.len() as f64
	);

	connection::close_connection(&stream);
}

fn main() {
	match Cli::parse().command {
		Command::Server(args) => run_server(args),
		Command::Client(args) => run_client(args),
	}
}
