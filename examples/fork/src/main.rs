#[cfg(target_os = "hermit")]
use hermit as _;
use hermit_abi::{fork, getpid, waitpid};

fn main() {
	println!("Try to fork a process...");

	let pid = unsafe { fork() };
	if pid == 0 {
		println!("Hello from child process with id {}!", unsafe { getpid() });
	} else if pid > 0 {
		println!(
			"Hello from parent process with id {}! Child has the id {}!",
			unsafe { getpid() },
			pid
		);

		unsafe {
			waitpid(pid);
		}
	} else {
		println!("Unable to fork a process!");
	}
}
