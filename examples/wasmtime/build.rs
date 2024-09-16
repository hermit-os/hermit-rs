use std::path::PathBuf;
use std::process::Command;
use std::{env, io};

fn main() -> io::Result<()> {
	let mut cargo = cargo();
	let out_dir = env::var_os("OUT_DIR").unwrap();

	let package = if cfg!(feature = "ci") {
		"hello_world"
	} else {
		"wasm-test"
	};

	cargo
		.arg("+nightly-2024-07-31")
		.arg("build")
		.arg("-Zunstable-options")
		.arg("-Zbuild-std=std,panic_abort")
		.arg("--target=wasm32-wasip1")
		.args(["--package", package])
		.arg("--release")
		.arg("--target-dir=target")
		.arg("--artifact-dir")
		.arg(&out_dir);
	let status = cargo.status()?;
	assert!(status.success());

	Ok(())
}

pub fn cargo() -> Command {
	sanitize("cargo")
}

fn sanitize(cmd: &str) -> Command {
	let cmd = {
		let exe = format!("{cmd}{}", env::consts::EXE_SUFFIX);
		// On windows, the userspace toolchain ends up in front of the rustup proxy in $PATH.
		// To reach the rustup proxy nonetheless, we explicitly query $CARGO_HOME.
		let mut cargo_home = PathBuf::from(env::var_os("CARGO_HOME").unwrap());
		cargo_home.push("bin");
		cargo_home.push(&exe);
		if cargo_home.exists() {
			cargo_home
		} else {
			PathBuf::from(exe)
		}
	};

	let mut cmd = Command::new(cmd);

	// Remove rust-toolchain-specific environment variables from kernel cargo
	cmd.env_remove("LD_LIBRARY_PATH");
	env::vars()
		.filter(|(key, _value)| {
			key.starts_with("CARGO") && !key.starts_with("CARGO_HOME")
				|| key.starts_with("RUST") && !key.starts_with("RUSTUP_HOME")
		})
		.for_each(|(key, _value)| {
			cmd.env_remove(&key);
		});

	cmd
}
