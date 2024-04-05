use std::fs::File;
use std::io::{Read, Write};
use std::time::{self, Instant};
use std::{env, f64, hint, thread, vec};

use rayon::prelude::*;

pub fn test_sleep() -> Result<(), ()> {
	let one_sec = time::Duration::from_millis(1000);
	let now = time::Instant::now();

	thread::sleep(one_sec);

	let elapsed = now.elapsed().as_millis();
	println!("Measured time for 1 second sleep: {} ms", elapsed);

	if !(985..=1015).contains(&elapsed) {
		Err(())
	} else {
		Ok(())
	}
}

pub fn thread_creation() -> Result<(), ()> {
	const N: usize = 10;

	// cache warmup
	{
		let builder = thread::Builder::new();
		let child = builder.spawn(|| {}).unwrap();
		let _ = child.join();
	}

	let now = Instant::now();
	for _ in 0..N {
		let builder = thread::Builder::new();
		let child = builder.spawn(|| {}).unwrap();
		let _ = child.join();
	}

	println!(
		"Time to create and to join a thread: {} ms",
		now.elapsed().as_secs_f64() * 1000.0f64 / f64::from(N as i32)
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
	println!("Pi: {mypi} (sequential)");

	if (mypi - f64::consts::PI).abs() < 0.00001 {
		Ok(())
	} else {
		Err(())
	}
}

pub fn pi_parallel(num_steps: u64) -> Result<(), ()> {
	let ncpus = thread::available_parallelism().unwrap().get();
	let pool = rayon::ThreadPoolBuilder::new()
		.num_threads(ncpus)
		.build()
		.unwrap();
	let step = 1.0 / num_steps as f64;

	let sum: f64 = pool.install(|| {
		(0..num_steps)
			.into_par_iter()
			.map(|i| {
				let x = (i as f64 + 0.5) * step;
				4.0 / (1.0 + x * x)
			})
			.sum()
	});

	let mypi = sum * (1.0 / num_steps as f64);
	println!("Pi: {mypi} (with {ncpus} threads)");

	if (mypi - f64::consts::PI).abs() < 0.00001 {
		Ok(())
	} else {
		Err(())
	}
}

pub fn read_file() -> Result<(), std::io::Error> {
	let mut file = File::open("/proc/version")?;
	let mut contents = String::new();
	file.read_to_string(&mut contents)?;

	println!("Version: {contents}");

	Ok(())
}

pub fn read_dir() -> Result<(), std::io::Error> {
	for entry in std::fs::read_dir("/proc")? {
		let entry = entry?;
		println!("Found {:?} in /proc", entry.file_name());
	}

	Ok(())
}

pub fn create_file() -> Result<(), std::io::Error> {
	{
		let mut file = File::create("/tmp/foo.txt")?;
		file.write_all(b"Hello, world!")?;
	}

	let content = {
		let mut file = File::open("/tmp/foo.txt")?;
		let mut content = String::new();
		file.read_to_string(&mut content)?;
		content
	};

	// delete temporary file
	std::fs::remove_file("/tmp/foo.txt")?;

	if content == "Hello, world!" {
		Ok(())
	} else {
		println!("Read invalid content: {} (len {})", content, content.len());
		let kind = std::io::ErrorKind::Other;
		Err(std::io::Error::from(kind))
	}
}

pub fn print_argv() -> Result<(), ()> {
	let args = env::args();

	// Prints each argument on a separate line
	for (i, argument) in args.enumerate() {
		println!("argument[{i}] = {argument}");
	}

	Ok(())
}

pub fn print_env() -> Result<(), ()> {
	let envs = env::vars();

	// We will iterate through the references to the element returned by
	// env::vars();
	for (key, value) in envs {
		println!("{key}: {value}");
	}

	Ok(())
}

pub fn hello() -> Result<(), ()> {
	println!("Hello, world!");
	println!("Привет, мир!");
	println!("こんにちは世界！");
	println!("你好世界！");
	println!("สวัสดีชาวโลก!");
	println!("Chào thế giới!");
	let crab = vec![0xF0_u8, 0x9F_u8, 0xA6_u8, 0x80_u8];
	println!(
		"Crab emoji: {}",
		String::from_utf8(crab).unwrap_or_default()
	);

	Ok(())
}

pub fn arithmetic() -> Result<(), ()> {
	let x = hint::black_box(f64::consts::PI) * 2.0;
	let y: f64 = x.exp();
	let z: f64 = y.ln();

	println!("x = {x}, e^x = {y}, ln(e^x) = {z}");

	Ok(())
}

pub fn threading() -> Result<(), ()> {
	// Make a vector to hold the children which are spawned.
	let mut children = vec![];

	for i in 0..2 {
		// Spin up another thread
		children.push(thread::spawn(move || {
			println!("this is thread number {i}");
		}));
	}

	for child in children {
		// Wait for the thread to finish. Returns a result.
		let _ = child.join();
	}

	Ok(())
}
