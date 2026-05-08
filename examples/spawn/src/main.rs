use std::time::{Duration, Instant};

#[cfg(target_os = "hermit")]
use hermit as _;
use hermit_abi::{spawn_process, waitpid, Pid};

fn spawn_bench(count: usize) -> Duration {
	let mut pids = vec![Pid::default(); count]; // Pre-allocate with zeros
	let app = c"/bin/true";

	let start = Instant::now();
	for pid in pids.iter_mut().take(count) {
		*pid = unsafe { spawn_process(app.as_ptr()) };

		if *pid <= 0 {
			println!("Unable to spawn a process!");
		}
	}

	for pid in &pids {
		unsafe {
			waitpid(*pid);
		}
	}

	start.elapsed()
}

fn main() {
	println!("Try to spawn a process...");

	let app = c"/bin/hello_world";
	let pid = unsafe { spawn_process(app.as_ptr()) };
	if pid > 0 {
		println!("Spawn process {app:?} with id {pid}!");

		unsafe {
			waitpid(pid);
		}
	} else {
		println!("Unable to spawn a process!");
	}

	// warmup cache
	let _ = spawn_bench(2);

	for i in 0..14 {
		let count = 1 << i;
		let result = spawn_bench(count);
		println!("Time to spawn/join {count} processes: {result:?}");
	}
}
