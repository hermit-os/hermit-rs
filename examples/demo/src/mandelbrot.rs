use std::time::Instant;

use rayon::prelude::*;

const MAX_ITER: u32 = if cfg!(debug_assertions) { 100 } else { 1000 };
const WIDTH: usize = if cfg!(debug_assertions) { 100 } else { 400 };
const HEIGHT: usize = if cfg!(debug_assertions) { 100 } else { 400 };
const SCALE: f64 = 3.0;

#[derive(Debug)]
pub enum Mode {
	Sequential,
	Parallel,
}

fn mandelbrot_at_point(cx: f64, cy: f64) -> u32 {
	let mut x = 0.0;
	let mut y = 0.0;
	let mut iter = 0;

	while iter < MAX_ITER && x * x + y * y <= 4.0 {
		let xtemp = x * x - y * y + cx;
		y = 2.0 * x * y + cy;
		x = xtemp;
		iter += 1;
	}
	iter
}

fn calculate_mandelbrot(mode: Mode) -> u64 {
	eprintln!();
	eprint!("Calculating Mandelbrot {:10}", format!("({mode:?}): "));

	let scale_x = SCALE / WIDTH as f64;
	let scale_y = SCALE / HEIGHT as f64;

	let now = Instant::now();

	let iter_count: u64 = match mode {
		Mode::Sequential => {
			let mut sum = 0u64;
			for y in 0..HEIGHT {
				for x in 0..WIDTH {
					let cx = (x as f64) * scale_x - SCALE / 2.0;
					let cy = (y as f64) * scale_y - SCALE / 2.0;
					sum += mandelbrot_at_point(cx, cy) as u64;
				}
			}
			sum
		}
		Mode::Parallel => (0..HEIGHT)
			.into_par_iter()
			.map(|y| {
				(0..WIDTH)
					.into_par_iter()
					.map(|x| {
						let cx = (x as f64) * scale_x - SCALE / 2.0;
						let cy = (y as f64) * scale_y - SCALE / 2.0;
						mandelbrot_at_point(cx, cy) as u64
					})
					.sum::<u64>()
			})
			.sum(),
	};

	let elapsed = now.elapsed();
	eprintln!("{elapsed:?}");
	iter_count
}

pub fn mandelbrot() {
	eprintln!();
	eprintln!("Mandelbrot Set ({WIDTH}x{HEIGHT})");

	let seq_count = calculate_mandelbrot(Mode::Sequential);
	let par_count = calculate_mandelbrot(Mode::Parallel);

	assert_eq!(
		seq_count, par_count,
		"Sequential and parallel results differ!"
	);
}
