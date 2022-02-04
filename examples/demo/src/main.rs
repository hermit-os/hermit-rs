#![allow(dead_code)]
#![allow(unused_imports)]
#![feature(thread_id_value)]

#[cfg(target_os = "hermit")]
use hermit_sys as _;

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
		stringify!(test_thread_local),
		test_result(test_thread_local())
	);
	println!(
		"Test {} ... {}",
		stringify!(arithmetic),
		test_result(arithmetic())
	);
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
	println!(
		"Test {} ... {}",
		stringify!(pi_sequential),
		test_result(pi_sequential(5000000))
	);
	println!(
		"Test {} ... {}",
		stringify!(pi_parallel),
		test_result(pi_parallel(5000000))
	);
	println!(
		"Test {} ... {}",
		stringify!(laplace),
		test_result(laplace(128, 128))
	);
	println!(
		"Test {} ... {}",
		stringify!(test_matmul_strassen),
		test_result(test_matmul_strassen())
	);
	println!(
		"Test {} ... {}",
		stringify!(thread_creation),
		test_result(thread_creation())
	);
}
