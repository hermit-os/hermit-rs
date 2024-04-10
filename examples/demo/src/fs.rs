use std::path::Path;
use std::{fs, io};

pub fn read_dir(path: &str) -> io::Result<()> {
	eprintln!();

	assert!(Path::new(path).is_dir());
	eprintln!("Reading {path:?} directory entries");
	let entries = fs::read_dir(path)?.collect::<Result<Vec<_>, _>>()?;

	assert!(!entries.is_empty());
	for entry in entries {
		let path = entry.path();
		eprintln!("Found {path:?}");
	}

	Ok(())
}

pub fn read_version() -> io::Result<()> {
	eprintln!();
	let path = "/proc/version";

	eprint!("{path} contains");
	let version = fs::read_to_string(path).unwrap();
	eprintln!(" {version:?}");

	Ok(())
}

fn test_file(path: &str) -> io::Result<()> {
	let contents = "Hello, world!";

	eprint!("{path:15} : writing");
	fs::write(path, contents)?;

	eprint!(", reading");
	let read = fs::read_to_string(path)?;
	assert_eq!(contents, read);

	// FIXME: this fails on Uhyve
	// eprintln!(", deleting");
	// fs::remove_file(path)?;

	Ok(())
}

pub fn file() -> io::Result<()> {
	eprintln!();
	test_file("/tmp/hello.txt")?;
	if cfg!(target_os = "hermit") && cfg!(feature = "fs") {
		test_file("/root/hello.txt")?;
	}
	Ok(())
}

pub fn dir() -> io::Result<()> {
	read_dir("/proc")?;
	// FIXME: this fails on both QEMU and Uhyve
	// read_dir("/root")?;
	Ok(())
}

pub fn fs() -> io::Result<()> {
	read_version()?;
	file()?;
	dir()?;
	Ok(())
}
