use std::io::{ErrorKind, Read};
use std::net::TcpStream;
use std::time::Instant;

use clap::Parser;
#[cfg(target_os = "hermit")]
use hermit as _;
use hermit_bench_output::log_benchmark_data;
use rust_tcp_io_perf::config::Config;
use rust_tcp_io_perf::connection;

fn mean(data: &[f64]) -> Option<f64> {
	let sum = data.iter().sum::<f64>();
	match data.len() {
		positive if positive > 0 => Some(sum / positive as f64),
		_ => None,
	}
}

fn std_deviation(data: &[f64]) -> Option<f64> {
	match (mean(data), data.len()) {
		(Some(data_mean), count) if count > 0 => {
			let variance =
				data.iter()
					.map(|value| {
						let diff = data_mean - *value;

						diff * diff
					})
					.sum::<f64>() / count as f64;

			Some(variance.sqrt())
		}
		_ => None,
	}
}

#[derive(Debug)]
#[allow(dead_code)]
struct BoxplotValues {
	whisk_min: f64,
	whisk_max: f64,
	median: f64,
	q1: f64,
	q3: f64,
	nr_outliers: usize,
	mean: f64,
	std_deviation: f64,
}

fn calculate_boxplot(durations: &[f64]) -> BoxplotValues {
	let mut durations = Vec::from(durations);
	durations.sort_by(|a, b| a.partial_cmp(b).unwrap());
	let q1 = *durations.get(durations.len() / 4).unwrap();
	let q3 = *durations.get(durations.len() * 3 / 4).unwrap();
	let outlier_min = q1 - 1.5 * (q3 - q1);
	let outlier_max = q3 + 1.5 * (q3 - q1);
	let filtered_durations = durations
		.iter()
		.filter(|&x| *x >= outlier_min && *x <= outlier_max)
		.collect::<Vec<&f64>>();

	let min = *filtered_durations[0];
	let max = *filtered_durations[filtered_durations.len() - 1];
	let median = **filtered_durations
		.get(filtered_durations.len() / 2)
		.unwrap_or(&&0.0);
	let outliers = durations
		.iter()
		.filter(|&x| *x < outlier_min || *x > outlier_max)
		.copied()
		.collect::<Vec<f64>>();
	BoxplotValues {
		whisk_min: min,
		whisk_max: max,
		median,
		q1,
		q3,
		nr_outliers: outliers.len(),
		std_deviation: std_deviation(&durations).unwrap(),
		mean: mean(&durations).unwrap(),
	}
}

fn receive_rounds(
	stream: &mut TcpStream,
	rounds: usize,
	bytes: usize,
	progress_print: bool,
) -> Vec<f64> {
	let mut buf = vec![0; bytes];
	let mut durations = Vec::with_capacity(rounds);

	let progress_prints = [
		1,
		rounds / 10,
		rounds / 10 * 2,
		rounds / 10 * 3,
		rounds / 10 * 4,
		rounds / 10 * 5,
		rounds / 10 * 6,
		rounds / 10 * 7,
		rounds / 10 * 8,
		rounds / 10 * 9,
	];
	for i in 0..rounds {
		if progress_print && progress_prints.contains(&i) {
			println!("round {i}/{}", rounds)
		}
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

fn main() {
	let args = Config::parse();

	println!(
		"starting server with {} bytes, {} warmup rounds and {} rounds",
		args.n_bytes, args.warmup, args.n_rounds
	);
	let mut stream = connection::server_listen_and_get_first_connection(&args.port.to_string());
	connection::setup(&args, &stream);

	let _ = receive_rounds(&mut stream, args.warmup, args.n_bytes, false);
	let durations = receive_rounds(&mut stream, args.n_rounds, args.n_bytes, true);

	log_benchmark_data("TCP server", "Mbit/s", mean(&durations).unwrap());

	let statistics = calculate_boxplot(&durations);
	println!("{statistics:#.2?}");
	println!(
		"{} outliers ({:.1}%)",
		statistics.nr_outliers,
		100.0 * statistics.nr_outliers as f64 / durations.len() as f64
	);

	connection::close_connection(&stream);
}
