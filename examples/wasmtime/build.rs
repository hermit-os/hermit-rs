use std::process::Command;
use std::{env, io};

fn main() -> io::Result<()> {
	let cargo = env::var_os("CARGO").unwrap();
	let out_dir = env::var_os("OUT_DIR").unwrap();

	let status = Command::new(cargo)
		.arg("build")
		.arg("-Zunstable-options")
		.arg("-Zbuild-std=std,panic_abort")
		.arg("--target=wasm32-unknown-unknown")
		.arg("--package=fibonacci")
		.arg("--release")
		.arg("--out-dir")
		.arg(&out_dir)
		.status()?;
	assert!(status.success());

	Ok(())
}
