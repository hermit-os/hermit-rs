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

mod laplace;
mod matmul;

pub use matmul::test_matmul_strassen;

#[inline]
fn get_timestamp_rdtscp() -> u64 {
	unsafe {
		let mut _aux = 0;
		let value = arch::__rdtscp(&mut _aux);
		arch::_mm_lfence();
		value
	}
}

pub fn thread_creation() -> Result<(), ()> {
	const N: usize = 10;

	// cache warmup
	let _ = get_timestamp_rdtscp();
	{
		let builder = thread::Builder::new();
		let child = builder.spawn(|| {}).unwrap();
		let _ = child.join();
	}

	let start = get_timestamp_rdtscp();
	for _ in 0..N {
		let builder = thread::Builder::new();
		let child = builder.spawn(|| {}).unwrap();
		let _ = child.join();
	}
	let end = get_timestamp_rdtscp();

	println!(
		"Time to create and to join a thread: {} ticks",
		(end - start) / N as u64
	);

	Ok(())
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

pub fn pi_sequential(num_steps: u64) -> Result<(), ()> {
	let step = 1.0 / num_steps as f64;
	let mut sum = 0 as f64;

	for i in 0..num_steps {
		let x = (i as f64 + 0.5) * step;
		sum += 4.0 / (1.0 + x * x);
	}

	let mypi = sum * (1.0 / num_steps as f64);
	println!("Pi: {} (sequential)", mypi);

	if (mypi - PI).abs() < 0.00001 {
		Ok(())
	} else {
		Err(())
	}
}

pub fn pi_parallel(nthreads: u64, num_steps: u64) -> Result<(), ()> {
	let step = 1.0 / num_steps as f64;
	let mut sum = 0.0 as f64;

	let threads: Vec<_> = (0..nthreads)
		.map(|tid| {
			thread::spawn(move || {
				let mut partial_sum = 0 as f64;
				let start = (num_steps / nthreads) * tid;
				let end = (num_steps / nthreads) * (tid + 1);

				for i in start..end {
					let x = (i as f64 + 0.5) * step;
					partial_sum += 4.0 / (1.0 + x * x);
				}

				partial_sum
			})
		})
		.collect();

	for t in threads {
		sum += t.join().unwrap();
	}

	let mypi = sum * (1.0 / num_steps as f64);
	println!("Pi: {} (with {} threads)", mypi, nthreads);

	if (mypi - PI).abs() < 0.00001 {
		Ok(())
	} else {
		Err(())
	}
}

pub fn read_file() -> Result<(), std::io::Error> {
	let mut file = File::open("/etc/hostname")?;
	let mut contents = String::new();
	file.read_to_string(&mut contents)?;

	println!("Hostname: {}", contents);

	Ok(())
}

pub fn create_file() -> Result<(), std::io::Error> {
	{
		let mut file = File::create("/tmp/foo.txt")?;
		file.write_all(b"Hello, world!")?;
	}

	let contents = {
		let mut file = File::open("/tmp/foo.txt")?;
		let mut contents = String::new();
		file.read_to_string(&mut contents)?;
		contents
	};

	// delete temporary file
	std::fs::remove_file("/tmp/foo.txt")?;

	if contents == "Hello, world!" {
		Ok(())
	} else {
		let kind = std::io::ErrorKind::Other;
		Err(std::io::Error::from(kind))
	}
}

pub fn print_argv() -> Result<(), ()> {
	let args = env::args();

	// Prints each argument on a separate line
	for (i, argument) in args.enumerate() {
		println!("argument[{}] = {}", i, argument);
	}

	Ok(())
}

pub fn print_env() -> Result<(), ()> {
	let envs = env::vars();

	// We will iterate through the references to the element returned by
	// env::vars();
	for (key, value) in envs {
		println!("{}: {}", key, value);
	}

	Ok(())
}

pub fn hello() -> Result<(), ()> {
	println!("Hello, world!");
	println!("Привет мир!");
	println!("こんにちは世界");
	println!("你好，世界");
	println!("สวัสดีชาวโลก");
	println!("Chào thế giới");
	let crab = vec![0xF0 as u8, 0x9F as u8, 0xA6 as u8, 0x80 as u8];
	println!(
		"Crab emoji: {}",
		String::from_utf8(crab).unwrap_or_default()
	);

	Ok(())
}

pub fn arithmetic() -> Result<(), ()> {
	let x = (get_timestamp_rdtscp() % 10) as f64 * 3.41f64;
	let y: f64 = x.exp();
	let z: f64 = y.log(E);

	println!("x = {}, e^x = {}, ln(e^x) = {}", x, y, z);

	Ok(())
}

pub fn threading() -> Result<(), ()> {
	// Make a vector to hold the children which are spawned.
	let mut children = vec![];

	for i in 0..2 {
		// Spin up another thread
		children.push(thread::spawn(move || {
			println!("this is thread number {}", i);
		}));
	}

	for child in children {
		// Wait for the thread to finish. Returns a result.
		let _ = child.join();
	}

	Ok(())
}

pub fn laplace(size_x: usize, size_y: usize) -> Result<(), ()> {
	let matrix = matrix_setup(size_x, size_y);

	let now = Instant::now();
	let (iterations, res) = laplace::compute(matrix, size_x, size_y);
	println!(
		"Time to solve {} s, iterations {}, residuum {}",
		now.elapsed().as_secs_f64(),
		iterations,
		res
	);

	if res < 0.01 {
		Ok(())
	} else {
		Err(())
	}
}

pub fn matrix_setup(size_x: usize, size_y: usize) -> vec::Vec<vec::Vec<f64>> {
	let mut matrix = vec![vec![0.0; size_x * size_y]; 2];

	// top row
	for x in 0..size_x {
		matrix[0][x] = 1.0;
		matrix[1][x] = 1.0;
	}

	// bottom row
	for x in 0..size_x {
		matrix[0][(size_y - 1) * size_x + x] = 1.0;
		matrix[1][(size_y - 1) * size_x + x] = 1.0;
	}

	// left row
	for y in 0..size_y {
		matrix[0][y * size_x] = 1.0;
		matrix[1][y * size_x] = 1.0;
	}

	// right row
	for y in 0..size_y {
		matrix[0][y * size_x + size_x - 1] = 1.0;
		matrix[1][y * size_x + size_x - 1] = 1.0;
	}

	matrix
}

fn send_http_rquest() -> Result<Vec<u8>, std::io::Error> {
	let mut stream = TcpStream::connect("10.0.5.1:7878")?;
	let buf =
		b"GET / HTTP/1.1\r\nHost: 10.0.5.1\r\nUser-Agent: hermit/0.3.16\r\nAccept: */*\r\n\r\n";
	stream.write_all(buf)?;

	let mut buf = Vec::new();
	stream.read_to_end(&mut buf)?;

	Ok(buf)
}

pub fn test_http_request() -> Result<(), std::io::Error> {
	let buf = send_http_rquest()?;
	let data = str::from_utf8(&buf).unwrap().to_owned();

	//println!("Receive {}", data);

	match data.starts_with("HTTP/1.1 200 OK") {
		true => println!("Check http response... ok"),
		_ => println!("Check http response... failed!"),
	};

	Ok(())
}
