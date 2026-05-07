#[cfg(target_os = "hermit")]
use hermit as _;

fn main() {
	std::process::exit(0);
}
