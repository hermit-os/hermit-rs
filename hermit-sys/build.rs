use std::borrow::Cow;
use std::env;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
	let kernel_src = if has_feature("with_submodule") {
		KernelSrc::from_submodule()
	} else {
		KernelSrc::download()
	};

	kernel_src.build();
}

struct KernelSrc {
	src_dir: PathBuf,
}

impl KernelSrc {
	fn from_submodule() -> Self {
		let mut src_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
		src_dir.set_file_name("libhermit-rs");
		Self { src_dir }
	}

	fn download() -> Self {
		let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
		let src_dir = out_dir.join("rusty-hermit");

		if !src_dir.exists() {
			let status = Command::new("cargo")
				.current_dir(out_dir)
				.arg("download")
				.arg("--output")
				.arg(&src_dir)
				.arg("--extract")
				.arg("rusty-hermit")
				.status()
				.expect("failed to start kernel download");
			assert!(
				status.success(),
				"Unable to download rusty-hermit. Is cargo-download installed?"
			);
		}

		Self { src_dir }
	}

	fn build(self) {
		let target_dir = {
			let mut target_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
			target_dir.push("target");
			target_dir
		};
		let manifest_path = self.src_dir.join("Cargo.toml");
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
			"aarch64" => "aarch64-unknown-none-hermitkernel",
			_ => panic!("Unsupported target arch: {}", target_arch),
		};

		cmd.arg("build")
			.arg("-Z")
			.arg("build-std=core,alloc")
			.arg("--target")
			.arg(kernel_triple)
			.arg("--manifest-path")
			.arg("Cargo.toml");

		cmd.current_dir(&self.src_dir);

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
		forward_features(
			&mut cmd,
			["acpi", "fsgsbase", "pci", "smp", "vga"].into_iter(),
		);

		let mut rustflags = vec!["-Zmutable-noalias=no".to_string()];
		let outer_rustflags = env::var("CARGO_ENCODED_RUSTFLAGS").unwrap();

		if has_feature("instrument") {
			rustflags.push("-Zinstrument-mcount".to_string());
			// Add outer rustflags to command
			rustflags.push(outer_rustflags);
		} else {
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

		// Kernel and user space has its own versions of panic handler, oom handler, memcpy, memset, etc,
		// Consequently, we rename the functions in the libos to avoid collisions.
		rename_symbols(symbols.iter(), &lib);

		println!("cargo:rustc-link-search=native={}", lib_location.display());
		println!("cargo:rustc-link-lib=static=hermit");

		println!("cargo:rerun-if-changed={}", self.src_dir.display());
		// HERMIT_LOG_LEVEL_FILTER sets the log level filter at compile time
		println!("cargo:rerun-if-env-changed=HERMIT_LOG_LEVEL_FILTER");
	}
}

fn has_feature(feature: &str) -> bool {
	let mut var = "CARGO_FEATURE_".to_string();

	var.extend(feature.chars().map(|c| match c {
		'-' => '_',
		c => c.to_ascii_uppercase(),
	}));

	env::var_os(&var).is_some()
}

fn forward_features<'a>(cmd: &mut Command, features: impl Iterator<Item = &'a str>) {
	let features = features.filter(|f| has_feature(f)).collect::<Vec<_>>();
	if !features.is_empty() {
		cmd.arg("--features");
		cmd.arg(features.join(" "));
	}
}

fn rename_symbols(symbols: impl Iterator<Item = impl AsRef<OsStr>>, lib: &Path) {
	let args = symbols.into_iter().flat_map(|symbol| {
		let option = OsStr::new("--redefine-sym");
		let arg = [symbol.as_ref(), "=kernel-".as_ref(), symbol.as_ref()]
			.into_iter()
			.collect::<OsString>();
		[Cow::Borrowed(option), Cow::Owned(arg)]
	});

	let status = Command::new("rust-objcopy")
		.args(args)
		.arg(lib)
		.status()
		.expect("Failed to execute rust-objcopy. Is cargo-binutils installed?");
	assert!(status.success(), "rust-objcopy was not successful");
}
