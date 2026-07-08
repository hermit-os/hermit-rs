use std::time::Instant;
use std::{f64, hint};

use rayon::prelude::*;

const STEPS: u64 = if cfg!(debug_assertions) {
	50_000
} else {
	5_000_000
};
const STEP_SIZE: f64 = 1.0 / STEPS as f64;

#[derive(Debug)]
pub enum Mode {
	Sequential,
	Parallel,
}

fn calculate_pi(mode: Mode) {
	eprintln!();
	eprint!("Calculating Pi {:14}", format!("({mode:?}): "));

	let steps = hint::black_box(STEPS);
	let map_step = |i| {
		let x = (i as f64 + 0.5) * STEP_SIZE;
		4.0 / (1.0 + x * x)
	};

	let now = Instant::now();
	let sum = match mode {
		Mode::Sequential => (0..steps).map(map_step).sum::<f64>(),
		Mode::Parallel => (0..steps).into_par_iter().map(map_step).sum::<f64>(),
	};
	let mypi = sum * STEP_SIZE;
	let elapsed = now.elapsed();

	eprintln!("{elapsed:?}");

	assert!((mypi - f64::consts::PI).abs() < 1e-10);
}

pub fn pi() {
	eprintln!();
	calculate_pi(Mode::Sequential);
	calculate_pi(Mode::Parallel);
}
