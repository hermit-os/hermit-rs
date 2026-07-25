#[cfg(target_os = "hermit")]
use hermit as _;

fn main() {
	println!("Hello, world!");

	for (i, arg) in std::env::args().enumerate() {
		println!("arg[{i}]: {arg}");
	}
	for (key, value) in std::env::vars() {
		println!("env: {key}={value}");
	}
}
