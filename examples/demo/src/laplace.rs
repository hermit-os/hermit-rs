//! Jacobi Stencil Iterations
//!
//! This module performs the Jacobi method for solving Laplace's differential equation.

use std::time::Instant;
use std::{mem, vec};

use rayon::prelude::*;

const SIZE: usize = if cfg!(debug_assertions) { 16 } else { 64 };
const ITERATIONS: usize = 1000;

pub fn laplace() {
	eprintln!();

	let mut matrix = matrix_setup(SIZE, SIZE);

	eprintln!("Laplace iterations");
	let now = Instant::now();
	let residual = compute(&mut matrix, SIZE, SIZE, ITERATIONS);
	let elapsed = now.elapsed();
	eprintln!("{ITERATIONS} iterations: {elapsed:?} (residual: {residual})");

	assert!(residual < 0.001);
}

fn matrix_setup(size_x: usize, size_y: usize) -> vec::Vec<f64> {
	let mut matrix = vec![0.0; size_x * size_y];

	// top row
	for f in matrix.iter_mut().take(size_x) {
		*f = 1.0;
	}

	// bottom row
	for x in 0..size_x {
		matrix[(size_y - 1) * size_x + x] = 1.0;
	}

	// left row
	for y in 0..size_y {
		matrix[y * size_x] = 1.0;
	}

	// right row
	for y in 0..size_y {
		matrix[y * size_x + size_x - 1] = 1.0;
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

fn compute(matrix: &mut [f64], size_x: usize, size_y: usize, iterations: usize) -> f64 {
	let mut clone = matrix.to_vec();

	let mut current = matrix;
	let mut next = &mut clone[..];

	for _ in 0..iterations {
		iteration(current, next, size_x, size_y);
		mem::swap(&mut current, &mut next);
	}

	get_residual(current, size_x, size_y)
}
