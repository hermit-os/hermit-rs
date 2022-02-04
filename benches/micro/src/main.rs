#![allow(dead_code)]
#![allow(unused_imports)]
#![feature(test)]
#![feature(bench_black_box)]

#[cfg(target_os = "linux")]
#[macro_use]
extern crate syscalls;

#[cfg(target_os = "hermit")]
use hermit_sys as _;

mod benches;

use benches::*;

fn main() {
	bench_sched_one_thread().unwrap();
	bench_sched_two_threads().unwrap();
	bench_syscall().unwrap();
	bench_mem().unwrap();
}
