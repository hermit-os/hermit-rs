//! Minimal demonstration of `pipe` + `fork`.
//!
//! The parent creates a pipe before forking. Because both pipe
//! descriptors are inherited by the child, the two processes end up
//! sharing the same in-kernel buffer: the child writes a message into the
//! write end, the parent reads it back from the read end.
//!
//! Each process closes the end it does not use. Closing the child's write
//! end is what lets the parent's read loop observe end-of-file (`read`
//! returning 0) once the message has been consumed.

#[cfg(target_os = "hermit")]
use hermit as _;
use hermit_abi::{close, exit, fork, getpid, pipe, read, waitpid, write};

const MESSAGE: &[u8] = b"Hello from the child!";

fn main() {
	// `pipefd[0]` becomes the read end, `pipefd[1]` the write end.
	let mut pipefd = [0i32; 2];
	if unsafe { pipe(pipefd.as_mut_ptr()) } < 0 {
		println!("Unable to create a pipe!");
		return;
	}
	let [read_fd, write_fd] = pipefd;

	let pid = unsafe { fork() };

	if pid == 0 {
		// --- child ---
		// The child only writes, so it closes the read end first.
		unsafe { close(read_fd) };

		let mut written = 0;
		while written < MESSAGE.len() {
			let n = unsafe {
				write(
					write_fd,
					MESSAGE[written..].as_ptr(),
					MESSAGE.len() - written,
				)
			};
			if n <= 0 {
				break;
			}
			written += n as usize;
		}

		// Dropping the write end signals end-of-file to the reader.
		unsafe { close(write_fd) };

		// Terminate the child directly via the raw exit syscall, avoiding
		// a std runtime tear-down in the forked address space.
		unsafe { exit(0) };
	} else if pid > 0 {
		// --- parent ---
		// The parent only reads, so it closes the write end. This is
		// essential: as long as *any* write end is open the reader would
		// never see end-of-file.
		unsafe { close(write_fd) };

		let mut buffer = [0u8; 128];
		let mut received = Vec::new();
		loop {
			let n = unsafe { read(read_fd, buffer.as_mut_ptr(), buffer.len()) };
			if n <= 0 {
				// 0 == end-of-file (child closed the write end), < 0 == error.
				break;
			}
			received.extend_from_slice(&buffer[..n as usize]);
		}

		unsafe { close(read_fd) };
		unsafe { waitpid(pid) };

		println!(
			"Parent {} received from child {}: {:?}",
			unsafe { getpid() },
			pid,
			String::from_utf8_lossy(&received),
		);
	} else {
		println!("Unable to fork a process!");
	}
}
