use std::env;
use std::process::Command;

#[cfg(not(feature = "rustc-dep-of-std"))]
fn build() {
	let out_dir = env::var("OUT_DIR").unwrap();
	let target_dir = out_dir.clone() + "/target";
	let profile = env::var("PROFILE").expect("PROFILE was not set");

	if profile == "release" {
		let _output = Command::new("cargo")
			.current_dir("libhermit-rs")
			.arg("build")
			.arg("-Z")
			.arg("build-std=core,alloc")
			.arg("--target")
			.arg("x86_64-unknown-hermit-kernel")
			.arg("--target-dir")
			.arg(target_dir)
			.arg("--release")
			.output()
			.expect("Unable to build kernel");
	} else {
		let _output = Command::new("cargo")
			.current_dir("libhermit-rs")
			.arg("build")
			.arg("-Z")
			.arg("build-std=core,alloc")
			.arg("--target")
			.arg("x86_64-unknown-hermit-kernel")
			.arg("--target-dir")
			.arg(target_dir)
			.output()
			.expect("Unable to build kernel");
	}

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
