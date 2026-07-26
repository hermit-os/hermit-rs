use hermit_bench_output::log_benchmark_data;

use crate::config::Config;
use crate::print_utils::BoxplotValues;
use crate::Protocol;

pub struct LatencyResult {
	pub hist: hdrhist::HDRHist,
	pub latencies: Vec<u64>,
}

pub trait Benchmark {
	type Result;
	const LABEL: &'static str;

	fn run(protocol: Protocol, args: &Config) -> Self::Result;
	fn log_statistics(protocol: Protocol, result: &Self::Result);

	fn execute(protocol: Protocol, args: Config) {
		println!(
			"{}: {} bytes, {} warmup rounds, {} rounds",
			Self::LABEL,
			args.n_bytes,
			args.warmup,
			args.n_rounds
		);
		let result = Self::run(protocol, &args);
		Self::log_statistics(protocol, &result);
	}
}

fn get_percentiles(summary: impl Iterator<Item = (f64, u64, u64)>, percentile: f64) -> f64 {
	let mut res = 0.0;

	for (quantile, lower, upper) in summary {
		if quantile == percentile {
			res = (lower as f64 + upper as f64) / 2.0;
		}
	}

	res
}

pub fn log_bw_statistics(protocol: Protocol, durations: &[f64]) {
	let label = match protocol {
		Protocol::Tcp => "TCP server",
		Protocol::Udp => "UDP server",
	};

	let statistics = BoxplotValues::<f64>::from(durations);
	log_benchmark_data(label, "Mbit/s", statistics.mean);
	println!("{statistics:#.2?}");
	println!(
		"{} outliers ({:.1}%)",
		statistics.nr_outliers,
		100.0 * statistics.nr_outliers as f64 / durations.len() as f64
	);
}

pub fn log_latency_statistics(protocol: Protocol, result: &LatencyResult) {
	let (p95_label, max_label) = match protocol {
		Protocol::Tcp => (
			"95th percentile TCP Client Latency",
			"Max TCP Client Latency",
		),
		#[cfg(not(target_os = "hermit"))]
		Protocol::Udp => (
			"95th percentile UDP Server Latency",
			"Max UDP Server Latency",
		),
		#[cfg(target_os = "hermit")]
		Protocol::Udp => (
			"95th percentile UDP Client Latency",
			"Max UDP Client Latency",
		),
	};

	log_benchmark_data(
		p95_label,
		"ns",
		get_percentiles(result.hist.summary(), 0.95),
	);
	log_benchmark_data(max_label, "ns", get_percentiles(result.hist.summary(), 1.0));

	if matches!(protocol, Protocol::Tcp) {
		let statistics = BoxplotValues::from(result.latencies.as_slice());
		println!("{statistics:#.2?}");
	}
}
