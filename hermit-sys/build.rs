extern crate walkdir;

use std::env;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use walkdir::{DirEntry, WalkDir};

fn build_hermit(src_dir: &Path, target_dir_opt: Option<&Path>) {
	let profile = env::var("PROFILE").expect("PROFILE was not set");
	let mut cmd = Command::new("cargo");
	cmd.current_dir(src_dir)
		.arg("build")
		.arg("-Z")
		.arg("build-std=core,alloc")
		.arg("--target")
		.arg("x86_64-unknown-hermit-kernel");

	if let Some(target_dir) = target_dir_opt {
		cmd.arg("--target-dir").arg(target_dir);
	}
	let target_dir = match target_dir_opt {
		Some(target_dir) => target_dir.to_path_buf(),
		None => src_dir.join("target"),
	};

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
			.unwrap_or_else(|_| "".into())
			.replace("-Z instrument-mcount", ""),
	);

	let output = cmd.output().expect("Unable to build kernel");
	let stdout = std::string::String::from_utf8(output.stdout);
	let stderr = std::string::String::from_utf8(output.stderr);

	println!("Build libhermit-rs output-status: {}", output.status);
	println!("Build libhermit-rs output-stdout: {}", stdout.unwrap());
	println!("Build libhermit-rs output-stderr: {}", stderr.unwrap());
	assert!(output.status.success());

	let lib_location = target_dir
		.join("x86_64-unknown-hermit-kernel")
		.join(&profile)
		.canonicalize()
		.unwrap(); // Must exist after building
	println!("cargo:rustc-link-search=native={}", lib_location.display());
	println!("cargo:rustc-link-lib=static=hermit");

	//HERMIT_LOG_LEVEL_FILTER sets the log level filter at compile time
	// Doesn't actually rebuild atm - see: https://github.com/rust-lang/cargo/issues/8306
	println!("cargo:rerun-if-env-changed=HERMIT_LOG_LEVEL_FILTER");
}

#[cfg(all(not(feature = "rustc-dep-of-std"), not(feature = "with_submodule")))]
fn build() {
	#[cfg(windows)]
	let out_dir = env::temp_dir();
	#[cfg(not(windows))]
	let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
	let src_dir = out_dir.join("rusty-hermit");

	let _output = Command::new("cargo")
		.current_dir(out_dir)
		.arg("download")
		.arg("--output")
		.arg(src_dir.clone().into_os_string())
		.arg("--extract")
		.arg("rusty-hermit")
		.output()
		.expect("Unable to download rusty-hermit. Please install `cargo-download`.");

	build_hermit(src_dir.as_ref(), None);
}

#[cfg(all(not(feature = "rustc-dep-of-std"), feature = "with_submodule"))]
fn build() {
	let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
	let target_dir = out_dir.clone().join("target");
	let src_dir = env::current_dir()
		.unwrap()
		.parent()
		.unwrap()
		.join("libhermit-rs");

	build_hermit(src_dir.as_ref(), Some(target_dir.as_ref()));
	configure_cargo_rerun_if_changed(src_dir.as_ref());
}

#[cfg(all(not(feature = "rustc-dep-of-std"), feature = "with_submodule"))]
fn configure_cargo_rerun_if_changed(src_dir: &Path) {
	fn is_not_ignored(entry: &DirEntry) -> bool {
		// Ignore .git .vscode and target directories, but not .cargo or .github
		if entry.depth() == 1
			&& entry.path().is_dir()
			&& (entry.path().ends_with("target")
				|| entry.path().ends_with(".git")
				|| entry.path().ends_with(".vscode"))
		{
			return false;
		}
		true
	}

	WalkDir::new(src_dir)
		.into_iter()
		.filter_entry(|e| is_not_ignored(e))
		.filter_map(|v| v.ok())
		.filter_map(|v| v.path().canonicalize().ok())
		.for_each(|x| println!("cargo:rerun-if-changed={}", x.display()));
}

#[cfg(not(feature = "rustc-dep-of-std"))]
fn create_constants() {
	let out_dir = env::var("OUT_DIR").expect("No out dir");
	let dest_path = Path::new(&out_dir).join("constants.rs");
	let mut f = File::create(&dest_path).expect("Could not create file");

	let ip = option_env!("HERMIT_IP");
	let ip = ip.map_or("10.0.5.3", |v| v);

	let gateway = option_env!("HERMIT_GATEWAY");
	let gateway = gateway.map_or("10.0.5.1", |v| v);

	let mask = option_env!("HERMIT_MASK");
	let mask = mask.map_or("255.255.255.0", |v| v);

	writeln!(&mut f, "const HERMIT_IP: &str = \"{}\";", ip).expect("Could not write file");
	println!("cargo:rerun-if-env-changed=HERMIT_IP");

	writeln!(&mut f, "const HERMIT_GATEWAY: &str = \"{}\";", gateway)
		.expect("Could not write file");
	println!("cargo:rerun-if-env-changed=HERMIT_GATEWAY");

	writeln!(&mut f, "const HERMIT_MASK: &str = \"{}\";", mask).expect("Could not write file");
	println!("cargo:rerun-if-env-changed=HERMIT_MASK");
}

fn main() {
	#[cfg(not(feature = "rustc-dep-of-std"))]
	create_constants();
	#[cfg(not(feature = "rustc-dep-of-std"))]
	build();
}
