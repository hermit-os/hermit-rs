use std::process::Command;
use std::{env, io};

fn main() -> io::Result<()> {
	let cargo = env::var_os("CARGO").unwrap();
	let out_dir = env::var_os("OUT_DIR").unwrap();

	let package = if cfg!(feature = "ci") {
		"hello_world"
	} else {
		"wasm-test"
	};

	let status = Command::new(cargo)
		.arg("build")
		.arg("-Zunstable-options")
		.arg("--target=wasm32-wasip1")
		.args(["--package", package])
		.arg("--release")
		.arg("--target-dir=target")
		.arg("--artifact-dir")
		.arg(&out_dir)
		.status()?;
	assert!(status.success());

	Ok(())
}
