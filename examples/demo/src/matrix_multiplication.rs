// Copyright (c) 2019 Stefan Lankes, RWTH Aachen University
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

#![allow(dead_code)]
#![allow(unused_imports)]
#![feature(thread_id_value)]

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
