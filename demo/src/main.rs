// Copyright (c) 2019 Stefan Lankes, RWTH Aachen University
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

#![allow(dead_code)]
#![allow(unused_imports)]

#[cfg(target_os = "hermit")]
extern crate hermit_sys;
extern crate rand;
extern crate rayon;

mod tests;

use tests::*;

fn test_result<T>(result: Result<(), T>) -> &'static str {
	match result {
		Ok(_) => "ok",
		Err(_) => "failed!",
	}
}

fn main() {
	println!("Test {} ... {}", stringify!(hello), test_result(hello()));
	println!(
		"Test {} ... {}",
		stringify!(print_argv),
		test_result(print_argv())
	);
	println!(
		"Test {} ... {}",
		stringify!(print_env),
		test_result(print_env())
	);
	println!(
		"Test {} ... {}",
		stringify!(read_file),
		test_result(read_file())
	);
	println!(
		"Test {} ... {}",
		stringify!(create_file),
		test_result(create_file())
	);
	println!(
		"Test {} ... {}",
		stringify!(threading),
		test_result(threading())
	);
	/*println!(
		"Test {} ... {}",
		stringify!(random_number),
		test_result(random_number())
	);*/
	println!(
		"Test {} ... {}",
		stringify!(pi_sequential),
		test_result(pi_sequential(5000000))
	);
	println!(
		"Test {} ... {}",
		stringify!(pi_parallel),
		test_result(pi_parallel(2, 5000000))
	);
	/*println!(
		"Test {} ... {}",
		stringify!(laplace),
		test_result(laplace(128, 128))
	);
	println!(
		"Test {} ... {}",
		stringify!(test_matmul_strassen),
		test_result(test_matmul_strassen())
	);*/
	/*println!(
		"Test {} ... {}",
		stringify!(thread_creation),
		test_result(thread_creation())
	);
	println!(
		"Test {} ... {}",
		stringify!(bench_sched_one_thread),
		test_result(bench_sched_one_thread())
	);
	println!(
		"Test {} ... {}",
		stringify!(bench_sched_two_threads),
		test_result(bench_sched_two_threads())
	);*/
	println!(
		"Test {} ... {}",
		stringify!(test_http_request),
		test_result(test_http_request())
	);
}
