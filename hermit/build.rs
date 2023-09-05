use std::env;
use std::ffi::OsStr;
use std::path::PathBuf;
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
	let is_docs_rs = env::var_os("DOCS_RS").is_some();
	if !targets_hermit || runs_clippy || is_docs_rs {
		return;
	}

	let kernel_src = KernelSrc::local().unwrap_or_else(KernelSrc::download);

	kernel_src.build();
}

struct KernelSrc {
	src_dir: PathBuf,
}

impl KernelSrc {
	fn local() -> Option<Self> {
		let mut src_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
		src_dir.set_file_name("kernel");
		src_dir.exists().then_some(Self { src_dir })
	}

	fn download() -> Self {
		let version = "0.6.4";
		let out_dir = out_dir();
		let src_dir = out_dir.join(format!("kernel-{version}"));

		if !src_dir.exists() {
			let url =
				format!("https://github.com/hermitcore/kernel/archive/refs/tags/v{version}.tar.gz");
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
		let arch = env::var_os("CARGO_CFG_TARGET_ARCH").unwrap();
		let profile = env::var("PROFILE").expect("PROFILE was not set");

		let cargo = {
			// On windows, the userspace toolchain ends up in front of the rustup proxy in $PATH.
			// To reach the rustup proxy nonetheless, we explicitly query $CARGO_HOME.
			let mut cargo_home = PathBuf::from(env::var_os("CARGO_HOME").unwrap());
			cargo_home.push("bin/cargo");
			cargo_home
		};

		let mut cmd = Command::new(cargo);

		// Remove rust-toolchain-specific environment variables from kernel cargo
		cmd.env_remove("LD_LIBRARY_PATH");
		env::vars()
			.filter(|(key, _value)| key.starts_with("CARGO") || key.starts_with("RUST"))
			.for_each(|(key, _value)| {
				cmd.env_remove(&key);
			});

		cmd.current_dir(&self.src_dir)
			.arg("run")
			.arg("--package=xtask")
			.arg("--target-dir")
			.arg(&target_dir)
			.arg("--")
			.arg("build")
			.arg("--arch")
			.arg(&arch)
			.args([
				"--profile",
				match profile.as_str() {
					"debug" => "dev",
					profile => profile,
				},
			])
			.arg("--target-dir")
			.arg(&target_dir);

		if has_feature("instrument") {
			cmd.arg("--instrument-mcount");
		}

		if has_feature("randomize-layout") {
			cmd.arg("--randomize-layout");
		}

		// Control enabled features via this crate's features
		cmd.arg("--no-default-features");
		forward_features(
			&mut cmd,
			[
				"acpi", "dhcpv4", "fsgsbase", "pci", "pci-ids", "smp", "tcp", "udp", "trace",
				"vga", "rtl8139", "fs",
			]
			.into_iter(),
		);

		let status = cmd.status().expect("failed to start kernel build");
		assert!(status.success());

		let lib_location = target_dir
			.join(&arch)
			.join(&profile)
			.canonicalize()
			.unwrap();

		println!("cargo:rustc-link-search=native={}", lib_location.display());
		println!("cargo:rustc-link-lib=static=hermit");

		let rerun_if_changed = |path| {
			println!(
				"cargo:rerun-if-changed={}",
				self.src_dir.join(path).display()
			);
		};

		rerun_if_changed(".cargo");
		rerun_if_changed("hermit-builtins/src");
		rerun_if_changed("hermit-builtins/Cargo.lock");
		rerun_if_changed("hermit-builtins/Cargo.toml");
		rerun_if_changed("src");
		rerun_if_changed("xtask");
		rerun_if_changed("Cargo.lock");
		rerun_if_changed("Cargo.toml");
		rerun_if_changed("rust-toolchain.toml");
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
		cmd.args(["--features", &features.join(" ")]);
	}
}
