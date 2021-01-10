extern crate llvm_tools;
extern crate target_build_utils;
extern crate walkdir;

use std::env;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{self, Command};

use target_build_utils::TargetInfo;
use walkdir::{DirEntry, WalkDir};

fn build_hermit(src_dir: &Path, target_dir_opt: Option<&Path>) {
	let target = TargetInfo::new().expect("Could not get target info");
	let profile = env::var("PROFILE").expect("PROFILE was not set");
	let mut cmd = Command::new("cargo");

	if target.target_arch() == "x86_64" {
		cmd.current_dir(src_dir)
			.arg("build")
			.arg("-Z")
			.arg("build-std=core,alloc")
			.arg("--target")
			.arg("x86_64-unknown-hermit-kernel");
	} else if target.target_arch() == "aarch64" {
		cmd.current_dir(src_dir)
			.arg("build")
			.arg("-Z")
			.arg("build-std=core,alloc")
			.arg("--target")
			.arg("aarch64-unknown-hermit");
	} else {
		panic!("Try to build for an unsupported platform");
	}

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

	// disable all default features
	cmd.arg("--no-default-features");

	// do we have to enable PCI support?
	#[cfg(feature = "pci")]
	{
		cmd.arg("--features");
		cmd.arg("pci");
	}

	// do we have to enable acpi support?
	#[cfg(feature = "acpi")]
	{
		cmd.arg("--features");
		cmd.arg("acpi");
	}

	// do we have to enable FSGSBASE support?
	#[cfg(feature = "fsgs_base")]
	{
		cmd.arg("--features");
		cmd.arg("fsgs_base");
	}
  
  // do we support multi-processor systems?
  #[cfg(feature = "smp")]
  {
    cmd.arg("--features");
		cmd.arg("smp");
  }

	#[cfg(feature = "instrument")]
	{
		cmd.env("RUSTFLAGS", "-Z instrument-mcount");
		// if instrument is not set, ensure that instrument is not in environment variables!
		cmd.env(
			"RUSTFLAGS",
			env::var("RUSTFLAGS")
				.unwrap_or_else(|_| "".into())
				.replace("-Z instrument-mcount", ""),
		);
	}

	let output = cmd.output().expect("Unable to build kernel");
	let stdout = std::string::String::from_utf8(output.stdout);
	let stderr = std::string::String::from_utf8(output.stderr);

	println!("Build libhermit-rs output-status: {}", output.status);
	println!("Build libhermit-rs output-stdout: {}", stdout.unwrap());
	println!("Build libhermit-rs output-stderr: {}", stderr.unwrap());
	assert!(output.status.success());

	let lib_location = if target.target_arch() == "x86_64" {
		target_dir
			.join("x86_64-unknown-hermit-kernel")
			.join(&profile)
			.canonicalize()
			.unwrap() // Must exist after building
	} else if target.target_arch() == "aarch64" {
		target_dir
			.join("aarch64-unknown-hermit")
			.join(&profile)
			.canonicalize()
			.unwrap() // Must exist after building
	} else {
		panic!("Try to build for an unsupported platform");
	};
	println!("Lib location: {}", lib_location.display());

	// Kernel and user space has its own versions of memcpy, memset, etc,
	// Consequently, we rename the functions in the libos to avoid collisions.
	// In addition, it provides us the offer to create a optimized version of memcpy
	// in user space.

	// get access to llvm tools shipped in the llvm-tools-preview rustup component
	let llvm_tools = match llvm_tools::LlvmTools::new() {
		Ok(tools) => tools,
		Err(llvm_tools::Error::NotFound) => {
			eprintln!("Error: llvm-tools not found");
			eprintln!("Maybe the rustup component `llvm-tools-preview` is missing?");
			eprintln!("  Install it through: `rustup component add llvm-tools-preview`");
			process::exit(1);
		}
		Err(err) => {
			eprintln!("Failed to retrieve llvm-tools component: {:?}", err);
			process::exit(1);
		}
	};

	let lib = lib_location.join("libhermit.a");

	let symbols: [&str; 5] = ["memcpy", "memmove", "memset", "memcmp", "bcmp"];
	for symbol in symbols.iter() {
		// determine llvm_objcopy
		let llvm_objcopy = llvm_tools
			.tool(&llvm_tools::exe("llvm-objcopy"))
			.expect("llvm_objcopy not found in llvm-tools");

		// rename symbols
		let mut cmd = Command::new(llvm_objcopy);
		cmd.arg("--redefine-sym")
			.arg(String::from(*symbol) + &String::from("=kernel") + &String::from(*symbol))
			.arg(lib.display().to_string());

		println!("cmd {:?}", cmd);
		let output = cmd.output().expect("Unable to rename symbols");
		let stdout = std::string::String::from_utf8(output.stdout);
		let stderr = std::string::String::from_utf8(output.stderr);
		println!("Rename symbols output-status: {}", output.status);
		println!("Rename symbols output-stdout: {}", stdout.unwrap());
		println!("Rename symbols output-stderr: {}", stderr.unwrap());
	}

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
