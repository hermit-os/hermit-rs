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
extern crate rayon;
#[cfg(target_os = "linux")]
#[macro_use]
extern crate syscalls;

mod benches;

use benches::*;

fn main() {
	bench_sched_one_thread().unwrap();
	bench_sched_two_threads().unwrap();
	bench_syscall().unwrap();
}
