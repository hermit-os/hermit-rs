use std::path::Path;
use std::{env, fs, io};

#[cfg(target_os = "hermit")]
use hermit as _;

fn main() -> io::Result<()> {
	read("/proc/version")?;
	read_dir("/proc")?;

	test_dir("/tmp")?;


	
	if cfg!(target_os = "hermit") {
		test_dir("/root")?;
	}

	// assert_chdir(".", "/")?;
	// assert_chdir("/", "/")?;
	// assert_chdir("/tmp", "/tmp")?;

	Ok(())
}

fn assert_chdir<P: AsRef<Path>, Q: AsRef<Path>>(path: P, expected: Q) -> io::Result<()> {
	let path = path.as_ref();
	let expected = expected.as_ref();

	eprintln!("chdir({})", path.display());
	env::set_current_dir(path)?;

	eprint!("cwd = ");
	let cwd = env::current_dir()?;
	eprintln!("{}", cwd.display());
	assert_eq!(cwd, expected);

	Ok(())
}

fn test_dir<P: AsRef<Path>>(path: P) -> io::Result<()> {
	let path = path.as_ref();

	if let Err(err) = remove_files(path) {
		eprintln!("err = {err}");
	}

	create_files(path)?;
	read_dir(path)?;
	remove_files(path)?;

	Ok(())
}

fn read<P: AsRef<Path>>(path: P) -> io::Result<()> {
	let path = path.as_ref();

	eprint!("{} contains", path.display());
	let content = fs::read_to_string(path).unwrap();
	eprintln!(" {content:?}");

	eprintln!();
	Ok(())
}

fn create_files<P: AsRef<Path>>(dir: P) -> io::Result<()> {
	let dir = dir.as_ref();
	let hello_path = dir.join("hello.txt");
	let hello_content = "Hello, world!";

	if hello_path.exists() {
		return Err(io::Error::from(io::ErrorKind::AlreadyExists));
	}

	eprint!("{:15} : writing", hello_path.display());
	fs::write(&hello_path, hello_content)?;

	eprint!(", writing");
	fs::write(&hello_path, hello_content)?;

	eprintln!(", reading");
	let read = fs::read_to_string(&hello_path)?;
	assert_eq!(hello_content, read);

	eprintln!();
	Ok(())
}

fn remove_files<P: AsRef<Path>>(dir: P) -> io::Result<()> {
	let dir = dir.as_ref();
	let hello_path = dir.join("hello.txt");

	if !hello_path.exists() {
		return Err(io::Error::from(io::ErrorKind::NotFound));
	}

	eprintln!("{:15} : removing", hello_path.display());
	fs::remove_file(hello_path)?;

	eprintln!();
	Ok(())
}

fn read_dir<P: AsRef<Path>>(path: P) -> io::Result<()> {
	let path = path.as_ref();
	eprintln!();

	assert!(path.is_dir());
	eprintln!("Reading {path:?} directory entries");
	let entries = fs::read_dir(path)?.collect::<Result<Vec<_>, _>>()?;

	assert!(!entries.is_empty());
	for entry in entries {
		let path = entry.path();
		eprintln!("Found {path:?}");

		let lstat = entry.metadata()?;
		eprintln!("lstat = {lstat:?}");

		// FIXME:
		// assert_eq!(entry.file_type()?, lstat.file_type());

		// FIXME:
		// let path_link = path.read_link()?;
		// eprintln!("path_link = {}", path_link.display());

		let stat = path.metadata()?;
		eprintln!("stat = {stat:?}");

		eprintln!();
	}

	eprintln!();
	Ok(())
}

mod fuse_test {
	use std::fs;
	use std::path::Path;

	#[cfg(target_os = "hermit")]
	const TEST_DIR: &str = "/root";
	#[cfg(not(target_os = "hermit"))]
	const TEST_DIR: &str = "/tmp/data";

	pub fn main() {
		let test_path = Path::new(TEST_DIR);
		assert!(test_path.is_dir());
		let paths = fs::read_dir(test_path).unwrap();

		let path_to_new_dir = test_path.join("new_dir");

		assert!(!path_to_new_dir.exists());
		fs::create_dir(&path_to_new_dir).unwrap();
		assert!(path_to_new_dir.exists());

		assert!(path_to_new_dir.exists());
		fs::remove_dir(&path_to_new_dir).unwrap();
		assert!(!path_to_new_dir.exists());
	}
}
