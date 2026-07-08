use std::fs::File;
use std::io::Read as _;
use std::path::Path;

#[cfg(target_os = "hermit")]
use hermit as _;

const TEST_DIR: &str = if cfg!(target_os = "hermit") {
	"/root"
} else {
	"./initramfs/root"
};

fn main() {
	let test_path = Path::new(TEST_DIR);

	let mut file = File::open(test_path.join("hello_world.txt")).unwrap();
	let mut buf = Vec::new();

	file.read_to_end(&mut buf).unwrap();
	assert_eq!(buf, b"Hello, world!\n");
}
