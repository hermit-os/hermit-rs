use std::borrow::Cow;
use std::env;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::Command;

use flate2::read::GzDecoder;
use tar::Archive;

fn main() {
	// TODO: Replace with is_some_with once stabilized
	// https://github.com/rust-lang/rust/issues/93050
	let targets_hermit =
		matches!(env::var_os("CARGO_CFG_TARGET_OS"), Some(os) if os == OsStr::new("hermit"));
	let runs_clippy =
		matches!(env::var_os("CARGO_CFG_FEATURE"), Some(os) if os == OsStr::new("cargo-clippy"));
	if !targets_hermit || runs_clippy {
		return;
	}

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
		let version = "0.3.54";
		let out_dir = out_dir();
		let src_dir = out_dir.join(format!("libhermit-rs-{version}"));

		if !src_dir.exists() {
			let url = format!(
				"https://github.com/hermitcore/libhermit-rs/archive/refs/tags/v{version}.tar.gz"
			);
			let response = ureq::get(url.as_str()).call().unwrap().into_reader();
			let tar = GzDecoder::new(response);
			let mut archive = Archive::new(tar);
			archive.unpack(src_dir.parent().unwrap()).unwrap();
		}

		Self { src_dir }
	}

	fn build(self) {
		let target_dir = target_dir();
		let manifest_path = self.src_dir.join("Cargo.toml");
		assert!(
			manifest_path.exists(),
			"kernel manifest path `{}` does not exist",
			manifest_path.display()
		);
		let user_target = env::var("TARGET").unwrap();
		let profile = env::var("PROFILE").expect("PROFILE was not set");

		let kernel_target = match user_target.as_str() {
			"x86_64-unknown-hermit" => "x86_64-unknown-none-hermitkernel",
			"aarch64-unknown-hermit" => "aarch64-unknown-none-hermitkernel",
			_ => panic!("Unsupported target: {}", user_target),
		};

		let mut cmd = Command::new("cargo");
		cmd.current_dir(&self.src_dir)
			.arg("build")
			.arg("-Z")
			.arg("build-std=core,alloc")
			.args(&["--target", kernel_target])
			.arg("--manifest-path")
			.arg("Cargo.toml")
			.arg("--target-dir")
			.arg(&target_dir);

		cmd.env_remove("RUSTUP_TOOLCHAIN");

		cmd.env("CARGO_TERM_COLOR", "always");

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
			.join(kernel_target)
			.join(&profile)
			.canonicalize()
			.unwrap();

		println!("Lib location: {}", lib_location.display());

		let lib = lib_location.join("libhermit.a");

		let mut symbols = vec!["rust_begin_unwind", "rust_oom"];

		match kernel_target {
			"x86_64-unknown-none-hermitkernel" => {
				symbols.extend(include_str!("x86_64-duplicate-symbols").lines())
			}
			"aarch64-unknown-none-hermitkernel" => {
				symbols.extend(include_str!("aarch64-duplicate-symbols").lines())
			}
			_ => (),
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

fn out_dir() -> PathBuf {
	env::var_os("OUT_DIR").unwrap().into()
}

fn target_dir() -> PathBuf {
	let mut target_dir = out_dir();
	target_dir.push("target");
	target_dir
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
		cmd.args(&["--features", &features.join(" ")]);
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
