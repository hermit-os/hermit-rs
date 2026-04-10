#[cfg(target_os = "hermit")]
use hermit as _;
use hermit_abi::{spawn_process, waitpid};

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
}
