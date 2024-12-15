//! Jacobi Stencil Iterations
//!
//! This module performs the Jacobi method for solving Laplace's differential equation.

use std::time::Instant;
use std::vec;

use rayon::prelude::*;

const SIZE: usize = if cfg!(debug_assertions) { 16 } else { 64 };

pub fn laplace() {
	eprintln!();

	let matrix = matrix_setup(SIZE, SIZE);

	eprintln!("Laplace iterations");
	let now = Instant::now();
	let (i, residual) = compute(matrix, SIZE, SIZE);
	let elapsed = now.elapsed();
	eprintln!("{i} iterations: {elapsed:?} (residual: {residual})");

	assert!(residual < 0.001);
}

fn matrix_setup(size_x: usize, size_y: usize) -> vec::Vec<vec::Vec<f64>> {
	let mut matrix = vec![vec![0.0; size_x * size_y]; 2];

	// top row
	for x in 0..size_x {
		matrix[0][x] = 1.0;
		matrix[1][x] = 1.0;
	}

	// bottom row
	for x in 0..size_x {
		matrix[0][(size_y - 1) * size_x + x] = 1.0;
		matrix[1][(size_y - 1) * size_x + x] = 1.0;
	}

	// left row
	for y in 0..size_y {
		matrix[0][y * size_x] = 1.0;
		matrix[1][y * size_x] = 1.0;
	}

	// right row
	for y in 0..size_y {
		matrix[0][y * size_x + size_x - 1] = 1.0;
		matrix[1][y * size_x + size_x - 1] = 1.0;
	}

	matrix
}

fn get_residual(matrix: &[f64], size_x: usize, size_y: usize) -> f64 {
	(1..size_y - 1)
		.into_par_iter()
		.map(|y| {
			let mut local_sum = 0.0;

			for x in 1..(size_x - 1) {
				let new = (matrix[y * size_x + x - 1]
					+ matrix[y * size_x + x + 1]
					+ matrix[(y + 1) * size_x + x]
					+ matrix[(y - 1) * size_x + x])
					* 0.25;

				let diff = new - matrix[y * size_x + x];
				local_sum += diff * diff;
			}

			local_sum
		})
		.sum()
}

fn iteration(cur: &[f64], next: &mut [f64], size_x: usize, size_y: usize) {
	next.par_chunks_mut(size_y)
		.enumerate() // to figure out where this chunk came from
		.for_each(|(chunk_index, slice)| {
			if chunk_index > 0 && chunk_index < size_y - 1 {
				let offset_base = chunk_index * size_x;

				for x in 1..size_x - 1 {
					slice[x] = (cur[offset_base + x - 1]
						+ cur[offset_base + x + 1]
						+ cur[offset_base + size_x + x]
						+ cur[offset_base - size_x + x])
						* 0.25;
				}
			}
		});
}

pub fn compute(mut matrix: vec::Vec<vec::Vec<f64>>, size_x: usize, size_y: usize) -> (usize, f64) {
	let mut counter = 0;

	while counter < 1000 {
		{
			// allow a borrow and a reference to the same vector
			let (current, next) = matrix.split_at_mut(1);

			iteration(&current[0], &mut next[0], size_x, size_y);
		}
		matrix.swap(0, 1);

		counter += 1;
	}

	(counter, get_residual(&matrix[0], size_x, size_y))
}
