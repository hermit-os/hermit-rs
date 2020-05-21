extern crate walkdir;

use std::env;
use std::process::Command;
use walkdir::{DirEntry, WalkDir};

#[cfg(all(not(feature = "rustc-dep-of-std"), not(feature = "with_submodule")))]
fn build() {
	#[cfg(windows)]
	let out_dir = env::temp_dir().to_str().unwrap().to_owned();
	#[cfg(not(windows))]
	let out_dir = env::var("OUT_DIR").unwrap();
	let profile = env::var("PROFILE").expect("PROFILE was not set");

	let _output = Command::new("cargo")
		.current_dir(out_dir.clone())
		.arg("download")
		.arg("--output")
		.arg(out_dir.clone() + "/rusty-hermit")
		.arg("--extract")
		.arg("rusty-hermit")
		.output()
		.expect("Unable to download rusty-hermit. Please install `cargo-download`.");

	let mut cmd = Command::new("cargo");
	cmd.current_dir(out_dir.clone() + "/rusty-hermit")
		.arg("build")
		.arg("-Z")
		.arg("build-std=core,alloc")
		.arg("--target")
		.arg("x86_64-unknown-hermit-kernel");

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

	let output = cmd.output().expect("Unable to build kernel");
	let stdout = std::string::String::from_utf8(output.stdout);
	let stderr = std::string::String::from_utf8(output.stderr);
	println!("Build libhermit-rs output-status: {}", output.status);
	println!("Build libhermit-rs output-stdout: {}", stdout.unwrap());
	println!("Build libhermit-rs output-stderr: {}", stderr.unwrap());
	assert!(output.status.success());

	println!(
		"cargo:rustc-link-search=native={}/rusty-hermit/target/x86_64-unknown-hermit-kernel/{}",
		out_dir.clone(),
		profile
	);
	println!("cargo:rustc-link-lib=static=hermit");
}

#[cfg(all(not(feature = "rustc-dep-of-std"), feature = "with_submodule"))]
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

	let output = cmd.output().expect("Unable to build kernel");
	let stdout = std::string::String::from_utf8(output.stdout);
	let stderr = std::string::String::from_utf8(output.stderr);
	println!("Build libhermit-rs output-status: {}", output.status);
	println!("Build libhermit-rs output-stdout: {}", stdout.unwrap());
	println!("Build libhermit-rs output-stderr: {}", stderr.unwrap());
	assert!(output.status.success());

	println!(
		"cargo:rustc-link-search=native={}/target/x86_64-unknown-hermit-kernel/{}",
		out_dir.clone(),
		profile
	);
	println!("cargo:rustc-link-lib=static=hermit");
}

#[cfg(all(not(feature = "rustc-dep-of-std"), feature = "with_submodule"))]
fn configure_cargo_rerun_if_changed() {
	fn is_not_ignored(entry: &DirEntry) -> bool {
		// Ignore .git .vscode and target directories, but not .cargo or .github
		if entry.depth() == 1 && entry.path().is_dir() {
			if entry.path().ends_with("target")
				|| entry.path().ends_with(".git")
				|| entry.path().ends_with(".vscode")
			{
				return false;
			}
		}
		true
	}

	WalkDir::new("../libhermit-rs")
		.into_iter()
		.filter_entry(|e| is_not_ignored(e))
		.filter_map(|v| v.ok())
		.filter_map(|v| v.path().canonicalize().ok())
		.for_each(|x| println!("cargo:rerun-if-changed={}", x.display()));
}

fn main() {
	#[cfg(all(not(feature = "rustc-dep-of-std"), feature = "with_submodule"))]
	configure_cargo_rerun_if_changed();
	#[cfg(not(feature = "rustc-dep-of-std"))]
	build();
}
