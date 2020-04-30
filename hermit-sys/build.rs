use std::env;
use std::process::Command;

#[cfg(not(feature = "rustc-dep-of-std"))]
fn build() {
	let out_dir = env::var("OUT_DIR").unwrap();
	let target_dir = out_dir.clone() + "/target";
	let profile = env::var("PROFILE").expect("PROFILE was not set");

	let mut cmd = Command::new("cargo");
	cmd.current_dir("../libhermit-rs")
		.arg("build")
		.arg("-Z")
		.arg("build-std=core,alloc")
		.arg("--target")
		.arg("x86_64-unknown-hermit-kernel")
		.arg("--target-dir")
		.arg(target_dir);

	if profile == "release" {
		cmd.arg("--release");
	}

	#[cfg(feature = "instrument")]
	cmd.env("RUSTFLAGS", "-Z instrument-mcount");
	// if instrument is not set, ensure that instrument is not in environment variables!
	#[cfg(not(feature = "instrument"))]
	cmd.env(
		"RUSTFLAGS",
		env::var("RUSTFLAGS")
			.unwrap_or("".into())
			.replace("-Z instrument-mcount", ""),
	);

	cmd.output().expect("Unable to build kernel");

	println!(
		"cargo:rustc-link-search=native={}/target/x86_64-unknown-hermit-kernel/{}",
		out_dir.clone(),
		profile
	);
	println!("cargo:rustc-link-lib=static=hermit");
}

fn main() {
	#[cfg(not(feature = "rustc-dep-of-std"))]
	build();
}
