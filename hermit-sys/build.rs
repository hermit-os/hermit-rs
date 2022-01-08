extern crate walkdir;

use std::borrow::Cow;
use std::env;
use std::ffi::OsStr;
use std::ffi::OsString;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use walkdir::{DirEntry, WalkDir};

macro_rules! forward_features {
	($cmd:expr, $($feature:literal,)+ ) => {
		let mut features = vec![];

		$(
			if cfg!(feature = $feature) {
				features.push($feature);
			}
		)+

		if !features.is_empty() {
			$cmd.arg("--features");
			$cmd.arg(features.join(" "));
		}
	};
}

fn build_hermit(src_dir: &Path, target_dir_opt: Option<&Path>) {
	let manifest_path = src_dir.join("Cargo.toml");
	assert!(
		manifest_path.exists(),
		"kernel manifest path `{}` does not exist",
		manifest_path.display()
	);
	let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();
	let profile = env::var("PROFILE").expect("PROFILE was not set");
	let mut cmd = Command::new("cargo");

	let kernel_triple = match target_arch.as_str() {
		"x86_64" => "x86_64-unknown-none",
		"aarch64" => "aarch64-unknown-none-softfloat",
		_ => panic!("Unsupported target arch: {}", target_arch),
	};

	cmd.arg("build")
		.arg("-Z")
		.arg("build-std=core,alloc")
		.arg("--target")
		.arg(kernel_triple)
		.arg("--manifest-path")
		.arg("Cargo.toml");

	cmd.current_dir(src_dir);

	cmd.env_remove("RUSTUP_TOOLCHAIN");
	if option_env!("RUSTC_WORKSPACE_WRAPPER")
		.unwrap_or_default()
		.ends_with("clippy-driver")
	{
		cmd.env("RUSTC_WORKSPACE_WRAPPER", "clippy-driver");
	}

	cmd.env("CARGO_TERM_COLOR", "always");

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

	// Control enabled features via this crate's features
	cmd.arg("--no-default-features");
	forward_features!(
		cmd,
		"aarch64-qemu-stdout",
		"acpi",
		"fsgsbase",
		"pci",
		"smp",
		"vga",
	);

	let mut rustflags = vec!["-Zmutable-noalias=no".to_string()];
	let outer_rustflags = env::var("CARGO_ENCODED_RUSTFLAGS").unwrap();

	#[cfg(feature = "instrument")]
	{
		rustflags.push("-Zinstrument-mcount".to_string());
		// Add outer rustflags to command
		rustflags.push(outer_rustflags);
	}

	#[cfg(not(feature = "instrument"))]
	{
		// If the `instrument` feature feature is not enabled,
		// filter it from outer rustflags before adding them to the command.
		if !outer_rustflags.is_empty() {
			let flags = outer_rustflags
				.split('\x1f')
				.filter(|&flag| !flag.contains("instrument-mcount"))
				.map(String::from);
			rustflags.extend(flags);
		}
	}

	cmd.env("CARGO_ENCODED_RUSTFLAGS", rustflags.join("\x1f"));

	let status = cmd.status().expect("failed to start kernel build");
	assert!(status.success());

	let lib_location = target_dir
		.join(kernel_triple)
		.join(&profile)
		.canonicalize()
		.unwrap();

	println!("Lib location: {}", lib_location.display());

	let lib = lib_location.join("libhermit.a");

	let mut symbols = vec!["rust_begin_unwind", "rust_oom"];

	if target_arch == "aarch64" {
		symbols.extend(include_str!("aarch64-duplicate-symbols").lines());
	}

	rename_symbols(symbols, &lib);

	println!("cargo:rustc-link-search=native={}", lib_location.display());
	println!("cargo:rustc-link-lib=static=hermit");

	// HERMIT_LOG_LEVEL_FILTER sets the log level filter at compile time
	println!("cargo:rerun-if-env-changed=HERMIT_LOG_LEVEL_FILTER");
}

/// Kernel and user space has its own versions of panic handler, oom handler, memcpy, memset, etc,
/// Consequently, we rename the functions in the libos to avoid collisions.
fn rename_symbols(symbols: impl IntoIterator<Item = impl AsRef<OsStr>>, lib: impl AsRef<Path>) {
	let args = symbols.into_iter().flat_map(|symbol| {
		let option = OsStr::new("--redefine-sym");
		let arg = [symbol.as_ref(), "=kernel-".as_ref(), symbol.as_ref()]
			.into_iter()
			.collect::<OsString>();
		[Cow::Borrowed(option), Cow::Owned(arg)]
	});

	let status = Command::new("rust-objcopy")
		.args(args)
		.arg(lib.as_ref())
		.status()
		.expect("Failed to execute rust-objcopy. Is cargo-binutils installed?");
	assert!(status.success(), "rust-objcopy was not successful");
}

#[cfg(all(not(feature = "rustc-dep-of-std"), not(feature = "with_submodule")))]
fn build() {
	#[cfg(windows)]
	let out_dir = env::temp_dir();
	#[cfg(not(windows))]
	let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
	let src_dir = out_dir.join("rusty-hermit");

	if !src_dir.as_path().exists() {
		let status = Command::new(env!("CARGO"))
			.current_dir(out_dir)
			.arg("download")
			.arg("--output")
			.arg(src_dir.clone().into_os_string())
			.arg("--extract")
			.arg("rusty-hermit")
			.status()
			.expect("failed to start kernel download");
		assert!(
			status.success(),
			"Unable to download rusty-hermit. Is cargo-download installed?"
		);
	}

	build_hermit(src_dir.as_ref(), None);
}

#[cfg(all(not(feature = "rustc-dep-of-std"), feature = "with_submodule"))]
fn build() {
	let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
	let target_dir = out_dir.join("target");
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
		.filter_entry(is_not_ignored)
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
