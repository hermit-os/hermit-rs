use core::arch::x86_64 as arch;
use std::env;
use std::f64::consts::{E, PI};
use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::str;
use std::thread;
use std::time::Instant;
use std::vec;
#[cfg(target_os = "linux")]
use syscalls::SYS_getpid;

#[inline]
fn get_timestamp_rdtscp() -> u64 {
	unsafe {
		let mut _aux = 0;
		let value = arch::__rdtscp(&mut _aux);
		arch::_mm_lfence();
		value
	}
}

extern "C" {
	#[cfg(target_os = "hermit")]
	fn sys_getpid() -> u32;
}

pub fn bench_syscall() -> Result<(), ()> {
	let n = 1000000;

	let ticks = unsafe {
		// cache warmup
		#[cfg(target_os = "hermit")]
		let _ = sys_getpid();
		#[cfg(target_os = "linux")]
		let _ = syscall!(SYS_getpid);
		let _ = get_timestamp_rdtscp();

		let start = get_timestamp_rdtscp();
		for _ in 0..n {
			#[cfg(target_os = "hermit")]
			let _ = sys_getpid();
			#[cfg(target_os = "linux")]
			let _ = syscall!(SYS_getpid);
		}
		get_timestamp_rdtscp() - start
	};

	println!("Time {} for a system call (in ticks)", ticks / n);

	Ok(())
}

pub fn bench_sched_one_thread() -> Result<(), ()> {
	let n = 1000000;

	// cache warmup
	thread::yield_now();
	thread::yield_now();
	let _ = get_timestamp_rdtscp();

	let start = get_timestamp_rdtscp();
	for _ in 0..n {
		thread::yield_now();
	}
	let ticks = get_timestamp_rdtscp() - start;

	println!("Scheduling time {} ticks (1 thread)", ticks / n);

	Ok(())
}

pub fn bench_sched_two_threads() -> Result<(), ()> {
	let n = 1000000;
	let nthreads = 2;

	// cache warmup
	thread::yield_now();
	thread::yield_now();
	let _ = get_timestamp_rdtscp();

	let start = get_timestamp_rdtscp();
	let threads: Vec<_> = (0..nthreads - 1)
		.map(|_| {
			thread::spawn(move || {
				for _ in 0..n {
					thread::yield_now();
				}
			})
		})
		.collect();

	for _ in 0..n {
		thread::yield_now();
	}

	let ticks = get_timestamp_rdtscp() - start;

	for t in threads {
		t.join().unwrap();
	}

	println!(
		"Scheduling time {} ticks (2 threads)",
		ticks / (nthreads * n)
	);

	Ok(())
}
