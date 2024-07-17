use std::process::Command;
use std::{env, io};

fn main() -> io::Result<()> {
	let cargo = env::var_os("CARGO").unwrap();
	let out_dir = env::var_os("OUT_DIR").unwrap();

	#[cfg(not(feature = "ci"))]
	let status = Command::new(cargo)
		.arg("build")
		.arg("-Zunstable-options")
		.arg("-Zbuild-std=std,panic_abort")
		.arg("--target=wasm32-wasi")
		.arg("--package=wasm-test")
		.arg("--release")
		.arg("--target-dir=target")
		.arg("--artifact-dir")
		.arg(&out_dir)
		.status()?;
	#[cfg(feature = "ci")]
	let status = Command::new(cargo)
		.arg("build")
		.arg("-Zunstable-options")
		.arg("-Zbuild-std=std,panic_abort")
		.arg("--target=wasm32-wasi")
		.arg("--package=hello_world")
		.arg("--release")
		.arg("--target-dir=target")
		.arg("--artifact-dir")
		.arg(&out_dir)
		.status()?;
	assert!(status.success());

	Ok(())
}
