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

trait StatCalcs: Sized {
	fn mean(data: &[Self]) -> Option<Self>;
	fn std_deviation(data: &[Self]) -> Option<f64>;
}
impl StatCalcs for f64 {
	fn mean(data: &[f64]) -> Option<f64> {
		let sum = data.iter().sum::<f64>();
		match data.len() {
			positive if positive > 0 => Some(sum / positive as f64),
			_ => None,
		}
	}

	fn std_deviation(data: &[f64]) -> Option<f64> {
		match (Self::mean(data), data.len()) {
			(Some(data_mean), count) if count > 0 => {
				let variance = data
					.iter()
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
}
impl StatCalcs for u64 {
	fn mean(data: &[u64]) -> Option<u64> {
		let sum = data.iter().sum::<u64>();
		match data.len() {
			positive if positive > 0 => Some(sum / positive as u64),
			_ => None,
		}
	}

	fn std_deviation(data: &[u64]) -> Option<f64> {
		match (Self::mean(data), data.len()) {
			(Some(data_mean), count) if count > 0 => {
				let variance = data
					.iter()
					.map(|value| {
						let diff = data_mean as f64 - *value as f64;

						diff * diff
					})
					.sum::<f64>() / count as f64;

				Some((variance as f64).sqrt())
			}
			_ => None,
		}
	}
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct BoxplotValues<T> {
	pub whisk_min: T,
	pub whisk_max: T,
	pub median: T,
	pub q1: T,
	pub q3: T,
	pub nr_outliers: usize,
	pub mean: T,
	pub std_deviation: f64,
}
impl<
		T: Clone
			+ PartialOrd<T>
			+ std::ops::Mul<T, Output = T>
			+ std::ops::Sub<T, Output = T>
			+ std::ops::Add<T, Output = T>
			+ std::ops::Div<Output = T>
			+ std::marker::Copy
			+ Default
			+ StatCalcs
			+ std::convert::From<u8>,
	> From<&[T]> for BoxplotValues<T>
{
	fn from(value: &[T]) -> Self {
		let mut durations = Vec::from(value);
		durations.sort_by(|a, b| a.partial_cmp(b).unwrap());
		let median = *durations
			.get(durations.len() / 2)
			.unwrap_or(&&Default::default());

		let q1 = *durations.get(durations.len() / 4).unwrap();
		let q3 = *durations.get(durations.len() * 3 / 4).unwrap();
		let iqr: T = <u8 as Into<T>>::into(3_u8) * (q3 - q1) / <u8 as Into<T>>::into(2_u8);
		let outlier_min = q1 - iqr.into();
		let outlier_max = q3 + iqr.into();
		let outliers = durations
			.iter()
			.filter(|&x| *x < outlier_min || *x > outlier_max)
			.copied()
			.collect::<Vec<T>>();

		let filtered_durations = durations
			.iter()
			.filter(|&x| *x >= outlier_min && *x <= outlier_max)
			.collect::<Vec<&T>>();
		let whisk_min = *filtered_durations[0];
		let whisk_max = *filtered_durations[filtered_durations.len() - 1];

		Self {
			whisk_min,
			whisk_max,
			median,
			q1,
			q3,
			nr_outliers: outliers.len(),
			std_deviation: T::std_deviation(&durations).unwrap(),
			mean: T::mean(&durations).unwrap(),
		}
	}
}
