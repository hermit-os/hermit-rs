#[cfg(target_os = "hermit")]
use hermit as _;

fn main() {
	println!("Hello, world!");

	println!("Arguments:");
	for argument in std::env::args() {
    	println!("{argument}");
	}

	println!("Environment variables:");
	for (key, value) in std::env::vars() {
    	println!("{key}: {value}");
	}
}
