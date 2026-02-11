use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, str};

use flate2::read::GzDecoder;
use tar::Archive;

fn main() {
	let targets_hermit =
		env::var_os("CARGO_CFG_TARGET_OS").is_some_and(|target_os| target_os == "hermit");
	let runs_clippy =
		env::var_os("CARGO_CFG_FEATURE").is_some_and(|feature| feature == "cargo-clippy");
	let is_docs_rs = env::var_os("DOCS_RS").is_some();
	let is_common_os = has_feature("common-os");
	if !targets_hermit || runs_clippy || is_docs_rs || is_common_os {
		return;
	}

	let kernel_src = KernelSrc::local().unwrap_or_else(KernelSrc::download);

	kernel_src.build();

	if has_feature("libc") {
		let libc = cc::Build::new()
			.get_compiler()
			.to_command()
			.arg("-print-file-name=libc.a")
			.output()
			.unwrap()
			.stdout;
		let libc = str::from_utf8(&libc).unwrap().trim_ascii_end();
		let libc_dir = Path::new(libc).parent().unwrap();
		if libc_dir.is_dir() {
			println!("cargo:rustc-link-search={}", libc_dir.display());
		}

		println!("cargo:rustc-link-lib=static=c");
	}
}

struct KernelSrc {
	src_dir: PathBuf,
}

impl KernelSrc {
	fn local() -> Option<Self> {
		if let Some(src_dir) = env::var_os("HERMIT_MANIFEST_DIR") {
			assert!(
				!src_dir.is_empty(),
				"HERMIT_MANIFEST_DIR is set to the empty string"
			);
			let src_dir = PathBuf::from(src_dir);
			return Some(Self { src_dir });
		}

		let mut src_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
		src_dir.set_file_name("kernel");
		src_dir.exists().then_some(Self { src_dir })
	}

	fn download() -> Self {
		let version = "0.12.0";
		let out_dir = out_dir();
		let src_dir = out_dir.join(format!("kernel-{version}"));

		if !src_dir.exists() {
			let url =
				format!("https://github.com/hermit-os/kernel/archive/refs/tags/v{version}.tar.gz");
			let response = ureq::get(url.as_str())
				.call()
				.unwrap()
				.into_body()
				.into_reader();
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
		let mut arch = env::var_os("CARGO_CFG_TARGET_ARCH").unwrap();
		let endian = env::var_os("CARGO_CFG_TARGET_ENDIAN").unwrap();
		let profile = env::var("PROFILE").expect("PROFILE was not set");

		if arch == "aarch64" && endian == "big" {
			arch = "aarch64_be".into();
		}

		let mut cargo = cargo();

		cargo
			.current_dir(&self.src_dir)
			.arg("run")
			.arg("--package=xtask")
			.arg("--no-default-features")
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

		if has_feature("randomize-layout") {
			cargo.arg("--randomize-layout");
		}

		// Control enabled features via this crate's features
		cargo.arg("--no-default-features");
		forward_features(
			&mut cargo,
			[
				"acpi",
				"dhcpv4",
				"dns",
				"fs",
				"fsgsbase",
				"gem-net",
				"idle-poll",
				"instrument-mcount",
				"kernel-stack",
				"log-target",
				"mman",
				"mmap",
				"net",
				"pci",
				"pci-ids",
				"rtl8139",
				"semihosting",
				"shell",
				"smp",
				"strace",
				"tcp",
				"trace",
				"udp",
				"vga",
				"virtio-console",
				"virtio-fs",
				"virtio-net",
				"virtio-vsock",
				// Deprecated
				"console",
				"vsock",
			]
			.into_iter(),
		);

		println!("cargo:warning=$ {cargo:?}");
		let status = cargo.status().expect("failed to start kernel build");
		assert!(status.success());

		let lib_location = target_dir
			.join(&arch)
			.join(&profile)
			.canonicalize()
			.unwrap();

		println!("cargo:rustc-link-search=native={}", lib_location.display());
		println!("cargo:rustc-link-lib=static=hermit");

		self.rerun_if_changed_cargo(&self.src_dir.join("Cargo.toml"));
		self.rerun_if_changed_cargo(&self.src_dir.join("hermit-builtins/Cargo.toml"));
		self.rerun_if_changed_cargo(&self.src_dir.join("hermit-macro/Cargo.toml"));

		println!(
			"cargo:rerun-if-changed={}",
			self.src_dir.join("rust-toolchain.toml").display()
		);

		println!("cargo:rerun-if-env-changed=HERMIT_CAREFUL");
		println!("cargo:rerun-if-env-changed=HERMIT_DNS1");
		println!("cargo:rerun-if-env-changed=HERMIT_DNS2");
		println!("cargo:rerun-if-env-changed=HERMIT_GATEWAY");
		println!("cargo:rerun-if-env-changed=HERMIT_IP");
		println!("cargo:rerun-if-env-changed=HERMIT_LOG_LEVEL_FILTER");
		println!("cargo:rerun-if-env-changed=HERMIT_MANIFEST_DIR");
		println!("cargo:rerun-if-env-changed=HERMIT_MASK");
		println!("cargo:rerun-if-env-changed=HERMIT_MRG_RXBUF_SIZE");
		println!("cargo:rerun-if-env-changed=HERMIT_MTU");
		println!("cargo:rerun-if-env-changed=NO_COLOR");
		println!("cargo:rerun-if-env-changed=UHYVE_MOUNT");
	}

	fn rerun_if_changed_cargo(&self, cargo_toml: &Path) {
		let mut cargo = cargo();

		let output = cargo
			.arg("tree")
			.arg(format!("--manifest-path={}", cargo_toml.display()))
			.arg("--prefix=none")
			.arg("--workspace")
			.output()
			.unwrap();

		let output = str::from_utf8(&output.stdout).unwrap();

		let path_deps = output.lines().filter_map(|dep| {
			let mut split = dep.split(&['(', ')']);
			split.next();
			let path = split.next()?;
			path.starts_with('/').then_some(path)
		});

		for path_dep in path_deps {
			println!("cargo:rerun-if-changed={path_dep}/src");
			println!("cargo:rerun-if-changed={path_dep}/Cargo.toml");
			if Path::new(path_dep).join("Cargo.lock").exists() {
				println!("cargo:rerun-if-changed={path_dep}/Cargo.lock");
			}
			if Path::new(path_dep).join("build.rs").exists() {
				println!("cargo:rerun-if-changed={path_dep}/build.rs");
			}
		}
	}
}

fn cargo() -> Command {
	let cargo = {
		let exe = format!("cargo{}", env::consts::EXE_SUFFIX);
		// On windows, the userspace toolchain ends up in front of the rustup proxy in $PATH.
		// To reach the rustup proxy nonetheless, we explicitly query $CARGO_HOME.
		let mut cargo_home = home::cargo_home().unwrap();
		cargo_home.push("bin");
		cargo_home.push(&exe);
		if cargo_home.exists() {
			cargo_home
		} else {
			PathBuf::from(exe)
		}
	};

	let mut cargo = Command::new(cargo);

	// Remove rust-toolchain-specific environment variables from kernel cargo
	cargo.env_remove("LD_LIBRARY_PATH");
	env::vars()
		.filter(|(key, _value)| key.starts_with("CARGO") || key.starts_with("RUST"))
		.for_each(|(key, _value)| {
			cargo.env_remove(&key);
		});

	cargo
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
