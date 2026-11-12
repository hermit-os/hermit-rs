use std::fs;
use std::io::{ErrorKind, Write};
use std::path::Path;

#[cfg(target_os = "hermit")]
use hermit as _;

#[cfg(target_os = "hermit")]
const TEST_DIR: &str = "/root";
#[cfg(not(target_os = "hermit"))]
const TEST_DIR: &str = "/tmp/data";

fn main() {
	let test_path = Path::new(TEST_DIR);
	assert!(test_path.is_dir());
	let paths = fs::read_dir(test_path).unwrap();

	let path_to_new_dir = test_path.join("new_dir");

	assert!(!path_to_new_dir.exists());
	fs::create_dir(&path_to_new_dir).unwrap();
	assert!(path_to_new_dir.exists());

	for direntry in paths {
		let direntry = direntry.unwrap();
		println!("\nPath: {}", direntry.path().display());
		println!("Name: {}", direntry.file_name().into_string().unwrap());
		let file_type = direntry.file_type().unwrap();
		if file_type.is_dir() {
			println!("Is dir!");
		} else if file_type.is_file() {
			println!("Is file!");
			println!("Content: {}", fs::read_to_string(direntry.path()).unwrap());
			let file = fs::File::open(direntry.path()).unwrap();
			assert!(file.metadata().unwrap().is_file());
		} else if file_type.is_symlink() {
			println!("Is symlink!");
			println!(
				"Points to file: {:?}",
				fs::metadata(direntry.path()).unwrap().file_type().is_file()
			);
		} else {
			println!("Unknown type!");
		}

		let meta_data = direntry.metadata().unwrap();
		assert!(meta_data.file_type() == file_type);

		println!("Size: {} bytes", meta_data.len());
		println!("Accessed: {:?}", meta_data.accessed().unwrap());
		println!("Created: {:?}", meta_data.created().unwrap());
		println!("Modified: {:?}", meta_data.modified().unwrap());
		println!("Read only: {:?}", meta_data.permissions().readonly());
	}

	assert!(path_to_new_dir.exists());
	test_subdir(&path_to_new_dir);
	fs::remove_dir(&path_to_new_dir).unwrap();
	assert!(!path_to_new_dir.exists());

	println!("Done.");
}

/// Creates and removes entries below a subdirectory, because the names
/// are resolved relative to the parent directory.
fn test_subdir(dir: &Path) {
	let sub_dir = dir.join("sub_dir");
	let file = sub_dir.join("file.txt");

	fs::create_dir(&sub_dir).unwrap();
	assert!(sub_dir.is_dir());

	let mut f = fs::File::create(&file).unwrap();
	f.write_all(b"Hello from a nested file").unwrap();
	drop(f);
	assert_eq!(
		fs::read_to_string(&file).unwrap(),
		"Hello from a nested file"
	);

	let names: Vec<_> = fs::read_dir(&sub_dir)
		.unwrap()
		.map(|entry| entry.unwrap().file_name())
		.collect();
	assert_eq!(names, ["file.txt"]);

	assert_eq!(
		fs::remove_dir(&sub_dir).unwrap_err().kind(),
		ErrorKind::DirectoryNotEmpty
	);
	assert!(file.is_file());

	fs::remove_file(&file).unwrap();
	assert_eq!(fs::metadata(&file).unwrap_err().kind(), ErrorKind::NotFound);
	assert_eq!(
		fs::remove_file(&file).unwrap_err().kind(),
		ErrorKind::NotFound
	);

	fs::remove_dir(&sub_dir).unwrap();
	assert!(!sub_dir.exists());
	assert_eq!(
		fs::remove_dir(&sub_dir).unwrap_err().kind(),
		ErrorKind::NotFound
	);
}
