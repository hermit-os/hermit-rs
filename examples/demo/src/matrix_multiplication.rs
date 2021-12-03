#![allow(dead_code)]
#![allow(unused_imports)]
#![feature(thread_id_value)]
#![feature(thread_local_const_init)]

#[cfg(target_os = "hermit")]
extern crate hermit_sys;
extern crate num_cpus;
extern crate rayon;
#[cfg(target_os = "linux")]
#[macro_use]
extern crate syscalls;

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
		stringify!(test_matmul_strassen),
		test_result(test_matmul_strassen())
	);
}
