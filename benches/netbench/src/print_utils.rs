/// Prints dashed line
fn print_line() {
	println!("\n-------------------------------------------------------------\n");
}

/// Nicely outputs summary of execution with stats and CDF points.
pub fn print_summary(hist: hdrhist::HDRHist) {
	println!("Sent/received everything!");
	print_line();
	println!("HDRHIST summary, measure in ns");
	print_line();
	println!("summary:\n{:#?}", hist.summary().collect::<Vec<_>>());
	print_line();
	println!("Summary_string:\n{}", hist.summary_string());
	print_line();
	println!("CDF summary:\n");
	for entry in hist.ccdf_upper_bound() {
		println!("{entry:?}");
	}
}

pub fn mean(data: &[f64]) -> Option<f64> {
	let sum = data.iter().sum::<f64>();
	match data.len() {
		positive if positive > 0 => Some(sum / positive as f64),
		_ => None,
	}
}

pub fn std_deviation(data: &[f64]) -> Option<f64> {
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
pub struct BoxplotValues {
	pub whisk_min: f64,
	pub whisk_max: f64,
	pub median: f64,
	pub q1: f64,
	pub q3: f64,
	pub nr_outliers: usize,
	pub mean: f64,
	pub std_deviation: f64,
}

pub fn calculate_boxplot(durations: &[f64]) -> BoxplotValues {
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
