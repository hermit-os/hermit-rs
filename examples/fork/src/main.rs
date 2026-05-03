use std::process::exit;
use std::time::{Duration, Instant};

#[cfg(target_os = "hermit")]
use hermit as _;
use hermit_abi::{exec, fork, getpid, waitpid, Pid};

fn fork_bench(count: usize) -> Duration {
	let mut pids = vec![Pid::default(); count]; // Pre-allocate with zeros

	let start = Instant::now();
	for pid in pids.iter_mut().take(count) {
		*pid = unsafe { fork() };

		if *pid == 0 {
			exit(0);
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
	println!("Try to fork a process...");

	let pid = unsafe { fork() };
	if pid == 0 {
		println!("Hello from child process with id {}!", unsafe { getpid() });

		let app = c"/bin/hello_world";
		unsafe {
			let _ = exec(app.as_ptr());
		}

		println!("ERROR: Exec failed!!!");
	} else if pid > 0 {
		println!(
			"Hello from parent process with id {}! Child has the id {}!",
			unsafe { getpid() },
			pid
		);

		unsafe {
			waitpid(pid);
		}

		println!("Measure overhead of the system call fork.");

		// warmup cache
		let _ = fork_bench(2);

		for i in 0..14 {
			let count = 1 << i;
			let result = fork_bench(count);
			println!("Time to fork/join {count} processes: {result:?}");
		}
	} else {
		println!("Unable to fork a process!");
	}

	println!("Leave program");
}
