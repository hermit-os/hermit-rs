use std::borrow::Cow;
use std::env;
use std::ffi::OsStr;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

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

fn build_hermit(src_dir: &Path) {
	let target_dir = {
		let mut target_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
		target_dir.push("target");
		target_dir
	};
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
		"x86_64" => "x86_64-unknown-none-hermitkernel",
		"aarch64" => "aarch64-unknown-hermit",
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

	cmd.arg("--target-dir").arg(&target_dir);

	if profile == "release" {
		cmd.arg("--release");
	}

	// Control enabled features via this crate's features
	cmd.arg("--no-default-features");
	forward_features!(cmd, "acpi", "fsgsbase", "pci", "smp", "vga",);

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

	println!("cargo:rerun-if-changed={}", src_dir.display());
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

#[cfg(not(feature = "with_submodule"))]
fn build() {
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

	build_hermit(src_dir.as_ref());
}

#[cfg(feature = "with_submodule")]
fn build() {
	let src_dir = env::current_dir()
		.unwrap()
		.parent()
		.unwrap()
		.join("libhermit-rs");

	build_hermit(src_dir.as_ref());
}

fn main() {
	build();
}
