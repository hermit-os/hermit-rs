#![allow(dead_code)]
#![allow(unused_imports)]
#![feature(thread_id_value)]

#[cfg(target_os = "hermit")]
use hermit as _;

mod tests;

use tests::*;

fn test_result<T>(result: Result<(), T>) -> &'static str {
	match result {
		Ok(_) => "ok",
		Err(_) => "failed!",
	}
}

fn main() {
	println!(
		"Test {} ... {}",
		stringify!(pi_sequential),
		test_result(pi_sequential(5000000))
	);
}
