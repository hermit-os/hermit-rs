#[cfg(target_os = "hermit")]
use hermit as _;

use std::{fs, path::Path};

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
	fs::remove_dir(&path_to_new_dir).unwrap();
	assert!(!path_to_new_dir.exists());

	println!("Done.");
}
