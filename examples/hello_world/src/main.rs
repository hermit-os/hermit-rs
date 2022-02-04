#[cfg(target_os = "hermit")]
use hermit_sys as _;

fn main() {
	println!("Hello World!");
}
