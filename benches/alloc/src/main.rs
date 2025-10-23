/*
MIT License

Copyright (c) 2022 Philipp Schuster

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
*/

/* Heavily modified by Shaun Beautement. All errors are probably my own. */

/*
 * This benchmark was revised for Hermit by Stefan Lankes.
 * The original code is part of the crate `talc`.
 */

#![feature(slice_ptr_get)]

use std::alloc::{alloc, dealloc, Layout};
use std::time::Instant;

#[cfg(target_os = "hermit")]
use hermit as _;
use hermit_bench_output::log_benchmark_data;

const BENCH_DURATION: f64 = 3.0;

fn main() {
	let bench_alloc = benchmark_allocator();
	print_bench_results(&bench_alloc);
}

/// Result of a bench run.
struct BenchRunResults {
	allocation_attempts: usize,
	successful_allocations: usize,
	pre_fail_allocations: usize,
	deallocations: usize,

	/// Sorted vector of the amount of clock ticks per successful allocation.
	all_alloc_measurements: Vec<u64>,
	/// Sorted vector of the amount of clock ticks per successful allocation under heap pressure.
	nofail_alloc_measurements: Vec<u64>,
	/// Sorted vector of the amount of clock ticks per deallocation.
	dealloc_measurements: Vec<u64>,
}

fn benchmark_allocator() -> BenchRunResults {
	#[cfg(target_arch = "x86_64")]
	let mut x = 0u32;
	#[cfg(target_arch = "x86_64")]
	let mut now_fn = || unsafe { std::arch::x86_64::__rdtscp(std::ptr::addr_of_mut!(x)) };
	#[cfg(target_arch = "aarch64")]
	let now_fn = || unsafe {
		let value: u64;
		std::arch::asm!(
			"mrs {value}, cntpct_el0",
			value = out(reg) value,
			options(nostack),
		);
		value
	};
	#[cfg(target_arch = "riscv64")]
	let now_fn = riscv::register::time::read64;

	let mut active_allocations = Vec::new();

	let mut all_alloc_measurements = Vec::new();
	let mut nofail_alloc_measurements = Vec::new();
	let mut dealloc_measurements = Vec::new();

	let mut allocation_attempts = 0;
	let mut successful_allocations = 0;
	let mut pre_fail_allocations = 0;
	let mut deallocations = 0;

	let mut any_alloc_failed = false;

	// run for 10s
	let bench_begin_time = Instant::now();
	while bench_begin_time.elapsed().as_secs_f64() <= BENCH_DURATION {
		let size = fastrand::usize((1 << 6)..(1 << 15));
		let align = 8 << (fastrand::u16(..).trailing_zeros() / 2);
		let layout = Layout::from_size_align(size, align).unwrap();

		let alloc_begin = now_fn();
		let ptr = unsafe { alloc(layout) };
		let alloc_ticks = now_fn() - alloc_begin;

		allocation_attempts += 1;
		if !ptr.is_null() {
			active_allocations.push((ptr, layout));

			successful_allocations += 1;
			if !any_alloc_failed {
				pre_fail_allocations += 1;
			}
		} else {
			any_alloc_failed = true;
		}

		all_alloc_measurements.push(alloc_ticks);
		if !any_alloc_failed {
			nofail_alloc_measurements.push(alloc_ticks);
		}

		if active_allocations.len() > 10 && fastrand::usize(..10) == 0 {
			for _ in 0..7 {
				let index = fastrand::usize(..active_allocations.len());
				let allocation = active_allocations.swap_remove(index);

				let dealloc_begin = now_fn();
				unsafe {
					dealloc(allocation.0, allocation.1);
				}
				let dealloc_ticks = now_fn() - dealloc_begin;

				deallocations += 1;
				dealloc_measurements.push(dealloc_ticks);
			}
		}
	}

	while !active_allocations.is_empty() {
		let index = fastrand::usize(..active_allocations.len());
		let allocation = active_allocations.swap_remove(index);

		let dealloc_begin = now_fn();
		unsafe {
			dealloc(allocation.0, allocation.1);
		}
		let dealloc_ticks = now_fn() - dealloc_begin;

		deallocations += 1;
		dealloc_measurements.push(dealloc_ticks);
	}

	// sort
	all_alloc_measurements.sort();
	nofail_alloc_measurements.sort();
	dealloc_measurements.sort();

	BenchRunResults {
		allocation_attempts,
		successful_allocations,
		pre_fail_allocations,
		deallocations,

		all_alloc_measurements,
		nofail_alloc_measurements,
		dealloc_measurements,
	}
}

fn print_bench_results(res: &BenchRunResults) {
	let allocation_success =
		(res.successful_allocations as f64 / res.allocation_attempts as f64) * 100.0;
	log_benchmark_data("Allocation success", "%", allocation_success);

	let deallocation_success =
		(res.deallocations as f64 / res.successful_allocations as f64) * 100.0;
	log_benchmark_data("Deallocation success", "%", deallocation_success);

	let pre_fail_alloc = (res.pre_fail_allocations as f64 / res.allocation_attempts as f64) * 100.0;
	log_benchmark_data("Pre-fail Allocations", "%", pre_fail_alloc);

	let avg_all_alloc = res.all_alloc_measurements.iter().sum::<u64>() as f64
		/ res.all_alloc_measurements.len() as f64;
	log_benchmark_data("Average Allocation time", "Ticks", avg_all_alloc);

	let avg_nofail_alloc = res.nofail_alloc_measurements.iter().sum::<u64>() as f64
		/ res.nofail_alloc_measurements.len() as f64;
	log_benchmark_data(
		"Average Allocation time (no fail)",
		"Ticks",
		avg_nofail_alloc,
	);

	let avg_dealloc =
		res.dealloc_measurements.iter().sum::<u64>() as f64 / res.dealloc_measurements.len() as f64;
	log_benchmark_data("Average Deallocation time", "Ticks", avg_dealloc);
}
