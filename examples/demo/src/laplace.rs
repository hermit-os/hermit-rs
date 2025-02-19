//! Jacobi Stencil Iterations
//!
//! This module performs the Jacobi method for solving Laplace's differential equation.

#![allow(clippy::reversed_empty_ranges)]

use std::mem;
use std::time::Instant;

use ndarray::{s, Array2, ArrayView2, ArrayViewMut2};
use rayon::prelude::*;

const SIZE: usize = if cfg!(debug_assertions) { 16 } else { 64 };
const ITERATIONS: usize = 1000;

pub fn laplace() {
	eprintln!();

	let mut matrix = matrix_setup(SIZE, SIZE);

	eprintln!("Laplace iterations");
	let now = Instant::now();
	let residual = compute(matrix.view_mut(), ITERATIONS);
	let elapsed = now.elapsed();
	eprintln!("{ITERATIONS} iterations: {elapsed:?} (residual: {residual})");

	assert!(residual < 0.001);
}

fn matrix_setup(size_x: usize, size_y: usize) -> Array2<f64> {
	let mut matrix = Array2::zeros((size_x, size_y));

	matrix.row_mut(0).fill(1.0);
	matrix.row_mut(size_x - 1).fill(1.0);
	matrix.column_mut(0).fill(1.0);
	matrix.column_mut(size_x - 1).fill(1.0);

	matrix
}

fn get_residual(matrix: ArrayView2<f64>) -> f64 {
	matrix
		.slice(s![1..-1, ..])
		.outer_iter()
		.into_par_iter()
		.enumerate()
		.map(|(i, row)| {
			let i = i + 1; // To compensate slicing

			let up = matrix.row(i - 1);
			let here = matrix.row(i);
			let down = matrix.row(i + 1);
			let len = row.len();
			assert_eq!(up.len(), len);
			assert_eq!(here.len(), len);
			assert_eq!(down.len(), len);

			let mut acc = 0.0;
			for j in 1..len - 1 {
				let sum = up[j] + down[j] + here[j - 1] + here[j + 1];
				let new = sum * 0.25;
				acc += (new - here[j]).powi(2);
			}
			acc
		})
		.sum()
}

fn iteration(current: ArrayView2<f64>, mut next: ArrayViewMut2<f64>) {
	next.slice_mut(s![1..-1, ..])
		.outer_iter_mut()
		.into_par_iter()
		.enumerate()
		.for_each(|(i, mut row)| {
			let i = i + 1; // To compensate slicing

			let up = current.row(i - 1);
			let here = current.row(i);
			let down = current.row(i + 1);
			let len = row.len();
			assert_eq!(up.len(), len);
			assert_eq!(here.len(), len);
			assert_eq!(down.len(), len);

			for j in 1..len - 1 {
				let sum = up[j] + down[j] + here[j - 1] + here[j + 1];
				row[j] = sum * 0.25;
			}
		});
}

fn compute(mut matrix: ArrayViewMut2<'_, f64>, iterations: usize) -> f64 {
	let mut owned = matrix.to_owned();

	let mut current = matrix.view_mut();
	let mut next = owned.view_mut();

	for _ in 0..iterations {
		iteration(current.view(), next.view_mut());
		mem::swap(&mut current, &mut next);
	}

	get_residual(current.view())
}
